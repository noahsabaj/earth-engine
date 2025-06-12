use crate::world::{BlockId, Chunk, ChunkPos, VoxelPos, ChunkManager, WorldGenerator, WorldInterface};
use std::collections::{HashMap, HashSet};
use cgmath::Point3;

pub struct World {
    chunk_manager: ChunkManager,
    chunk_size: u32,
}

impl World {
    pub fn new(chunk_size: u32) -> Self {
        // Create a simple flat world generator for backwards compatibility
        let generator = Box::new(FlatWorldGenerator::new());
        let chunk_manager = ChunkManager::new(8, chunk_size, generator);
        
        Self {
            chunk_manager,
            chunk_size,
        }
    }
    
    pub fn new_with_generator(chunk_size: u32, view_distance: i32, generator: Box<dyn WorldGenerator>) -> Self {
        let chunk_manager = ChunkManager::new(view_distance, chunk_size, generator);
        
        Self {
            chunk_manager,
            chunk_size,
        }
    }
    
    pub fn update_loaded_chunks(&mut self, player_pos: Point3<f32>) {
        self.chunk_manager.update_loaded_chunks(player_pos);
    }
    
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunk_manager.get_chunk(pos)
    }
    
    pub fn get_chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        self.chunk_manager.get_chunk_mut(pos)
    }
    
    pub fn set_chunk(&mut self, pos: ChunkPos, chunk: Chunk) {
        // For backwards compatibility - directly insert into loaded chunks
        if let Some(existing) = self.chunk_manager.get_chunk_mut(pos) {
            *existing = chunk;
        }
    }
    
    pub fn get_block(&self, pos: VoxelPos) -> BlockId {
        self.chunk_manager.get_block(pos)
    }
    
    pub fn set_block(&mut self, pos: VoxelPos, block: BlockId) {
        self.chunk_manager.set_block(pos, block);
    }
    
    pub fn chunks(&self) -> &HashMap<ChunkPos, Chunk> {
        self.chunk_manager.get_loaded_chunks()
    }
    
    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }
    
    pub fn is_block_in_bounds(&self, pos: VoxelPos) -> bool {
        // In an infinite world, all positions are valid
        true
    }
    
    pub fn take_dirty_chunks(&mut self) -> std::collections::HashSet<ChunkPos> {
        self.chunk_manager.take_dirty_chunks()
    }
    
    // Lighting methods
    pub fn get_sky_light(&self, pos: VoxelPos) -> u8 {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.chunk_manager.get_chunk(chunk_pos) {
            chunk.get_sky_light(local_pos.0, local_pos.1, local_pos.2)
        } else {
            0
        }
    }
    
    pub fn set_sky_light(&mut self, pos: VoxelPos, level: u8) {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.chunk_manager.get_chunk_mut(chunk_pos) {
            chunk.set_sky_light(local_pos.0, local_pos.1, local_pos.2, level);
        }
    }
    
    pub fn get_block_light(&self, pos: VoxelPos) -> u8 {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.chunk_manager.get_chunk(chunk_pos) {
            chunk.get_block_light(local_pos.0, local_pos.1, local_pos.2)
        } else {
            0
        }
    }
    
    pub fn set_block_light(&mut self, pos: VoxelPos, level: u8) {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.chunk_manager.get_chunk_mut(chunk_pos) {
            chunk.set_block_light(local_pos.0, local_pos.1, local_pos.2, level);
        }
    }
    
    pub fn is_block_transparent(&self, pos: VoxelPos) -> bool {
        let block_id = self.get_block(pos);
        // For now, only air and water are transparent
        block_id == BlockId::AIR || block_id == BlockId(6) // Water
    }
    
    pub fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        self.chunk_manager.get_surface_height(world_x, world_z)
    }
}

// Simple flat world generator for backwards compatibility
struct FlatWorldGenerator;

impl FlatWorldGenerator {
    fn new() -> Self {
        Self
    }
}

impl WorldGenerator for FlatWorldGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> Chunk {
        let mut chunk = Chunk::new(chunk_pos, chunk_size);
        
        // Generate flat terrain at y=8
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                for y in 0..9 {
                    let block_id = if y < 8 {
                        BlockId(2) // Dirt
                    } else {
                        BlockId(1) // Grass
                    };
                    chunk.set_block(x, y, z, block_id);
                }
            }
        }
        
        chunk
    }
    
    fn get_surface_height(&self, _world_x: f64, _world_z: f64) -> i32 {
        8 // Fixed height for flat world
    }
}

// Implement WorldInterface for World
impl WorldInterface for World {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        // Delegate to chunk_manager
        self.chunk_manager.get_block(pos)
    }
    
    fn set_block(&mut self, pos: VoxelPos, block: BlockId) {
        // Delegate to chunk_manager
        self.chunk_manager.set_block(pos, block);
    }
    
    fn update_loaded_chunks(&mut self, player_pos: Point3<f32>) {
        // Delegate to chunk_manager
        self.chunk_manager.update_loaded_chunks(player_pos);
    }
    
    fn chunk_size(&self) -> u32 {
        // Return the stored chunk_size field
        self.chunk_size
    }
    
    fn is_block_in_bounds(&self, pos: VoxelPos) -> bool {
        // In an infinite world, all positions are valid
        true
    }
    
    fn get_sky_light(&self, pos: VoxelPos) -> u8 {
        // Call the struct's method implementation
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.chunk_manager.get_chunk(chunk_pos) {
            chunk.get_light(local_pos.0, local_pos.1, local_pos.2).sky
        } else {
            0
        }
    }
    
    fn set_sky_light(&mut self, pos: VoxelPos, level: u8) {
        // Call the struct's method implementation
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.chunk_manager.get_chunk_mut(chunk_pos) {
            let mut light = chunk.get_light(local_pos.0, local_pos.1, local_pos.2);
            light.sky = level;
            chunk.set_light(local_pos.0, local_pos.1, local_pos.2, light);
        }
    }
    
    fn get_block_light(&self, pos: VoxelPos) -> u8 {
        // Call the struct's method implementation
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.chunk_manager.get_chunk(chunk_pos) {
            chunk.get_light(local_pos.0, local_pos.1, local_pos.2).block
        } else {
            0
        }
    }
    
    fn set_block_light(&mut self, pos: VoxelPos, level: u8) {
        // Call the struct's method implementation
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.chunk_manager.get_chunk_mut(chunk_pos) {
            let mut light = chunk.get_light(local_pos.0, local_pos.1, local_pos.2);
            light.block = level;
            chunk.set_light(local_pos.0, local_pos.1, local_pos.2, light);
        }
    }
    
    fn is_chunk_loaded(&self, pos: ChunkPos) -> bool {
        self.chunk_manager.get_chunk(pos).is_some()
    }
    
    fn take_dirty_chunks(&mut self) -> HashSet<ChunkPos> {
        // Delegate to chunk_manager
        self.chunk_manager.take_dirty_chunks()
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        // Delegate to chunk_manager
        self.chunk_manager.get_surface_height(world_x, world_z)
    }
    
    fn is_block_transparent(&self, pos: VoxelPos) -> bool {
        // For now, only air is transparent
        let block = self.chunk_manager.get_block(pos);
        block == BlockId::AIR
    }
}