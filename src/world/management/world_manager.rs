//! Unified world manager that works with GPU and CPU backends

use crate::world::{
    compute::{ComputeError, UnifiedCompute, UnifiedComputeConfig},
    core::{BlockId, ChunkPos, VoxelPos},
    generation::{
        GeneratorConfig as UnifiedGeneratorConfig, GeneratorError, UnifiedGenerator, WorldGenerator,
    },
    interfaces::ChunkData,
    management::{select_backend, Backend, BackendRequirements},
    storage::{StorageError, UnifiedStorage},
};
use std::sync::{Arc, RwLock};

/// Unified world manager that provides a consistent API across GPU and CPU backends
pub struct UnifiedWorldManager {
    storage: UnifiedStorage,
    generator: UnifiedGenerator,
    compute: Option<UnifiedCompute>,
    backend: Backend,
    config: WorldManagerConfig,
    /// Track loaded chunks for GPU backend (since GPU WorldBuffer doesn't track this)
    loaded_gpu_chunks: std::collections::HashSet<ChunkPos>,
    /// CPU cache for chunks when using GPU storage (for mesh generation)
    /// Protected by RwLock for thread-safe concurrent access
    cpu_chunk_cache: Arc<RwLock<std::collections::HashMap<ChunkPos, crate::world::storage::ChunkSoA>>>,
}

