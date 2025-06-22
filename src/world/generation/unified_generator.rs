//! Unified generation interface that works across CPU and GPU backends

use super::{DefaultWorldGenerator, TerrainParams};
use crate::world::core::{BlockId, ChunkPos};
use crate::world::storage::ChunkSoA;

/// Universal world generation interface
pub trait WorldGenerator: Send + Sync {
    /// Generate a chunk at the given position
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> ChunkSoA;

    /// Get surface height at world coordinates
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32;

    /// Find a safe spawn height
    fn find_safe_spawn_height(&self, world_x: f64, world_z: f64) -> f32 {
        let surface_height = self.get_surface_height(world_x, world_z);
        (surface_height as f32 + 3.0).clamp(20.0, 250.0)
    }

    /// Check if this generator uses GPU backend
    fn is_gpu(&self) -> bool;

    /// Get GPU world buffer if available
    fn get_world_buffer(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<crate::world::storage::WorldBuffer>>> {
        None
    }
}

/// Unified generator that can operate in GPU or CPU mode
pub enum UnifiedGenerator {
    /// GPU-accelerated generation (primary)
    Gpu {
        generator: Box<dyn WorldGenerator>,
        device: std::sync::Arc<wgpu::Device>,
        buffer_manager: std::sync::Arc<crate::gpu::GpuBufferManager>,
    },
    /// CPU-based generation (fallback)
    Cpu { generator: Box<dyn WorldGenerator> },
}

impl UnifiedGenerator {
    /// Create GPU-based generator with a provided generator
    pub async fn new_gpu_with_generator(
        generator: Box<dyn WorldGenerator>,
        device: std::sync::Arc<wgpu::Device>,
        buffer_manager: std::sync::Arc<crate::gpu::GpuBufferManager>,
    ) -> Result<Self, GeneratorError> {
        Ok(UnifiedGenerator::Gpu {
            generator,
            device,
            buffer_manager,
        })
    }

    /// Create GPU-based generator
    pub async fn new_gpu(
        device: std::sync::Arc<wgpu::Device>,
        buffer_manager: std::sync::Arc<crate::gpu::GpuBufferManager>,
        config: GeneratorConfig,
    ) -> Result<Self, GeneratorError> {
        // Create the GPU terrain generator
        let terrain_generator = super::TerrainGeneratorSOABuilder::new()
            .with_vectorization(config.use_vectorization)
            .build(device.clone(), buffer_manager.clone())
            .map_err(|e| GeneratorError::InitError(format!("Failed to create terrain generator: {:?}", e)))?;

        // Create world buffer for GPU operations
        let world_buffer_desc = crate::world::storage::WorldBufferDescriptor {
            view_distance: 16, // 16 chunks view distance
            enable_atomics: true,
            enable_readback: false,
        };
        let world_buffer = std::sync::Arc::new(std::sync::Mutex::new(
            crate::world::storage::WorldBuffer::new(device.clone(), &world_buffer_desc),
        ));

        // Note: GPU generation through the WorldGenerator trait will return empty chunks
        // since the trait doesn't provide access to command encoders. Actual GPU generation
        // must be done through the renderer when a command encoder is available.

        // Create GPU world generator wrapper
        let gpu_generator = super::GpuWorldGenerator::new(
            std::sync::Arc::new(terrain_generator),
            device.clone(),
            buffer_manager.queue().clone(),
            world_buffer,
        );

        Ok(UnifiedGenerator::Gpu {
            generator: Box::new(gpu_generator) as Box<dyn WorldGenerator>,
            device,
            buffer_manager,
        })
    }

    /// Create CPU-based generator with a provided generator
    pub fn new_cpu_with_generator(
        generator: Box<dyn WorldGenerator>,
    ) -> Result<Self, GeneratorError> {
        Ok(UnifiedGenerator::Cpu { generator })
    }

    /// Create CPU-based generator
    pub fn new_cpu(config: GeneratorConfig) -> Result<Self, GeneratorError> {
        let generator = DefaultWorldGenerator::new(config.terrain_params.seed);

        Ok(UnifiedGenerator::Cpu {
            generator: Box::new(generator) as Box<dyn WorldGenerator>,
        })
    }

    /// Check if using GPU backend
    pub fn is_gpu(&self) -> bool {
        matches!(self, UnifiedGenerator::Gpu { .. })
    }
}

impl WorldGenerator for UnifiedGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> ChunkSoA {
        match self {
            UnifiedGenerator::Gpu { generator, .. } => {
                // Delegate to the actual generator
                generator.generate_chunk(chunk_pos, chunk_size)
            }
            UnifiedGenerator::Cpu { generator } => generator.generate_chunk(chunk_pos, chunk_size),
        }
    }

    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        match self {
            UnifiedGenerator::Gpu { generator, .. } => {
                generator.get_surface_height(world_x, world_z)
            }
            UnifiedGenerator::Cpu { generator } => generator.get_surface_height(world_x, world_z),
        }
    }

    fn is_gpu(&self) -> bool {
        self.is_gpu()
    }

    fn get_world_buffer(
        &self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<crate::world::storage::WorldBuffer>>> {
        match self {
            UnifiedGenerator::Gpu { generator, .. } => generator.get_world_buffer(),
            UnifiedGenerator::Cpu { generator } => generator.get_world_buffer(),
        }
    }
}

/// Configuration for unified generator
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    pub terrain_params: TerrainParams,
    pub block_ids: BlockIds,
    pub use_vectorization: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            terrain_params: TerrainParams::default(),
            block_ids: BlockIds::default(),
            use_vectorization: true,
        }
    }
}

/// Block IDs for generation
#[derive(Debug, Clone, Copy)]
pub struct BlockIds {
    pub air: BlockId,
    pub grass: BlockId,
    pub dirt: BlockId,
    pub stone: BlockId,
    pub water: BlockId,
    pub sand: BlockId,
}

impl Default for BlockIds {
    fn default() -> Self {
        Self {
            air: BlockId::AIR,
            grass: BlockId::GRASS,
            dirt: BlockId::DIRT,
            stone: BlockId::STONE,
            water: BlockId::WATER,
            sand: BlockId::SAND,
        }
    }
}

/// Generation errors
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("GPU initialization failed: {message}")]
    GpuInitFailed { message: String },

    #[error("Invalid configuration: {field}")]
    InvalidConfig { field: String },

    #[error("Backend not available: {backend}")]
    BackendNotAvailable { backend: String },
    
    #[error("Initialization error: {0}")]
    InitError(String),
}
