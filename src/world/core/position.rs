use cgmath::Vector3;
use serde::{Deserialize, Serialize};

// Include constants from root constants.rs
include!("../../../constants.rs");

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

    /// Create ChunkPos from world position coordinates
    pub fn from_world_pos(world_x: i32, world_z: i32) -> Self {
        Self::new(
            world_x.div_euclid(core::CHUNK_SIZE as i32),
            0,
            world_z.div_euclid(core::CHUNK_SIZE as i32),
        )
    }

    /// Create ChunkPos from VoxelPos
    pub fn from_voxel_pos(voxel_pos: VoxelPos) -> Self {
        Self::new(
            voxel_pos.x.div_euclid(core::CHUNK_SIZE as i32),
            voxel_pos.y.div_euclid(core::CHUNK_SIZE as i32),
            voxel_pos.z.div_euclid(core::CHUNK_SIZE as i32),
        )
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

    /// Get chunk offset (same as to_local_pos but returns VoxelPos)
    pub fn to_chunk_offset(&self, chunk_size: u32) -> VoxelPos {
        let (x, y, z) = self.to_local_pos(chunk_size);
        VoxelPos::new(x as i32, y as i32, z as i32)
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
