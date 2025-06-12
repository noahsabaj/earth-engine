use std::sync::Arc;
use std::collections::HashMap;
use crate::{BlockId, ChunkPos, Chunk, VoxelPos};
use crate::world::{ParallelWorld};

/// Wrapper that provides World-like interface for ParallelWorld
/// This is a temporary solution to make ParallelWorld work with existing GameContext
pub struct ParallelWorldWrapper {
    parallel_world: Arc<ParallelWorld>,
    chunk_size: u32,
}

impl ParallelWorldWrapper {
    pub fn new(parallel_world: Arc<ParallelWorld>) -> Self {
        let chunk_size = parallel_world.config().chunk_size;
        Self {
            parallel_world,
            chunk_size,
        }
    }
    
    pub fn update_loaded_chunks(&mut self, player_pos: cgmath::Point3<f32>) {
        self.parallel_world.update(player_pos);
    }
    
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        // Can't provide direct reference due to Arc<RwLock>
        // This is a limitation of the current design
        None
    }
    
    pub fn get_chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        // Can't provide mutable reference due to Arc<RwLock>
        None
    }
    
    pub fn set_chunk(&mut self, pos: ChunkPos, chunk: Chunk) {
        // Not supported for parallel world
    }
    
    pub fn get_block(&self, pos: VoxelPos) -> BlockId {
        self.parallel_world.get_block(pos)
    }
    
    pub fn set_block(&mut self, pos: VoxelPos, block: BlockId) {
        self.parallel_world.set_block(pos, block);
    }
    
    pub fn chunks(&self) -> &HashMap<ChunkPos, Chunk> {
        // Can't provide direct reference to chunks
        panic!("Direct chunk access not supported for ParallelWorld");
    }
    
    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }
    
    pub fn is_block_in_bounds(&self, pos: VoxelPos) -> bool {
        true // Infinite world
    }
    
    pub fn take_dirty_chunks(&mut self) -> std::collections::HashSet<ChunkPos> {
        // ParallelWorld handles dirty chunks internally
        std::collections::HashSet::new()
    }
    
    // Lighting methods
    pub fn get_sky_light(&self, pos: VoxelPos) -> u8 {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size());
        let local_pos = pos.to_local_pos(self.chunk_size());
        
        if let Some(chunk_lock) = self.parallel_world.chunk_manager().get_chunk(chunk_pos) {
            let chunk = chunk_lock.read();
            chunk.get_sky_light(local_pos.0, local_pos.1, local_pos.2)
        } else {
            0
        }
    }
    
    pub fn set_sky_light(&mut self, pos: VoxelPos, level: u8) {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size());
        let local_pos = pos.to_local_pos(self.chunk_size());
        
        if let Some(chunk_lock) = self.parallel_world.chunk_manager().get_chunk(chunk_pos) {
            let mut chunk = chunk_lock.write();
            chunk.set_sky_light(local_pos.0, local_pos.1, local_pos.2, level);
        }
    }
    
    pub fn get_block_light(&self, pos: VoxelPos) -> u8 {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size());
        let local_pos = pos.to_local_pos(self.chunk_size());
        
        if let Some(chunk_lock) = self.parallel_world.chunk_manager().get_chunk(chunk_pos) {
            let chunk = chunk_lock.read();
            chunk.get_block_light(local_pos.0, local_pos.1, local_pos.2)
        } else {
            0
        }
    }
    
    pub fn set_block_light(&mut self, pos: VoxelPos, level: u8) {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size());
        let local_pos = pos.to_local_pos(self.chunk_size());
        
        if let Some(chunk_lock) = self.parallel_world.chunk_manager().get_chunk(chunk_pos) {
            let mut chunk = chunk_lock.write();
            chunk.set_block_light(local_pos.0, local_pos.1, local_pos.2, level);
        }
    }
}