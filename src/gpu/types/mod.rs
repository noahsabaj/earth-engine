//! GPU type definitions for all systems

pub mod core;
pub mod terrain;
// Future modules:
// pub mod lighting;
// pub mod physics;
// pub mod particles;

// Re-export core traits
pub use core::{GpuData, TypedGpuBuffer, Vec2, Vec3, Vec4};

// Re-export terrain types
pub use terrain::{BlockDistribution, TerrainParams, MAX_BLOCK_DISTRIBUTIONS};