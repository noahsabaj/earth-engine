//! Unified world manager that works with GPU and CPU backends

use std::sync::Arc;
use crate::world_unified::{
    core::{ChunkPos, VoxelPos, BlockId},
    storage::{UnifiedStorage, StorageError},
    generation::{UnifiedGenerator, GeneratorConfig, GeneratorError},
    compute::{UnifiedCompute, UnifiedComputeConfig, ComputeError},
    management::{Backend, BackendRequirements, select_backend},
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
        config: WorldManagerConfig,
    ) -> Result<Self, WorldError> {
        let queue = Arc::new(device.queue());
        
        // Create GPU storage
        let storage = UnifiedStorage::new_gpu(device.clone(), &config.storage_config)
            .await
            .map_err(|e| WorldError::StorageInitFailed { message: e.to_string() })?;
        
        // Create GPU generator
        let buffer_manager = Arc::new(crate::gpu::GpuBufferManager::new(device.clone()));
        let generator = UnifiedGenerator::new_gpu(
            device.clone(),
            buffer_manager,
            config.generator_config.clone(),
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
        let generator = UnifiedGenerator::new_cpu(config.generator_config.clone())
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
        config: WorldManagerConfig,
    ) -> Result<Self, WorldError> {
        let backend = select_backend(device.as_deref(), &config.backend_requirements).await;
        
        match backend {
            Backend::Gpu => {
                if let Some(device) = device {
                    Self::new_gpu(device, config).await
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

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub max_loaded_chunks: usize,
    pub memory_limit_mb: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_loaded_chunks: 1024,
            memory_limit_mb: 512,
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