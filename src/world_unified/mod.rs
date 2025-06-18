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

pub mod core;
pub mod storage;
pub mod generation;
pub mod compute;
pub mod management;
pub mod interfaces;

// Re-export core types for convenience
pub use core::{
    Block, BlockId, RenderData, PhysicsProperties,
    ChunkPos, VoxelPos, 
    Ray, RaycastHit, BlockFace, cast_ray,
    BlockRegistry
};

// Re-export storage systems
pub use storage::{
    // GPU-first storage
    WorldBuffer, WorldBufferDescriptor, VoxelData,
    // CPU fallback storage
    ChunkSoA as Chunk, ChunkMemoryStats,
    GpuChunk, GpuChunkManager, GpuChunkStats
};

// Re-export generation systems
pub use generation::{
    // Unified generation interface
    WorldGenerator,
    // GPU generators (primary)
    TerrainGeneratorSOA, TerrainGeneratorSOABuilder,
    // CPU generators (fallback)
    DefaultWorldGenerator, TerrainGenerator, CaveGenerator, OreGenerator
};

// Re-export compute systems
pub use compute::{
    // GPU kernels and optimization
    UnifiedWorldKernel, UnifiedKernelConfig, SystemFlags,
    SparseVoxelOctree, OctreeNode, OctreeStats,
    VoxelBvh, BvhNode, BvhStats,
    HierarchicalPhysics, PhysicsQuery, QueryResult,
    // GPU lighting and effects
    GpuLighting, WeatherGpu, WeatherData
};

// Re-export management systems
pub use management::{
    // Unified managers
    UnifiedWorldManager, WorldManagerConfig,
    // Performance and statistics
    WorldPerformanceMetrics, GenerationStats
};

// Re-export interfaces
pub use interfaces::{
    WorldInterface, ReadOnlyWorldInterface,
    ChunkManagerInterface, GeneratorInterface
};

/// Helper function to convert voxel position to chunk position
/// Following DOP principles - pure function that transforms data
pub fn voxel_to_chunk_pos(voxel_pos: VoxelPos, chunk_size: u32) -> ChunkPos {
    voxel_pos.to_chunk_pos(chunk_size)
}

/// Unified world creation function that automatically chooses GPU or CPU backend
pub async fn create_unified_world(
    device: Option<std::sync::Arc<wgpu::Device>>,
    config: WorldManagerConfig,
) -> Result<UnifiedWorldManager, crate::world_unified::management::WorldError> {
    if let Some(device) = device {
        // GPU-first path
        UnifiedWorldManager::new_gpu(device, config).await
    } else {
        // CPU fallback path
        UnifiedWorldManager::new_cpu(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_to_chunk_conversion() {
        let voxel_pos = VoxelPos { x: 65, y: 32, z: -15 };
        let chunk_pos = voxel_to_chunk_pos(voxel_pos, 32);
        
        // 65 / 32 = 2, 32 / 32 = 1, -15 / 32 = -1
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