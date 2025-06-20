//! Unified world manager that works with GPU and CPU backends

use std::sync::Arc;
use crate::world::{
    core::{ChunkPos, VoxelPos, BlockId},
    storage::{UnifiedStorage, StorageError},
    generation::{UnifiedGenerator, GeneratorError, GeneratorConfig as UnifiedGeneratorConfig, WorldGenerator},
    compute::{UnifiedCompute, UnifiedComputeConfig, ComputeError},
    management::{Backend, BackendRequirements, select_backend},
    interfaces::ChunkData,
};

/// Unified world manager that provides a consistent API across GPU and CPU backends
pub struct UnifiedWorldManager {
    storage: UnifiedStorage,
    generator: UnifiedGenerator,
    compute: Option<UnifiedCompute>,
    backend: Backend,
    config: WorldManagerConfig,
}

impl UnifiedWorldManager {
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
            .map_err(|e| WorldError::StorageInitFailed { message: e.to_string() })?;
        
        // Create GPU generator
        let buffer_manager = Arc::new(crate::gpu::GpuBufferManager::new(device.clone(), queue.clone()));
        
        // Convert to unified generator config
        let gen_config = UnifiedGeneratorConfig::default(); // TODO: Convert from config.generator_config
        
        let generator = UnifiedGenerator::new_gpu(
            device.clone(),
            buffer_manager,
            gen_config,
        )
        .await
        .map_err(|e| WorldError::GeneratorInitFailed { message: e.to_string() })?;
        
        // Create GPU compute backend
        let compute = Some(
            UnifiedCompute::new(device.clone(), queue, config.compute_config.clone())
                .await
                .map_err(|e| WorldError::ComputeInitFailed { message: e.to_string() })?,
        );
        
        Ok(Self {
            storage,
            generator,
            compute,
            backend: Backend::Gpu,
            config,
        })
    }
    
    /// Create a new CPU-based world manager
    pub fn new_cpu(config: WorldManagerConfig) -> Result<Self, WorldError> {
        // Create CPU storage
        let storage = UnifiedStorage::new_cpu();
        
        // Create CPU generator
        // Convert to unified generator config
        let gen_config = UnifiedGeneratorConfig::default(); // TODO: Convert from config.generator_config
        
        let generator = UnifiedGenerator::new_cpu(gen_config)
            .map_err(|e| WorldError::GeneratorInitFailed { message: e.to_string() })?;
        
        Ok(Self {
            storage,
            generator,
            compute: None,
            backend: Backend::Cpu,
            config,
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
                    Err(WorldError::BackendNotAvailable { backend: "GPU".to_string() })
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
                // TODO: Implement GPU block access
                BlockId::AIR
            }
            UnifiedStorage::Cpu { chunks } => {
                let chunk_pos = pos.to_chunk_pos(self.config.chunk_size);
                let chunk_offset = pos.to_chunk_offset(self.config.chunk_size);
                
                if let Some(chunk) = chunks.get(&chunk_pos) {
                    chunk.get_block(chunk_offset.x as u32, chunk_offset.y as u32, chunk_offset.z as u32)
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
                Err(WorldError::OperationNotSupported { operation: "set_block on GPU".to_string() })
            }
            UnifiedStorage::Cpu { chunks } => {
                let chunk_pos = pos.to_chunk_pos(self.config.chunk_size);
                let chunk_offset = pos.to_chunk_offset(self.config.chunk_size);
                
                // Generate chunk if it doesn't exist
                if !chunks.contains_key(&chunk_pos) {
                    let chunk = self.generator.generate_chunk(chunk_pos, self.config.chunk_size);
                    chunks.insert(chunk_pos, chunk);
                }
                
                if let Some(chunk) = chunks.get_mut(&chunk_pos) {
                    chunk.set_block(chunk_offset.x as u32, chunk_offset.y as u32, chunk_offset.z as u32, block_id);
                }
                
                Ok(())
            }
        }
    }
    
    /// Load a chunk at the specified position
    pub fn load_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), WorldError> {
        match &mut self.storage {
            UnifiedStorage::Gpu { .. } => {
                // TODO: Implement GPU chunk loading
                Ok(())
            }
            UnifiedStorage::Cpu { chunks } => {
                if !chunks.contains_key(&chunk_pos) {
                    let chunk = self.generator.generate_chunk(chunk_pos, self.config.chunk_size);
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
                // TODO: Implement GPU chunk checking
                false
            }
            UnifiedStorage::Cpu { chunks } => {
                chunks.contains_key(&chunk_pos)
            }
        }
    }
    
    /// Get the number of loaded chunks
    pub fn loaded_chunk_count(&self) -> usize {
        match &self.storage {
            UnifiedStorage::Gpu { .. } => {
                // TODO: Implement GPU chunk counting
                0
            }
            UnifiedStorage::Cpu { chunks } => {
                chunks.len()
            }
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
            UnifiedStorage::Cpu { chunks } => {
                Box::new(chunks.iter().map(|(pos, chunk)| (*pos, chunk as &dyn ChunkData)))
            }
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