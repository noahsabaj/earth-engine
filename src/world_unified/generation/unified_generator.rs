//! Unified generation interface that works across CPU and GPU backends

use crate::world_unified::core::{ChunkPos, BlockId};
use crate::world_unified::storage::ChunkSoA;
use super::{TerrainGeneratorSOA, TerrainGeneratorSOABuilder, DefaultWorldGenerator, TerrainParams};

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
    fn get_world_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::world_unified::storage::WorldBuffer>>> {
        None
    }
}

/// Unified generator that can operate in GPU or CPU mode
pub enum UnifiedGenerator {
    /// GPU-accelerated generation (primary)
    Gpu {
        generator: TerrainGeneratorSOA,
        device: std::sync::Arc<wgpu::Device>,
        buffer_manager: std::sync::Arc<crate::gpu::GpuBufferManager>,
    },
    /// CPU-based generation (fallback)
    Cpu {
        generator: DefaultWorldGenerator,
    },
}

impl UnifiedGenerator {
    /// Create GPU-based generator
    pub async fn new_gpu(
        device: std::sync::Arc<wgpu::Device>,
        buffer_manager: std::sync::Arc<crate::gpu::GpuBufferManager>,
        config: GeneratorConfig,
    ) -> Result<Self, GeneratorError> {
        let generator = TerrainGeneratorSOABuilder::new()
            .with_vectorization(config.use_vectorization)
            .build(device.clone(), buffer_manager.clone());
            
        Ok(UnifiedGenerator::Gpu {
            generator,
            device,
            buffer_manager,
        })
    }
    
    /// Create CPU-based generator
    pub fn new_cpu(config: GeneratorConfig) -> Result<Self, GeneratorError> {
        let generator = DefaultWorldGenerator::new(
            config.terrain_params.seed,
            config.block_ids.grass,
            config.block_ids.dirt,
            config.block_ids.stone,
            config.block_ids.water,
            config.block_ids.sand,
        );
        
        Ok(UnifiedGenerator::Cpu { generator })
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
                // TODO: Implement GPU chunk generation
                // For now, fall back to a simple chunk
                ChunkSoA::new(chunk_pos, chunk_size)
            }
            UnifiedGenerator::Cpu { generator } => {
                generator.generate_chunk(chunk_pos, chunk_size)
            }
        }
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        match self {
            UnifiedGenerator::Gpu { .. } => {
                // TODO: Implement GPU surface height query
                64 // Placeholder
            }
            UnifiedGenerator::Cpu { generator } => {
                generator.get_surface_height(world_x, world_z)
            }
        }
    }
    
    fn is_gpu(&self) -> bool {
        self.is_gpu()
    }
    
    fn get_world_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::world_unified::storage::WorldBuffer>>> {
        match self {
            UnifiedGenerator::Gpu { generator, .. } => {
                // TODO: Get world buffer from GPU generator
                None
            }
            UnifiedGenerator::Cpu { .. } => None,
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
    pub grass: BlockId,
    pub dirt: BlockId,
    pub stone: BlockId,
    pub water: BlockId,
    pub sand: BlockId,
}

impl Default for BlockIds {
    fn default() -> Self {
        Self {
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
}