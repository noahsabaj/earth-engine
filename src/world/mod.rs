mod block;
mod basic_blocks;
mod block_drops;
mod block_entity;
mod zero_alloc_block_entity;
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
pub use basic_blocks::{AirBlock, StoneBlock, GrassBlock};
pub use block_drops::{BlockDropHandler, MiningProgress};
pub use block_entity::{BlockEntity, BlockEntityData, FurnaceBlockEntity, ChestBlockEntity};
pub use zero_alloc_block_entity::{BlockEntityKeys, serialize_furnace_zero_alloc, serialize_chest_zero_alloc, KEYS, SLOT_KEYS};
pub use position::{ChunkPos, VoxelPos};
pub use ray::{Ray, RaycastHit, BlockFace, cast_ray};
pub use registry::BlockRegistry;
pub use world::World;
pub use world_interface::WorldInterface;
pub use chunk_manager::{ChunkManagerData, ChunkManagerConfig, ChunkLoadingStats, create_chunk_manager_data};
pub use concurrent_chunk_manager::ConcurrentChunkManager;
pub use concurrent_world::ConcurrentWorld;
pub use parallel_chunk_manager::{ParallelChunkManager, GenerationStats};
pub use parallel_world::{ParallelWorld, ParallelWorldConfig, WorldPerformanceMetrics};
pub use generation::{WorldGenerator, DefaultWorldGenerator};
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