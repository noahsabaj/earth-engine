use crate::world::{BlockId, VoxelPos, ChunkPos, Chunk};
use cgmath::Point3;
use std::collections::{HashMap, HashSet};

/// Common interface for world implementations
/// This trait allows GameContext to work with both World and ParallelWorld
pub trait WorldInterface: Send + Sync {
    /// Get a block at the given position
    fn get_block(&self, pos: VoxelPos) -> BlockId;
    
    /// Set a block at the given position
    fn set_block(&mut self, pos: VoxelPos, block: BlockId);
    
    /// Update loaded chunks based on player position
    fn update_loaded_chunks(&mut self, player_pos: Point3<f32>);
    
    /// Get chunk size
    fn chunk_size(&self) -> u32;
    
    /// Check if a block position is in bounds (for infinite worlds, always true)
    fn is_block_in_bounds(&self, pos: VoxelPos) -> bool {
        true // Default implementation for infinite worlds
    }
    
    /// Get sky light level at position
    fn get_sky_light(&self, pos: VoxelPos) -> u8;
    
    /// Set sky light level at position
    fn set_sky_light(&mut self, pos: VoxelPos, level: u8);
    
    /// Get block light level at position
    fn get_block_light(&self, pos: VoxelPos) -> u8;
    
    /// Set block light level at position
    fn set_block_light(&mut self, pos: VoxelPos, level: u8);
    
    /// Check if a chunk is loaded
    fn is_chunk_loaded(&self, pos: ChunkPos) -> bool;
    
    /// Take dirty chunks that need remeshing
    fn take_dirty_chunks(&mut self) -> HashSet<ChunkPos>;
    
    /// Get the surface height at the given world coordinates
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32;
    
    /// Check if a block is transparent (for lighting)
    fn is_block_transparent(&self, pos: VoxelPos) -> bool;
}