impl UnifiedWorldManager {
    /// Create a new GPU-based world manager with a provided generator
    pub async fn new_gpu_with_generator(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: WorldManagerConfig,
        generator: Box<dyn WorldGenerator>,
    ) -> Result<Self, WorldError> {
        // Create GPU storage
        let world_buffer_desc = crate::world::storage::WorldBufferDescriptor {
            view_distance: config.view_distance,
            enable_atomics: true,
            enable_readback: cfg!(debug_assertions),
        };
        let storage = UnifiedStorage::new_gpu(device.clone(), &world_buffer_desc)
            .await
            .map_err(|e| WorldError::StorageInitFailed {
                message: e.to_string(),
            })?;

        // Create GPU buffer manager
        let buffer_manager = Arc::new(crate::gpu::GpuBufferManager::new(
            device.clone(),
            queue.clone(),
        ));

        // Wrap the provided generator as a UnifiedGenerator
        let unified_generator =
            UnifiedGenerator::new_gpu_with_generator(generator, device.clone(), buffer_manager)
                .await
                .map_err(|e| WorldError::GeneratorInitFailed {
                    message: e.to_string(),
                })?;

        // Create GPU compute backend
        let compute = Some(
            UnifiedCompute::new(device.clone(), queue, config.compute_config.clone())
                .await
                .map_err(|e| WorldError::ComputeInitFailed {
                    message: e.to_string(),
                })?,
        );

        Ok(Self {
            storage,
            generator: unified_generator,
            compute,
            backend: Backend::Gpu,
            config,
            loaded_gpu_chunks: std::collections::HashSet::new(),
            cpu_chunk_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Create a new GPU-based world manager
    pub async fn new_gpu(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: WorldManagerConfig,
    ) -> Result<Self, WorldError> {
        // Create GPU storage
        let world_buffer_desc = crate::world::storage::WorldBufferDescriptor {
            view_distance: config.view_distance,
            enable_atomics: true,
            enable_readback: cfg!(debug_assertions),
        };
        let storage = UnifiedStorage::new_gpu(device.clone(), &world_buffer_desc)
            .await
            .map_err(|e| WorldError::StorageInitFailed {
                message: e.to_string(),
            })?;

        // Create GPU generator
        let buffer_manager = Arc::new(crate::gpu::GpuBufferManager::new(
            device.clone(),
            queue.clone(),
        ));

        // Convert to unified generator config
        let gen_config = UnifiedGeneratorConfig::default(); // TODO: Convert from config.generator_config

        let generator = UnifiedGenerator::new_gpu(device.clone(), buffer_manager, gen_config)
            .await
            .map_err(|e| WorldError::GeneratorInitFailed {
                message: e.to_string(),
            })?;

        // Create GPU compute backend
        let compute = Some(
            UnifiedCompute::new(device.clone(), queue, config.compute_config.clone())
                .await
                .map_err(|e| WorldError::ComputeInitFailed {
                    message: e.to_string(),
                })?,
        );

        Ok(Self {
            storage,
            generator,
            compute,
            backend: Backend::Gpu,
            config,
            loaded_gpu_chunks: std::collections::HashSet::new(),
            cpu_chunk_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Create a new CPU-based world manager with a provided generator
    pub fn new_cpu_with_generator(
        config: WorldManagerConfig,
        generator: Box<dyn WorldGenerator>,
    ) -> Result<Self, WorldError> {
        // Create CPU storage
        let storage = UnifiedStorage::new_cpu();

        // Wrap the provided generator as a UnifiedGenerator
        let unified_generator =
            UnifiedGenerator::new_cpu_with_generator(generator).map_err(|e| {
                WorldError::GeneratorInitFailed {
                    message: e.to_string(),
                }
            })?;

        Ok(Self {
            storage,
            generator: unified_generator,
            compute: None,
            backend: Backend::Cpu,
            config,
            loaded_gpu_chunks: std::collections::HashSet::new(),
            cpu_chunk_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Create a new CPU-based world manager
    pub fn new_cpu(config: WorldManagerConfig) -> Result<Self, WorldError> {
        // Create CPU storage
        let storage = UnifiedStorage::new_cpu();

        // Create CPU generator
        // Convert to unified generator config
        let gen_config = UnifiedGeneratorConfig::default(); // TODO: Convert from config.generator_config

        let generator =
            UnifiedGenerator::new_cpu(gen_config).map_err(|e| WorldError::GeneratorInitFailed {
                message: e.to_string(),
            })?;

        Ok(Self {
            storage,
            generator,
            compute: None,
            backend: Backend::Cpu,
            config,
            loaded_gpu_chunks: std::collections::HashSet::new(),
            cpu_chunk_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Create a world manager with automatic backend selection
    pub async fn new_auto(
        device: Option<Arc<wgpu::Device>>,
        queue: Option<Arc<wgpu::Queue>>,
        config: WorldManagerConfig,
    ) -> Result<Self, WorldError> {
        let backend = select_backend(device.as_deref(), &config.backend_requirements).await;

        match backend {
            Backend::Gpu => {
                if let (Some(device), Some(queue)) = (device, queue) {
                    Self::new_gpu(device, queue, config).await
                } else {
                    Err(WorldError::BackendNotAvailable {
                        backend: "GPU".to_string(),
                    })
                }
            }
            Backend::Cpu => Self::new_cpu(config),
            Backend::Auto => unreachable!("Auto should be resolved to Gpu or Cpu"),
        }
    }

    /// Get a block at the specified position
    pub fn get_block(&self, pos: VoxelPos) -> BlockId {
        match &self.storage {
            UnifiedStorage::Gpu { world_buffer, .. } => {
                // Check CPU cache first (used for mesh generation)
                let chunk_pos = pos.to_chunk_pos(self.config.chunk_size);
                let chunk_offset = pos.to_chunk_offset(self.config.chunk_size);

                if let Ok(cache) = self.cpu_chunk_cache.read() {
                    if let Some(chunk) = cache.get(&chunk_pos) {
                        let block = chunk.get_block(
                            chunk_offset.x as u32,
                            chunk_offset.y as u32,
                            chunk_offset.z as u32,
                        );
                        log::trace!("[UnifiedWorldManager::get_block] 📦 Found block {:?} in CPU cache at pos {:?}", block, pos);
                        return block;
                    }
                }

                // TODO: Implement direct GPU block access
                log::trace!("[UnifiedWorldManager::get_block] ⚠️ GPU block access not implemented and not in CPU cache, returning AIR for pos {:?}", pos);
                BlockId::AIR
            }
            UnifiedStorage::Cpu { chunks } => {
                let chunk_pos = pos.to_chunk_pos(self.config.chunk_size);
                let chunk_offset = pos.to_chunk_offset(self.config.chunk_size);

                if let Some(chunk) = chunks.get(&chunk_pos) {
                    chunk.get_block(
                        chunk_offset.x as u32,
                        chunk_offset.y as u32,
                        chunk_offset.z as u32,
                    )
                } else {
                    BlockId::AIR
                }
            }
        }
    }

    /// Set a block at the specified position
    pub fn set_block(&mut self, pos: VoxelPos, block_id: BlockId) -> Result<(), WorldError> {
        match &mut self.storage {
            UnifiedStorage::Gpu { .. } => {
                // TODO: Implement GPU block modification
                Err(WorldError::OperationNotSupported {
                    operation: "set_block on GPU".to_string(),
                })
            }
            UnifiedStorage::Cpu { chunks } => {
                let chunk_pos = pos.to_chunk_pos(self.config.chunk_size);
                let chunk_offset = pos.to_chunk_offset(self.config.chunk_size);

                // Generate chunk if it doesn't exist
                if !chunks.contains_key(&chunk_pos) {
                    let chunk = self
                        .generator
                        .generate_chunk(chunk_pos, self.config.chunk_size);
                    chunks.insert(chunk_pos, chunk);
                }

                if let Some(chunk) = chunks.get_mut(&chunk_pos) {
                    chunk.set_block(
                        chunk_offset.x as u32,
                        chunk_offset.y as u32,
                        chunk_offset.z as u32,
                        block_id,
                    );
                }

                Ok(())
            }
        }
    }

    /// Load a chunk at the specified position
    pub fn load_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), WorldError> {
        match &mut self.storage {
            UnifiedStorage::Gpu {
                world_buffer,
                device,
            } => {
                // GPU chunk loading - generate directly into WorldBuffer
                // For GPU storage, generate chunk on CPU and store in cache
                // TODO: Implement proper GPU chunk generation
                log::info!(
                    "[UnifiedWorldManager::load_chunk] 🎲 Using CPU generator for chunk {:?}",
                    chunk_pos
                );

                let chunk = self
                    .generator
                    .generate_chunk(chunk_pos, self.config.chunk_size);

                log::info!("[UnifiedWorldManager::load_chunk] ✅ Generated chunk {:?}, storing in CPU cache", chunk_pos);

                // Store in CPU cache for mesh generation access
                if let Ok(mut cache) = self.cpu_chunk_cache.write() {
                    cache.insert(chunk_pos, chunk);
                }
                self.loaded_gpu_chunks.insert(chunk_pos);

                log::info!(
                    "[UnifiedWorldManager] Generated chunk {:?} with CPU generator in GPU mode",
                    chunk_pos
                );
                Ok(())
            }
            UnifiedStorage::Cpu { chunks } => {
                if !chunks.contains_key(&chunk_pos) {
                    let chunk = self
                        .generator
                        .generate_chunk(chunk_pos, self.config.chunk_size);
                    chunks.insert(chunk_pos, chunk);
                }
                Ok(())
            }
        }
    }

    /// Check if using GPU backend
    pub fn is_gpu(&self) -> bool {
        self.backend == Backend::Gpu
    }

    /// Get backend type
    pub fn backend(&self) -> Backend {
        self.backend
    }

    /// Get configuration
    pub fn config(&self) -> &WorldManagerConfig {
        &self.config
    }

    /// Check if a chunk is loaded
    pub fn is_chunk_loaded(&self, chunk_pos: ChunkPos) -> bool {
        match &self.storage {
            UnifiedStorage::Gpu { .. } => {
                // Check our tracking set for GPU chunks
                self.loaded_gpu_chunks.contains(&chunk_pos)
            }
            UnifiedStorage::Cpu { chunks } => chunks.contains_key(&chunk_pos),
        }
    }

    /// Get the number of loaded chunks
    pub fn loaded_chunk_count(&self) -> usize {
        match &self.storage {
            UnifiedStorage::Gpu { .. } => self.loaded_gpu_chunks.len(),
            UnifiedStorage::Cpu { chunks } => chunks.len(),
        }
    }

    /// Get surface height at a world position
    pub fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        // Simple height calculation for now
        // TODO: This should query actual terrain data
        let x_int = world_x as i32;
        let z_int = world_z as i32;

        // For now, return a simple height based on position
        // This would normally query the terrain generator or loaded chunks
        64 + ((x_int as f32 * 0.1).sin() * 8.0) as i32 + ((z_int as f32 * 0.1).cos() * 8.0) as i32
    }

    /// Get all loaded chunks (for persistence)
    pub fn chunks(&self) -> Box<dyn Iterator<Item = (ChunkPos, &dyn ChunkData)> + '_> {
        match &self.storage {
            UnifiedStorage::Gpu { .. } => {
                // TODO: Implement GPU chunk iteration
                Box::new(std::iter::empty())
            }
            UnifiedStorage::Cpu { chunks } => Box::new(
                chunks
                    .iter()
                    .map(|(pos, chunk)| (*pos, chunk as &dyn ChunkData)),
            ),
        }
    }

    /// Get a specific chunk
    pub fn get_chunk(&self, chunk_pos: ChunkPos) -> Option<&dyn ChunkData> {
        match &self.storage {
            UnifiedStorage::Gpu { .. } => {
                // TODO: Implement GPU chunk access
                None
            }
            UnifiedStorage::Cpu { chunks } => {
                chunks.get(&chunk_pos).map(|chunk| chunk as &dyn ChunkData)
            }
        }
    }

    /// Get GPU world buffer if available
    pub fn get_world_buffer(
        &self,
    ) -> Option<Arc<std::sync::Mutex<crate::world::storage::WorldBuffer>>> {
        self.storage.gpu_world_buffer()
    }

    /// Get loaded GPU chunks (read-only access)
    pub fn loaded_gpu_chunks(&self) -> &std::collections::HashSet<ChunkPos> {
        &self.loaded_gpu_chunks
    }

    /// Get storage reference for internal access
    pub fn storage(&self) -> &UnifiedStorage {
        &self.storage
    }
}

// StorageConfig and GeneratorConfig are imported from generation module

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub world_size: u32,
    pub world_height: u32,
    pub memory_limit_mb: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            world_size: 256,
            world_height: 256,
            memory_limit_mb: 1024,
        }
    }
}

/// Generator configuration
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    pub seed: u32,
    pub enable_caves: bool,
    pub enable_ores: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            seed: 12345,
            enable_caves: true,
            enable_ores: true,
        }
    }
}

/// Configuration for unified world manager
#[derive(Debug, Clone)]
pub struct WorldManagerConfig {
    pub chunk_size: u32,
    pub view_distance: u32,
    pub storage_config: StorageConfig,
    pub generator_config: GeneratorConfig,
    pub compute_config: UnifiedComputeConfig,
    pub backend_requirements: BackendRequirements,
}

impl Default for WorldManagerConfig {
    fn default() -> Self {
        Self {
            chunk_size: 32,
            view_distance: 8,
            storage_config: StorageConfig::default(),
            generator_config: GeneratorConfig::default(),
            compute_config: UnifiedComputeConfig::default(),
            backend_requirements: BackendRequirements::default(),
        }
    }
}

/// World management errors
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("Storage initialization failed: {message}")]
    StorageInitFailed { message: String },

    #[error("Generator initialization failed: {message}")]
    GeneratorInitFailed { message: String },

    #[error("Compute initialization failed: {message}")]
    ComputeInitFailed { message: String },

    #[error("Backend not available: {backend}")]
    BackendNotAvailable { backend: String },

    #[error("Operation not supported: {operation}")]
    OperationNotSupported { operation: String },

    #[error("Invalid chunk position: {x}, {y}, {z}")]
    InvalidChunkPosition { x: i32, y: i32, z: i32 },

    #[error("Memory allocation failed: {size} bytes")]
    MemoryAllocationFailed { size: u64 },
}
