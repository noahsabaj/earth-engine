//! GPU-first world generation - TerrainGeneratorSOA
//!
//! This module provides GPU-accelerated world generation,
//! following the GPU-first architecture principle.

use crate::constants::terrain::SEA_LEVEL;

mod caves;
mod gpu_world_generator;
mod ores;
mod terrain_gpu;
mod unified_generator;

// GPU generation
pub use gpu_world_generator::GpuWorldGenerator;
pub use terrain_gpu::{TerrainGeneratorSOA, TerrainGeneratorSOABuilder};

// Supporting generators (these should also be GPU-based eventually)
pub use caves::CaveGenerator;
pub use ores::OreGenerator;

// Unified generation interface
pub use unified_generator::{
    BlockIds, GeneratorConfig, GeneratorError, UnifiedGenerator, WorldGenerator,
};

/// Create a GPU-based generator
pub async fn create_unified_generator(
    device: std::sync::Arc<wgpu::Device>,
    buffer_manager: std::sync::Arc<crate::gpu::GpuBufferManager>,
    config: GeneratorConfig,
) -> Result<UnifiedGenerator, GeneratorError> {
    UnifiedGenerator::new_gpu(device, buffer_manager, config).await
}

/// Terrain generation parameters that work across CPU and GPU backends
#[derive(Debug, Clone, Copy)]
pub struct TerrainParams {
    pub seed: u32,
    pub sea_level: f32,
    pub terrain_scale: f32,
    pub mountain_threshold: f32,
    pub cave_threshold: f32,
    pub terrain_amplitude: f32,
    pub terrain_offset: f32,
    pub water_level: i32,
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            seed: 12345,
            sea_level: SEA_LEVEL as f32, // Sea level in voxels
            terrain_scale: 0.01,
            mountain_threshold: 0.6,
            cave_threshold: 0.3,
            terrain_amplitude: 40.0,
            terrain_offset: SEA_LEVEL as f32, // Base terrain height at sea level
            water_level: SEA_LEVEL,           // Water level in voxels
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_params_default() {
        let params = TerrainParams::default();
        assert_eq!(params.seed, 12345);
        assert_eq!(params.sea_level, SEA_LEVEL as f32);
    }

}
