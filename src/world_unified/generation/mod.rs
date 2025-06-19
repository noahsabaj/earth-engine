//! Unified world generation - GPU TerrainGeneratorSOA (primary) + CPU generators (fallback)
//!
//! This module provides unified world generation that can operate in either
//! GPU-accelerated mode (primary) or CPU fallback mode, with the same interface.

mod terrain_gpu;
mod terrain_cpu;
mod cpu_fallback;
mod caves;
mod ores;
mod unified_generator;
#[cfg(feature = "legacy-world-modules")]
mod legacy_adapter;

// GPU generation (primary)
pub use terrain_gpu::{TerrainGeneratorSOA, TerrainGeneratorSOABuilder};

// CPU generation (fallback)
pub use terrain_cpu::{TerrainGenerator, DefaultWorldGenerator};
pub use cpu_fallback::CpuWorldGenerator;
pub use caves::CaveGenerator;
pub use ores::OreGenerator;

// Unified generation interface
pub use unified_generator::{WorldGenerator, UnifiedGenerator, GeneratorConfig, GeneratorError, BlockIds};

// Legacy compatibility
#[cfg(feature = "legacy-world-modules")]
pub use legacy_adapter::{LegacyGeneratorAdapter, create_legacy_gpu_generator};

/// Create a unified generator that automatically chooses GPU or CPU backend
pub async fn create_unified_generator(
    device: Option<std::sync::Arc<wgpu::Device>>,
    buffer_manager: Option<std::sync::Arc<crate::gpu::GpuBufferManager>>,
    config: GeneratorConfig,
) -> Result<UnifiedGenerator, GeneratorError> {
    if let (Some(device), Some(buffer_manager)) = (device, buffer_manager) {
        // GPU-first path
        UnifiedGenerator::new_gpu(device, buffer_manager, config).await
    } else {
        // CPU fallback path
        UnifiedGenerator::new_cpu(config)
    }
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
            sea_level: 64.0,
            terrain_scale: 0.01,
            mountain_threshold: 0.6,
            cave_threshold: 0.3,
            terrain_amplitude: 40.0,
            terrain_offset: 64.0,
            water_level: 64,
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
        assert_eq!(params.sea_level, 64.0);
    }
    
    #[tokio::test]
    async fn test_cpu_generator_creation() {
        let config = GeneratorConfig::default();
        let generator = create_unified_generator(None, None, config).await;
        assert!(generator.is_ok());
        assert!(!generator.unwrap().is_gpu());
    }
}