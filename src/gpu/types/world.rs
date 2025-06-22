//! GPU type definitions for world data
//!
//! This module defines GPU-compatible types for world storage and chunk metadata,
//! ensuring consistency between CPU and GPU representations.

use crate::gpu::automation::auto_layout::AutoLayout;
use crate::gpu::automation::auto_wgsl::AutoWgsl;
use bytemuck::{Pod, Zeroable};
use encase::{ShaderSize, ShaderType};

/// Chunk metadata stored on GPU
/// Aligned to 16 bytes for efficient GPU access
///
/// This is the SINGLE SOURCE OF TRUTH for chunk metadata.
/// All systems must use this definition.
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug)]
pub struct ChunkMetadata {
    /// Packed flags and position data
    /// Bits 0-15: X position offset within world
    /// Bits 16-31: Z position offset within world
    pub flags: u32,

    /// Last modification timestamp (frame number)
    pub timestamp: u32,

    /// Checksum for validation
    pub checksum: u32,

    /// Y position (stored separately for alignment)
    pub y_position: u32,

    /// Slot index in WorldBuffer for this chunk
    pub slot_index: u32,

    /// Reserved for future use (maintains 16-byte alignment)
    pub _reserved: [u32; 3],
}

// Implement AutoWgsl for automatic WGSL generation
crate::auto_wgsl!(
    ChunkMetadata,
    name = "ChunkMetadata",
    fields = [
        flags: "u32",
        timestamp: "u32",
        checksum: "u32",
        y_position: "u32",
        slot_index: "u32",
        _reserved: "u32"[3],
    ]
);

// Implement AutoLayout for automatic memory layout
crate::impl_auto_layout!(
    ChunkMetadata,
    fields = [
        flags: u32 = "flags",
        timestamp: u32 = "timestamp",
        checksum: u32 = "checksum",
        y_position: u32 = "y_position",
        slot_index: u32 = "slot_index",
        _reserved: [u32; 3] = "_reserved"
    ]
);

impl ChunkMetadata {
    /// Create metadata for a chunk at the given position
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        // Pack X and Z into flags field
        let flags = ((x & 0xFFFF) as u32) << 16 | (z & 0xFFFF) as u32;

        Self {
            flags,
            timestamp: 0,
            checksum: 0,
            y_position: y as u32,
            slot_index: 0,
            _reserved: [0; 3],
        }
    }

    /// Extract X position from packed flags
    #[inline]
    pub fn x_position(&self) -> i32 {
        ((self.flags >> 16) & 0xFFFF) as i16 as i32
    }

    /// Extract Z position from packed flags
    #[inline]
    pub fn z_position(&self) -> i32 {
        (self.flags & 0xFFFF) as i16 as i32
    }

    /// Get Y position
    #[inline]
    pub fn y_position(&self) -> i32 {
        self.y_position as i32
    }

    /// Update timestamp
    #[inline]
    pub fn update_timestamp(&mut self, frame: u32) {
        self.timestamp = frame;
    }
}

impl Default for ChunkMetadata {
    fn default() -> Self {
        Self {
            flags: 0,
            timestamp: 0,
            checksum: 0,
            y_position: 0,
            slot_index: 0,
            _reserved: [0; 3],
        }
    }
}

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
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug)]
pub struct VoxelData {
    /// Packed voxel data
    pub data: u32,
}

// Implement AutoWgsl for VoxelData
crate::auto_wgsl!(
    VoxelData,
    name = "VoxelData",
    fields = [
        data: "u32",
    ]
);

// Implement AutoLayout for VoxelData
crate::impl_auto_layout!(
    VoxelData,
    fields = [
        data: u32 = "data"
    ]
);

impl VoxelData {
    /// Air block constant
    pub const AIR: Self = Self { data: 0 };

    /// Create a new voxel data entry
    #[inline]
    pub fn new(block_id: u16, light: u8, sky_light: u8, metadata: u8) -> Self {
        let packed = (block_id as u32)
            | ((light as u32 & 0xF) << 16)
            | ((sky_light as u32 & 0xF) << 20)
            | ((metadata as u32 & 0xF) << 24);
        Self { data: packed }
    }

    /// Extract block ID
    #[inline]
    pub fn block_id(&self) -> u16 {
        (self.data & 0xFFFF) as u16
    }

    /// Extract light level
    #[inline]
    pub fn light_level(&self) -> u8 {
        ((self.data >> 16) & 0xF) as u8
    }

    /// Extract sky light level
    #[inline]
    pub fn sky_light_level(&self) -> u8 {
        ((self.data >> 20) & 0xF) as u8
    }

    /// Extract metadata
    #[inline]
    pub fn metadata(&self) -> u8 {
        ((self.data >> 24) & 0xF) as u8
    }

    /// Check if this is an air block
    #[inline]
    pub fn is_air(&self) -> bool {
        self.block_id() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_metadata_size() {
        // Ensure ChunkMetadata is exactly 16 bytes
        assert_eq!(std::mem::size_of::<ChunkMetadata>(), 16);
    }

    #[test]
    fn test_voxel_data_size() {
        // Ensure VoxelData is exactly 4 bytes
        assert_eq!(std::mem::size_of::<VoxelData>(), 4);
    }
}
