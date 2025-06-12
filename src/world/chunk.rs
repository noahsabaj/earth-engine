use crate::world::BlockId;
use crate::lighting::{LightMap, LightLevel};
use cgmath::Vector3;
use serde::{Serialize, Deserialize};

/// Position of a chunk in the world (chunk coordinates)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    
    /// Convert to world position (multiply by chunk size)
    pub fn to_world_pos(&self, chunk_size: u32) -> Vector3<f32> {
        Vector3::new(
            (self.x * chunk_size as i32) as f32,
            (self.y * chunk_size as i32) as f32,
            (self.z * chunk_size as i32) as f32,
        )
    }
    
    /// Create a new chunk position offset by the given amounts
    pub fn offset(&self, dx: i32, dy: i32, dz: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.z + dz)
    }
    
    /// Calculate squared distance to another chunk position
    pub fn distance_squared_to(&self, other: ChunkPos) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }
}

/// Position of a voxel in the world (world coordinates)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VoxelPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl VoxelPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    
    /// Get the chunk this voxel belongs to
    pub fn to_chunk_pos(&self, chunk_size: u32) -> ChunkPos {
        let size = chunk_size as i32;
        ChunkPos::new(
            self.x.div_euclid(size),
            self.y.div_euclid(size),
            self.z.div_euclid(size),
        )
    }
    
    /// Get local position within chunk
    pub fn to_local_pos(&self, chunk_size: u32) -> (u32, u32, u32) {
        let size = chunk_size as i32;
        (
            self.x.rem_euclid(size) as u32,
            self.y.rem_euclid(size) as u32,
            self.z.rem_euclid(size) as u32,
        )
    }
    
    /// Create VoxelPos from world position (glam Vec3)
    pub fn from_world_pos(pos: glam::Vec3) -> Self {
        Self {
            x: pos.x.floor() as i32,
            y: pos.y.floor() as i32,
            z: pos.z.floor() as i32,
        }
    }
}

/// A chunk of voxels
#[derive(Clone)]
pub struct Chunk {
    position: ChunkPos,
    size: u32,
    blocks: Vec<BlockId>,
    light_map: LightMap,
    dirty: bool,
    light_dirty: bool,
}

impl Chunk {
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
    
    /// Get the chunk position
    pub fn position(&self) -> ChunkPos {
        self.position
    }
    
    /// Get block at local position
    pub fn get_block(&self, x: u32, y: u32, z: u32) -> BlockId {
        if x >= self.size || y >= self.size || z >= self.size {
            return BlockId::AIR;
        }
        let index = self.get_index(x, y, z);
        // Safety: index is guaranteed to be in bounds by get_index and the bounds check above
        self.blocks.get(index).copied().unwrap_or(BlockId::AIR)
    }
    
    /// Get block using VoxelPos (assumes local coordinates)
    pub fn get_block_at(&self, pos: VoxelPos) -> BlockId {
        self.get_block(pos.x as u32, pos.y as u32, pos.z as u32)
    }
    
    /// Set block at local position
    pub fn set_block(&mut self, x: u32, y: u32, z: u32, block: BlockId) {
        if x >= self.size || y >= self.size || z >= self.size {
            return;
        }
        let index = self.get_index(x, y, z);
        // Safety: index is guaranteed to be in bounds by get_index and the bounds check above
        if let Some(block_ref) = self.blocks.get_mut(index) {
            *block_ref = block;
            self.dirty = true;
        }
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
    
    /// Get all blocks for iteration
    pub fn blocks(&self) -> &[BlockId] {
        &self.blocks
    }
    
    pub fn size(&self) -> u32 {
        self.size
    }
    
    fn get_index(&self, x: u32, y: u32, z: u32) -> usize {
        debug_assert!(x < self.size && y < self.size && z < self.size,
                     "get_index called with out-of-bounds coordinates: ({}, {}, {}) for size {}",
                     x, y, z, self.size);
        (x + y * self.size + z * self.size * self.size) as usize
    }
    
    /// Get light level at local position
    pub fn get_light(&self, x: u32, y: u32, z: u32) -> LightLevel {
        self.light_map.get_light(x, y, z)
    }
    
    /// Set light level at local position
    pub fn set_light(&mut self, x: u32, y: u32, z: u32, light: LightLevel) {
        self.light_map.set_light(x, y, z, light);
        self.light_dirty = true;
        self.dirty = true;
    }
    
    /// Get sky light level
    pub fn get_sky_light(&self, x: u32, y: u32, z: u32) -> u8 {
        self.light_map.get_light(x, y, z).sky
    }
    
    /// Set sky light level
    pub fn set_sky_light(&mut self, x: u32, y: u32, z: u32, level: u8) {
        self.light_map.set_sky_light(x, y, z, level);
        self.light_dirty = true;
        self.dirty = true;
    }
    
    /// Get block light level
    pub fn get_block_light(&self, x: u32, y: u32, z: u32) -> u8 {
        self.light_map.get_light(x, y, z).block
    }
    
    /// Set block light level
    pub fn set_block_light(&mut self, x: u32, y: u32, z: u32, level: u8) {
        self.light_map.set_block_light(x, y, z, level);
        self.light_dirty = true;
        self.dirty = true;
    }
    
    /// Check if lighting needs update
    pub fn is_light_dirty(&self) -> bool {
        self.light_dirty
    }
    
    /// Mark lighting as clean
    pub fn mark_light_clean(&mut self) {
        self.light_dirty = false;
    }
}