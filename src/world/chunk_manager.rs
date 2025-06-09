use std::collections::{HashMap, HashSet};
use cgmath::Point3;
use crate::{Chunk, ChunkPos, VoxelPos, BlockId};
use crate::lighting::SkylightCalculator;
use super::generation::WorldGenerator;

pub struct ChunkManager {
    loaded_chunks: HashMap<ChunkPos, Chunk>,
    view_distance: i32,
    chunk_size: u32,
    generator: Box<dyn WorldGenerator>,
    // Track which chunks need meshing
    dirty_chunks: HashSet<ChunkPos>,
    // Cache for recently unloaded chunks
    chunk_cache: HashMap<ChunkPos, Chunk>,
    cache_size: usize,
}

impl ChunkManager {
    pub fn new(view_distance: i32, chunk_size: u32, generator: Box<dyn WorldGenerator>) -> Self {
        Self {
            loaded_chunks: HashMap::new(),
            view_distance,
            chunk_size,
            generator,
            dirty_chunks: HashSet::new(),
            chunk_cache: HashMap::new(),
            cache_size: 64, // Cache up to 64 chunks
        }
    }
    
    pub fn update_loaded_chunks(&mut self, player_pos: Point3<f32>) {
        // Convert player position to chunk coordinates
        let player_chunk = ChunkPos::new(
            (player_pos.x / self.chunk_size as f32).floor() as i32,
            (player_pos.y / self.chunk_size as f32).floor() as i32,
            (player_pos.z / self.chunk_size as f32).floor() as i32,
        );
        
        // Calculate which chunks should be loaded
        let mut chunks_to_load = HashSet::new();
        for dx in -self.view_distance..=self.view_distance {
            for dy in -self.view_distance..=self.view_distance {
                for dz in -self.view_distance..=self.view_distance {
                    // Simple sphere check for view distance
                    let distance_sq = dx * dx + dy * dy + dz * dz;
                    if distance_sq <= self.view_distance * self.view_distance {
                        let chunk_pos = ChunkPos::new(
                            player_chunk.x + dx,
                            player_chunk.y + dy,
                            player_chunk.z + dz,
                        );
                        chunks_to_load.insert(chunk_pos);
                    }
                }
            }
        }
        
        // Unload chunks that are too far
        let mut chunks_to_unload = Vec::new();
        for &chunk_pos in self.loaded_chunks.keys() {
            if !chunks_to_load.contains(&chunk_pos) {
                chunks_to_unload.push(chunk_pos);
            }
        }
        
        // Unload chunks and add to cache
        for chunk_pos in chunks_to_unload {
            if let Some(chunk) = self.loaded_chunks.remove(&chunk_pos) {
                // Add to cache
                self.chunk_cache.insert(chunk_pos, chunk);
                
                // Trim cache if too large
                if self.chunk_cache.len() > self.cache_size {
                    // Remove oldest chunk (simple FIFO for now)
                    if let Some(&oldest) = self.chunk_cache.keys().next() {
                        self.chunk_cache.remove(&oldest);
                    }
                }
            }
        }
        
        // Load new chunks
        for chunk_pos in chunks_to_load {
            if !self.loaded_chunks.contains_key(&chunk_pos) {
                // Check cache first
                let chunk = if let Some(cached_chunk) = self.chunk_cache.remove(&chunk_pos) {
                    cached_chunk
                } else {
                    // Generate new chunk
                    self.generator.generate_chunk(chunk_pos, self.chunk_size)
                };
                
                self.loaded_chunks.insert(chunk_pos, chunk);
                self.dirty_chunks.insert(chunk_pos);
            }
        }
    }
    
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.loaded_chunks.get(&pos)
    }
    
    pub fn get_chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        if let Some(chunk) = self.loaded_chunks.get_mut(&pos) {
            self.dirty_chunks.insert(pos);
            Some(chunk)
        } else {
            None
        }
    }
    
    pub fn get_block(&self, pos: VoxelPos) -> BlockId {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.loaded_chunks.get(&chunk_pos) {
            chunk.get_block(local_pos.0, local_pos.1, local_pos.2)
        } else {
            BlockId::AIR
        }
    }
    
    pub fn set_block(&mut self, pos: VoxelPos, block: BlockId) {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.get_chunk_mut(chunk_pos) {
            chunk.set_block(local_pos.0, local_pos.1, local_pos.2, block);
            
            // Mark neighboring chunks as dirty if on edge
            if local_pos.0 == 0 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x - 1, chunk_pos.y, chunk_pos.z));
            }
            if local_pos.0 == self.chunk_size - 1 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x + 1, chunk_pos.y, chunk_pos.z));
            }
            if local_pos.1 == 0 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y - 1, chunk_pos.z));
            }
            if local_pos.1 == self.chunk_size - 1 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y + 1, chunk_pos.z));
            }
            if local_pos.2 == 0 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y, chunk_pos.z - 1));
            }
            if local_pos.2 == self.chunk_size - 1 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y, chunk_pos.z + 1));
            }
        }
    }
    
    pub fn get_loaded_chunks(&self) -> &HashMap<ChunkPos, Chunk> {
        &self.loaded_chunks
    }
    
    pub fn take_dirty_chunks(&mut self) -> HashSet<ChunkPos> {
        std::mem::take(&mut self.dirty_chunks)
    }
    
    pub fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        self.generator.get_surface_height(world_x, world_z)
    }
}