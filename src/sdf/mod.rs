/// Hybrid SDF-Voxel System
/// 
/// Provides smooth terrain rendering using Signed Distance Fields
/// while maintaining voxel-based gameplay mechanics.
/// 
/// Key features:
/// - GPU-accelerated SDF generation from voxel data
/// - Marching cubes for smooth surface extraction
/// - Seamless transitions between voxel and smooth rendering
/// - LOD support with natural smoothing at distance

pub mod error;
pub mod sdf_data;
pub mod sdf_generator;
pub mod marching_cubes;
pub mod surface_extractor;
pub mod hybrid_collision;
pub mod sdf_lod;
pub mod dual_storage;

pub use sdf_data::{SdfBuffer, SdfChunk, SdfConstants, SmoothVertex, SdfValue};
pub use sdf_generator::{SdfGenerator, SdfGenerationParams};
pub use marching_cubes::{MarchingCubes, MarchTable};
pub use surface_extractor::{SurfaceExtractor, SurfaceMesh, ExtractionParams};
pub use hybrid_collision::{HybridCollider, CollisionMode};
pub use sdf_lod::{SdfLod, LodLevel};
pub use dual_storage::{DualRepresentation, RenderMode};

/// SDF cell size relative to voxel size
pub const SDF_RESOLUTION_FACTOR: f32 = 0.5; // 2x resolution for smoother surfaces

/// Margin size for SDF generation (in voxels)
pub const SDF_MARGIN: u32 = 4; // Extra cells for smooth chunk borders

/// Maximum distance to track in SDF
pub const SDF_MAX_DISTANCE: f32 = 8.0; // In voxel units

/// Threshold for surface extraction
pub const SDF_SURFACE_THRESHOLD: f32 = 0.0;

#[cfg(test)]
mod tests;