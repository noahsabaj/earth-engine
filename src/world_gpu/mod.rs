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

pub mod world_buffer;
pub mod terrain_generator;
pub mod chunk_modifier;
pub mod gpu_lighting;
pub mod unified_memory;
pub mod migration;
pub mod streaming_world;

#[cfg(test)]
mod tests;

pub mod benchmarks;

pub use world_buffer::{WorldBuffer, WorldBufferDescriptor, VoxelData};
pub use terrain_generator::{TerrainGenerator, TerrainParams};
pub use chunk_modifier::{ChunkModifier, ModificationCommand};
pub use gpu_lighting::GpuLighting;
pub use unified_memory::{UnifiedMemoryManager, UnifiedMemoryLayout, SystemType, MemoryStats};
pub use migration::WorldMigrator;
pub use benchmarks::{GpuWorldBenchmarks, PerformanceComparison, PerformanceImprovements};
pub use streaming_world::{StreamingWorldBuffer, WorldStats, create_planet_world};