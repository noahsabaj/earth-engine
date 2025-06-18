//! World buffer layout definitions
//! 
//! Defines the GPU buffer structures for world voxel data and chunk metadata.

use bytemuck::{Pod, Zeroable};
use crate::world::ChunkPos;
use super::constants::*;
use crate::gpu::constants::core::VOXELS_PER_CHUNK;

/// Packed voxel data format for GPU storage
/// Uses 32 bits per voxel for efficient memory usage
/// 
/// Memory layout:
/// - Bits 0-15: Block ID (supports 65,536 block types)
/// - Bits 16-19: Light level (0-15)
/// - Bits 20-23: Sky light level (0-15)
/// - Bits 24-27: Metadata (flags, rotation, etc)
/// - Bits 28-31: Reserved for future use
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VoxelData(pub u32);

impl VoxelData {
    /// Air block constant
    pub const AIR: Self = Self(0);
    
    /// Create a new voxel data entry
    #[inline]
    pub fn new(block_id: u16, light: u8, sky_light: u8, metadata: u8) -> Self {
        let packed = (block_id as u32) 
            | ((light as u32 & 0xF) << 16)
            | ((sky_light as u32 & 0xF) << 20)
            | ((metadata as u32 & 0xF) << 24);
        Self(packed)
    }
    
    /// Extract block ID
    #[inline]
    pub fn block_id(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
    
    /// Extract light level
    #[inline]
    pub fn light_level(&self) -> u8 {
        ((self.0 >> 16) & 0xF) as u8
    }
    
    /// Extract sky light level
    #[inline]
    pub fn sky_light_level(&self) -> u8 {
        ((self.0 >> 20) & 0xF) as u8
    }
    
    /// Extract metadata
    #[inline]
    pub fn metadata(&self) -> u8 {
        ((self.0 >> 24) & 0xF) as u8
    }
    
    /// Check if this is an air block
    #[inline]
    pub fn is_air(&self) -> bool {
        self.block_id() == 0
    }
    
    /// Set block ID while preserving other data
    #[inline]
    pub fn with_block_id(mut self, block_id: u16) -> Self {
        self.0 = (self.0 & 0xFFFF0000) | (block_id as u32);
        self
    }
    
    /// Set light level while preserving other data
    #[inline]
    pub fn with_light_level(mut self, light: u8) -> Self {
        self.0 = (self.0 & 0xFFF0FFFF) | ((light as u32 & 0xF) << 16);
        self
    }
}

/// Chunk metadata stored on GPU
/// Aligned to 16 bytes for efficient GPU access
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ChunkMetadata {
    /// Flags: bit 0 = loaded, bit 1 = modified, bit 2-31 = reserved
    pub flags: u32,
    
    /// Last modification timestamp (frame number)
    pub last_modified: u32,
    
    /// LOD level (0 = full detail, higher = less detail)
    pub lod_level: u32,
    
    /// Reserved for future use (maintains 16-byte alignment)
    pub reserved: u32,
}

impl ChunkMetadata {
    /// Create new metadata for a fresh chunk
    pub fn new() -> Self {
        Self {
            flags: 0,
            last_modified: 0,
            lod_level: 0,
            reserved: 0,
        }
    }
    
    /// Check if chunk is loaded
    #[inline]
    pub fn is_loaded(&self) -> bool {
        (self.flags & 1) != 0
    }
    
    /// Set loaded flag
    #[inline]
    pub fn set_loaded(&mut self, loaded: bool) {
        if loaded {
            self.flags |= 1;
        } else {
            self.flags &= !1;
        }
    }
    
    /// Check if chunk is modified
    #[inline]
    pub fn is_modified(&self) -> bool {
        (self.flags & 2) != 0
    }
    
    /// Set modified flag
    #[inline]
    pub fn set_modified(&mut self, modified: bool) {
        if modified {
            self.flags |= 2;
        } else {
            self.flags &= !2;
        }
    }
}

/// World buffer layout information
pub struct WorldBufferLayout {
    /// Maximum number of chunks
    pub max_chunks: u32,
    
    /// View distance
    pub view_distance: u32,
    
    /// Total voxel count
    pub total_voxels: u64,
    
    /// Voxel buffer size in bytes
    pub voxel_buffer_size: u64,
    
    /// Metadata buffer size in bytes
    pub metadata_buffer_size: u64,
}

impl WorldBufferLayout {
    /// Create layout for a given view distance
    pub fn new(view_distance: u32) -> Self {
        let diameter = 2 * view_distance + 1;
        let max_chunks = diameter * diameter * diameter;
        let total_voxels = max_chunks as u64 * VOXELS_PER_CHUNK as u64;
        
        Self {
            max_chunks,
            view_distance,
            total_voxels,
            voxel_buffer_size: total_voxels * VOXEL_DATA_SIZE,
            metadata_buffer_size: max_chunks as u64 * CHUNK_METADATA_SIZE,
        }
    }
    
    /// Calculate slot offset for a chunk
    #[inline]
    pub fn chunk_offset(&self, slot: u32) -> u64 {
        super::calculations::chunk_slot_offset(slot)
    }
    
    /// Get memory usage in MB
    pub fn memory_usage_mb(&self) -> f32 {
        let total_bytes = self.voxel_buffer_size + self.metadata_buffer_size;
        total_bytes as f32 / (1024.0 * 1024.0)
    }
}