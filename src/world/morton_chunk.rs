use crate::world::{BlockId, ChunkPos, VoxelPos};
use crate::lighting::{LightMap, LightLevel};
use crate::morton::{morton_encode_chunk, morton_decode_chunk};
use serde::{Serialize, Deserialize};

/// A chunk of voxels using Morton encoding for cache-friendly access
/// 
/// Morton encoding provides 3-5x better cache performance by ensuring
/// spatially close voxels are also close in memory.
pub struct MortonChunk {
    position: ChunkPos,
    size: u32,
    /// Blocks stored in Morton order for better cache locality
    blocks: Vec<BlockId>,
    /// Light map still uses linear indexing (will optimize later)
    light_map: LightMap,
    dirty: bool,
    light_dirty: bool,
}

impl MortonChunk {
    pub fn new(position: ChunkPos, size: u32) -> Self {
        let total_blocks = (size * size * size) as usize;
        Self {
            position,
            size,
            blocks: vec![BlockId::AIR; total_blocks],
            light_map: LightMap::new(size),
            dirty: true,
            light_dirty: true,
        }
    }
    
    /// Convert linear chunk to Morton-ordered chunk
    pub fn from_linear_chunk(chunk: &super::Chunk) -> Self {
        let mut morton_chunk = Self::new(chunk.position(), chunk.size());
        
        // Copy blocks in Morton order
        for x in 0..chunk.size() {
            for y in 0..chunk.size() {
                for z in 0..chunk.size() {
                    let block = chunk.get_block(x, y, z);
                    morton_chunk.set_block(x, y, z, block);
                }
            }
        }
        
        morton_chunk
    }
    
    /// Get the chunk position
    pub fn position(&self) -> ChunkPos {
        self.position
    }
    
    /// Get block at local position using Morton encoding
    pub fn get_block(&self, x: u32, y: u32, z: u32) -> BlockId {
        if x >= self.size || y >= self.size || z >= self.size {
            return BlockId::AIR;
        }
        let index = self.get_morton_index(x, y, z);
        self.blocks[index]
    }
    
    /// Get block using VoxelPos (assumes local coordinates)
    pub fn get_block_at(&self, pos: VoxelPos) -> BlockId {
        self.get_block(pos.x as u32, pos.y as u32, pos.z as u32)
    }
    
    /// Set block at local position using Morton encoding
    pub fn set_block(&mut self, x: u32, y: u32, z: u32, block: BlockId) {
        if x >= self.size || y >= self.size || z >= self.size {
            return;
        }
        let index = self.get_morton_index(x, y, z);
        self.blocks[index] = block;
        self.dirty = true;
    }
    
    /// Set block using VoxelPos (assumes local coordinates)
    pub fn set_block_at(&mut self, pos: VoxelPos, block: BlockId) {
        self.set_block(pos.x as u32, pos.y as u32, pos.z as u32, block);
    }
    
    /// Check if chunk needs remeshing
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Mark chunk as clean (mesh updated)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
    
    /// Mark chunk as dirty (needs remeshing)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    
    /// Clear dirty flag (alias for mark_clean)
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
    
    /// Get all blocks for iteration (in Morton order)
    pub fn blocks(&self) -> &[BlockId] {
        &self.blocks
    }
    
    /// Get blocks in linear order (for compatibility)
    pub fn blocks_linear(&self) -> Vec<BlockId> {
        let mut linear = vec![BlockId::AIR; self.blocks.len()];
        
        for morton_idx in 0..self.blocks.len() {
            let pos = morton_decode_chunk(morton_idx as u32);
            let linear_idx = (pos.x + pos.y * self.size as i32 + pos.z * self.size as i32 * self.size as i32) as usize;
            linear[linear_idx] = self.blocks[morton_idx];
        }
        
        linear
    }
    
    pub fn size(&self) -> u32 {
        self.size
    }
    
