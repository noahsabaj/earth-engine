//! World buffer layout definitions
//!
//! This module re-exports world GPU types from the unified type system.

use crate::buffer_layouts::*;
use crate::core::VOXELS_PER_CHUNK;
use crate::ChunkPos;

// Re-export world types from the unified GPU type system
pub use crate::gpu::types::world::{ChunkMetadata, VoxelData};

/// World buffer layout information
pub struct WorldBufferLayout {
    /// Maximum number of chunks
    pub max_chunks: u32,

    /// View distance
    pub view_distance: u32,

    /// Bytes per chunk (voxel data)
    pub bytes_per_chunk: u64,

    /// Total buffer size for voxel data
    pub voxel_buffer_size: u64,

    /// Total buffer size for metadata
    pub metadata_buffer_size: u64,
}

impl WorldBufferLayout {
    /// Create a new world buffer layout
    pub fn new(view_distance: u32) -> Self {
        // Calculate max chunks based on view distance
        // Using a cubic region around the player
        let chunks_per_axis = (view_distance * 2 + 1) as u32;
        let max_chunks = chunks_per_axis * chunks_per_axis * chunks_per_axis;

        // Calculate buffer sizes
        let bytes_per_chunk = (VOXELS_PER_CHUNK * std::mem::size_of::<VoxelData>() as u32) as u64;
        let voxel_buffer_size = bytes_per_chunk * max_chunks as u64;
        let metadata_buffer_size =
            (max_chunks * std::mem::size_of::<ChunkMetadata>() as u32) as u64;

        Self {
            max_chunks,
            view_distance,
            bytes_per_chunk,
            voxel_buffer_size,
            metadata_buffer_size,
        }
    }

    /// Get the byte offset for a chunk in the voxel buffer
    pub fn chunk_offset(&self, chunk_index: u32) -> u64 {
        chunk_index as u64 * self.bytes_per_chunk
    }
}

/// Binding indices for world buffers
pub mod bindings {
    /// Voxel data buffer binding
    pub const VOXEL_BUFFER: u32 = 0;

    /// Metadata buffer binding
    pub const METADATA_BUFFER: u32 = 1;

    /// Parameters buffer binding (for terrain generation)
    pub const PARAMS_BUFFER: u32 = 2;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_buffer_layout() {
        let layout = WorldBufferLayout::new(2);

        // View distance 2 = 5x5x5 chunks
        assert_eq!(layout.max_chunks, 125);

        // Each chunk has VOXELS_PER_CHUNK voxels, 4 bytes each
        assert_eq!(layout.bytes_per_chunk, VOXELS_PER_CHUNK as u64 * 4);

        // Metadata is 16 bytes per chunk
        assert_eq!(layout.metadata_buffer_size, 125 * 16);
    }
}
