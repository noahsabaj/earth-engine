//! Unified World Module - GPU-first with CPU fallbacks
//!
//! This module consolidates all world functionality into a coherent GPU-first
//! architecture while maintaining CPU fallbacks for compatibility and flexibility.
//!
//! # Architecture Overview
//!
//! - **Core**: Fundamental data types (Block, Position, Ray)
//! - **Storage**: GPU WorldBuffer (primary) + CPU Chunks (fallback)
//! - **Generation**: GPU TerrainGeneratorSOA (primary) + CPU generators (fallback)
//! - **Compute**: GPU kernels, shaders, and optimization structures
//! - **Management**: Unified chunk/world managers supporting both CPU and GPU modes
//! - **Interfaces**: Clean abstractions that work across CPU/GPU implementations
//!
//! # Design Principles
//!
//! 1. **GPU-first**: GPU systems are the primary, high-performance path
//! 2. **CPU fallback**: CPU systems provide compatibility and debugging
//! 3. **Unified API**: Same interface whether using CPU or GPU backend
//! 4. **DOP architecture**: Data-oriented design throughout
//! 5. **Zero-copy**: Minimize CPU/GPU transfers where possible

pub mod blocks;
pub mod compute;
pub mod core;
pub mod error;
pub mod generation;
pub mod interfaces;
pub mod lighting;
pub mod management;
pub mod storage;
pub mod weather_manager;

// Re-export core types for convenience
pub use core::{
    Block, BlockFace, BlockId, BlockRegistry, ChunkPos, PhysicsProperties, Ray, RaycastHit,
    RenderData, VoxelPos,
};

// Re-export storage systems
pub use storage::{
    ChunkMemoryStats,
    // CPU fallback storage
    ChunkSoA as Chunk,
    GpuChunk,
    GpuChunkManager,
    GpuChunkStats,
    VoxelData,
    // GPU-first storage
    WorldBuffer,
    WorldBufferDescriptor,
};

// Re-export generation systems
pub use generation::{
    CaveGenerator,
    // CPU generators (fallback)
    DefaultWorldGenerator,
    OreGenerator,
    TerrainGenerator,
    // GPU generators (primary)
    TerrainGeneratorSOA,
    TerrainGeneratorSOABuilder,
    // Unified generation interface
    WorldGenerator,
};

// Re-export compute systems
pub use compute::{
    // GPU optimization structures will be added later
    // GPU lighting and effects
    GpuLighting,
    PrecipitationParticle,
    SystemFlags,
    UnifiedKernelConfig,
    // GPU kernels and optimization
    UnifiedWorldKernel,
    WeatherData,
    WeatherGpu,
};

// Re-export management systems
pub use management::{
    GenerationStats,
    // Parallel world support
    ParallelWorld,
    ParallelWorldConfig,
    SpawnFinder,
    // Unified managers
    UnifiedWorldManager,
    WorldManagerConfig,
    // Performance and statistics
    WorldPerformanceMetrics,
};

// Re-export interfaces
pub use interfaces::{
    ChunkData, ChunkManager, ChunkManagerInterface, DefaultChunkManager, GeneratorInterface,
    OperationResult, QueryResult, ReadOnlyWorldInterface, UnifiedWorldInterface, WorldConfig,
    WorldError, WorldInterface, WorldOperation, WorldQuery,
};

// Re-export block system
pub use blocks::{
    register_basic_blocks, DirtBlock, GlowstoneBlock, GrassBlock, SandBlock, StoneBlock, WaterBlock,
};

// Re-export lighting system
pub use lighting::{
    DayNightCycleData, LightLevel, LightType, LightUpdate, LightingStats, SkylightCalculator,
    TimeOfDayData,
};

// Re-export weather system
pub use weather_manager::{WeatherManager, WeatherZone};

/// Helper function to convert voxel position to chunk position
/// Following DOP principles - pure function that transforms data
pub fn voxel_to_chunk_pos(voxel_pos: VoxelPos, chunk_size: u32) -> ChunkPos {
    voxel_pos.to_chunk_pos(chunk_size)
}

/// Unified world creation function that automatically chooses GPU or CPU backend
pub async fn create_unified_world(
    device: Option<std::sync::Arc<wgpu::Device>>,
    queue: Option<std::sync::Arc<wgpu::Queue>>,
    config: WorldManagerConfig,
) -> Result<UnifiedWorldManager, crate::world::management::WorldError> {
    if let (Some(device), Some(queue)) = (device, queue) {
        // GPU-first path
        UnifiedWorldManager::new_gpu(device, queue, config).await
    } else {
        // CPU fallback path
        UnifiedWorldManager::new_cpu(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::core::CHUNK_SIZE;

    #[test]
    fn test_voxel_to_chunk_conversion() {
        let voxel_pos = VoxelPos {
            x: 65,
            y: 32,
            z: -15,
        };
        let chunk_pos = voxel_to_chunk_pos(voxel_pos, 32);

        // 65 / 32 = 2, 32 / 32 = 1, -15 / 32 = -1
        assert_eq!(chunk_pos.x, 2);
        assert_eq!(chunk_pos.y, 1);
        assert_eq!(chunk_pos.z, -1);
    }

    #[test]
    fn test_voxel_to_chunk_conversion_with_constant() {
        // Test with actual chunk size constant
        let voxel_pos = VoxelPos {
            x: 125,
            y: 75,
            z: -25,
        };
        let chunk_pos = voxel_to_chunk_pos(voxel_pos, CHUNK_SIZE);

        // With CHUNK_SIZE=50: 125/50=2, 75/50=1, -25/50=-1
        assert_eq!(chunk_pos.x, 2);
        assert_eq!(chunk_pos.y, 1);
        assert_eq!(chunk_pos.z, -1);
    }

    #[test]
    fn test_block_id_constants() {
        assert_eq!(BlockId::AIR, BlockId(0));
        assert_ne!(BlockId::STONE, BlockId::AIR);
    }
}
