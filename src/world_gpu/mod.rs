/// GPU World Architecture
/// 
/// This module implements Sprint 21's GPU-resident world system where
/// all world data lives permanently on the GPU. The CPU acts only as
/// a coordinator, providing hints and high-level commands.
///
/// Key components:
/// - WorldBuffer: Unified GPU buffer for all world data
/// - GPU terrain generation using compute shaders
/// - Atomic operations for world modifications
/// - Zero-copy architecture between generation and rendering

pub mod error;
pub mod world_buffer;
pub mod terrain_generator;
pub mod chunk_modifier;
pub mod gpu_lighting;
pub mod gpu_lighting_migration;
pub mod unified_memory;
pub mod migration;
pub mod unified_kernel;
pub mod sparse_octree;
pub mod bvh;
pub mod hierarchical_physics;
pub mod unified_benchmark;
pub mod weather_gpu;
pub mod weather_migration;
// pub mod streaming_world; // Temporarily disabled for Sprint 27

#[cfg(test)]
mod tests;

pub mod benchmarks;

pub use world_buffer::{WorldBuffer, WorldBufferDescriptor, VoxelData};
pub use terrain_generator::{TerrainGenerator, TerrainParams};
pub use chunk_modifier::{ChunkModifier, ModificationCommand};
pub use gpu_lighting::GpuLighting;
pub use gpu_lighting_migration::{GpuLightPropagator, GpuBlockProvider, migrate_to_gpu_lighting};
pub use unified_memory::{UnifiedMemoryManager, UnifiedMemoryLayout, SystemType, MemoryStats};
pub use migration::WorldMigrator;
pub use unified_kernel::{UnifiedWorldKernel, UnifiedKernelConfig, SystemFlags};
pub use sparse_octree::{SparseVoxelOctree, OctreeNode, OctreeStats, OctreeUpdater};
pub use bvh::{VoxelBvh, BvhNode, BvhStats};
pub use hierarchical_physics::{HierarchicalPhysics, PhysicsQuery, QueryResult, QueryType};
pub use unified_benchmark::{UnifiedKernelBenchmark, UnifiedBenchmarkResults};
pub use benchmarks::{GpuWorldBenchmarks, PerformanceComparison, PerformanceImprovements};
pub use weather_gpu::{WeatherGpu, WeatherGpuDescriptor, WeatherData, WeatherTransition, PrecipitationParticle, WeatherConfig};
// pub use streaming_world::{StreamingWorldBuffer, WorldStats, create_planet_world}; // Temporarily disabled