mod block;
mod basic_blocks;
mod position;
mod ray;
mod registry;
mod world;
mod world_interface;
mod read_only_interface;
pub mod generation;
pub mod chunk_manager;
pub mod concurrent_chunk_manager;
mod concurrent_world;
mod parallel_chunk_manager;
mod parallel_world;
pub mod gpu_chunk;
mod morton_chunk;
mod chunk_soa;
pub mod data_chunk_manager;
mod frame_budget;
mod spawn_finder;

pub use block::{Block, BlockId, RenderData, PhysicsProperties};
pub use basic_blocks::{AirBlock, StoneBlock, GrassBlock, register_basic_blocks};
// Game-specific modules removed from engine core
pub use position::{ChunkPos, VoxelPos};
pub use ray::{Ray, RaycastHit, BlockFace, cast_ray};
pub use registry::BlockRegistry;
pub use world::World;
pub use world_interface::WorldInterface;
pub use chunk_manager::{ChunkManagerData, ChunkManagerConfig, ChunkLoadingStats, create_chunk_manager_data, create_gpu_chunk_manager_data};
pub use concurrent_chunk_manager::ConcurrentChunkManager;
pub use concurrent_world::ConcurrentWorld;
pub use parallel_chunk_manager::{ParallelChunkManager, GenerationStats};
pub use parallel_world::{ParallelWorld, ParallelWorldConfig, WorldPerformanceMetrics};
pub use generation::{WorldGenerator, DefaultWorldGenerator, GpuWorldGenerator, GpuDefaultWorldGenerator, create_gpu_default_world_generator};
pub use gpu_chunk::{GpuChunk, GpuChunkManager, GpuChunkStats};
pub use morton_chunk::MortonChunk;
pub use chunk_soa::{ChunkSoA, ChunkMemoryStats};
pub use data_chunk_manager::{ChunkManagerData as DataChunkManagerData, ChunkMetadata, ChunkBatchOps};
pub use frame_budget::{FrameBudget, ChunkLoadThrottler};
pub use spawn_finder::SpawnFinder;

// Re-export ChunkSoA as Chunk for compatibility
pub use chunk_soa::ChunkSoA as Chunk;

/// Helper function to convert voxel position to chunk position
/// Following DOP principles - pure function that transforms data
pub fn voxel_to_chunk_pos(voxel_pos: VoxelPos, chunk_size: u32) -> ChunkPos {
    voxel_pos.to_chunk_pos(chunk_size)
}

// === MIGRATION COMPATIBILITY LAYER ===
// This allows gradual migration from world to world_unified
// TODO: Remove this after full migration to world_unified

#[cfg(feature = "legacy-world-modules")]
mod unified_compat {
    // Re-export core types from world_unified to maintain compatibility
    pub use crate::world_unified::core::{
        Block as UnifiedBlock,
        BlockId as UnifiedBlockId,
        BlockRegistry as UnifiedBlockRegistry,
        ChunkPos as UnifiedChunkPos,
        VoxelPos as UnifiedVoxelPos,
        Ray as UnifiedRay,
        RaycastHit as UnifiedRaycastHit,
        BlockFace as UnifiedBlockFace,
        RenderData as UnifiedRenderData,
        PhysicsProperties as UnifiedPhysicsProperties,
    };
    
    pub use crate::world_unified::storage::ChunkSoA as UnifiedChunk;
    pub use crate::world_unified::management::UnifiedWorldManager as UnifiedWorld;
    pub use crate::world_unified::generation::WorldGenerator as UnifiedWorldGenerator;
    
    // Type conversion helpers
    pub fn to_unified_block_id(id: super::BlockId) -> UnifiedBlockId {
        UnifiedBlockId(id.0)
    }
    
    pub fn from_unified_block_id(id: UnifiedBlockId) -> super::BlockId {
        super::BlockId(id.0)
    }
    
    pub fn to_unified_voxel_pos(pos: super::VoxelPos) -> UnifiedVoxelPos {
        UnifiedVoxelPos::new(pos.x, pos.y, pos.z)
    }
    
    pub fn from_unified_voxel_pos(pos: UnifiedVoxelPos) -> super::VoxelPos {
        super::VoxelPos::new(pos.x, pos.y, pos.z)
    }
    
    pub fn to_unified_chunk_pos(pos: super::ChunkPos) -> UnifiedChunkPos {
        UnifiedChunkPos { x: pos.x, y: pos.y, z: pos.z }
    }
    
    pub fn from_unified_chunk_pos(pos: UnifiedChunkPos) -> super::ChunkPos {
        super::ChunkPos { x: pos.x, y: pos.y, z: pos.z }
    }
}

#[cfg(feature = "legacy-world-modules")]
pub use unified_compat::*;