    /// Get Morton-encoded index for coordinates
    #[inline(always)]
    fn get_morton_index(&self, x: u32, y: u32, z: u32) -> usize {
        let pos = VoxelPos {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        };
        morton_encode_chunk(pos) as usize
    }
    
    /// Iterator for efficient neighbor access
    pub fn iter_neighbors(&self, x: u32, y: u32, z: u32) -> NeighborIterator {
        NeighborIterator::new(self, x, y, z)
    }
    
    // Light methods (still using linear indexing for now)
    pub fn get_light(&self, x: u32, y: u32, z: u32) -> LightLevel {
        self.light_map.get_light(x, y, z)
    }
    
    pub fn set_light(&mut self, x: u32, y: u32, z: u32, light: LightLevel) {
        self.light_map.set_light(x, y, z, light);
        self.light_dirty = true;
        self.dirty = true;
    }
    
    pub fn get_sky_light(&self, x: u32, y: u32, z: u32) -> u8 {
        self.light_map.get_light(x, y, z).sky
    }
    
    pub fn set_sky_light(&mut self, x: u32, y: u32, z: u32, level: u8) {
        self.light_map.set_sky_light(x, y, z, level);
        self.light_dirty = true;
        self.dirty = true;
    }
    
    pub fn get_block_light(&self, x: u32, y: u32, z: u32) -> u8 {
        self.light_map.get_light(x, y, z).block
    }
    
    pub fn set_block_light(&mut self, x: u32, y: u32, z: u32, level: u8) {
        self.light_map.set_block_light(x, y, z, level);
        self.light_dirty = true;
        self.dirty = true;
    }
    
    pub fn is_light_dirty(&self) -> bool {
        self.light_dirty
    }
    
    pub fn mark_light_clean(&mut self) {
        self.light_dirty = false;
    }
}

/// Iterator for accessing neighbors efficiently in Morton order
pub struct NeighborIterator<'a> {
    chunk: &'a MortonChunk,
    center_x: u32,
    center_y: u32,
    center_z: u32,
    index: usize,
}

impl<'a> NeighborIterator<'a> {
    fn new(chunk: &'a MortonChunk, x: u32, y: u32, z: u32) -> Self {
        Self {
            chunk,
            center_x: x,
            center_y: y,
            center_z: z,
            index: 0,
        }
    }
}

impl<'a> Iterator for NeighborIterator<'a> {
    type Item = (i32, i32, i32, BlockId);
    
    fn next(&mut self) -> Option<Self::Item> {
        const OFFSETS: [(i32, i32, i32); 27] = [
            // Center
            (0, 0, 0),
            // Face neighbors
            (-1, 0, 0), (1, 0, 0),
            (0, -1, 0), (0, 1, 0),
            (0, 0, -1), (0, 0, 1),
            // Edge neighbors
            (-1, -1, 0), (-1, 1, 0), (1, -1, 0), (1, 1, 0),
            (-1, 0, -1), (-1, 0, 1), (1, 0, -1), (1, 0, 1),
            (0, -1, -1), (0, -1, 1), (0, 1, -1), (0, 1, 1),
            // Corner neighbors
            (-1, -1, -1), (-1, -1, 1), (-1, 1, -1), (-1, 1, 1),
            (1, -1, -1), (1, -1, 1), (1, 1, -1), (1, 1, 1),
        ];
        
        while self.index < OFFSETS.len() {
            let (dx, dy, dz) = OFFSETS[self.index];
            self.index += 1;
            
            let nx = self.center_x as i32 + dx;
            let ny = self.center_y as i32 + dy;
            let nz = self.center_z as i32 + dz;
            
            if nx >= 0 && ny >= 0 && nz >= 0 && 
               nx < self.chunk.size as i32 && 
               ny < self.chunk.size as i32 && 
               nz < self.chunk.size as i32 {
                let block = self.chunk.get_block(nx as u32, ny as u32, nz as u32);
                return Some((dx, dy, dz, block));
            }
        }
        
        None
    }
}