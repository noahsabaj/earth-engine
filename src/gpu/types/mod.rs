//! GPU type definitions for all systems

pub mod core;
pub mod terrain;
pub mod weather;
pub mod world;
// Future modules:
// pub mod lighting;
// pub mod physics;
// pub mod particles;

// Re-export core traits
pub use core::{GpuData, TypedGpuBuffer, Vec2, Vec3, Vec4};

// Re-export terrain types
pub use terrain::{BlockDistribution, TerrainParams};

// Re-export world types
pub use world::{ChunkMetadata, VoxelData};

// Re-export weather types
pub use weather::{
    PrecipitationParticleGpu, WeatherConfigGpu, WeatherDataGpu, WeatherTransitionGpu,
};

pub use crate::constants::core::MAX_BLOCK_DISTRIBUTIONS;
