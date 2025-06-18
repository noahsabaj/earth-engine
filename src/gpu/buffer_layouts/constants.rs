//! Buffer size constants and calculations
//! 
//! Central location for all buffer-related constants to ensure consistency
//! across CPU and GPU code.

use crate::gpu::constants::core::{CHUNK_SIZE, VOXELS_PER_CHUNK};

// ===== Buffer Element Sizes =====

/// Size of a single voxel data element (u32)
pub const VOXEL_DATA_SIZE: u64 = 4;

/// Size of chunk metadata structure
pub const CHUNK_METADATA_SIZE: u64 = 16;

/// Size of instance data structure
pub const INSTANCE_DATA_SIZE: u64 = 96; // 4x4 matrix + 4 floats color + 4 floats custom

/// Size of culling instance data
pub const CULLING_INSTANCE_SIZE: u64 = 32; // 3 floats pos + 1 float radius + 2 u32 + 2 u32 padding

/// Size of indirect draw command
pub const INDIRECT_COMMAND_SIZE: u64 = 16; // 4 u32 values

/// Size of indirect indexed draw command  
pub const INDIRECT_INDEXED_COMMAND_SIZE: u64 = 20; // 5 u32 values

/// Size of draw metadata structure
pub const DRAW_METADATA_SIZE: u64 = 32; // 8 floats + 4 u32 values

/// Size of camera uniform buffer (aligned to 256)
pub const CAMERA_UNIFORM_SIZE: u64 = 256;

/// Size of culling camera data
pub const CULLING_CAMERA_SIZE: u64 = 256; // Includes frustum planes

// ===== Buffer Slot Sizes =====

/// Size of a single chunk slot in world buffer
pub const CHUNK_BUFFER_SLOT_SIZE: u64 = VOXELS_PER_CHUNK as u64 * VOXEL_DATA_SIZE;

/// Maximum chunks based on view distance
pub const MAX_CHUNKS_VIEW_DISTANCE_3: u32 = 343; // (2*3+1)³ = 7³
pub const MAX_CHUNKS_VIEW_DISTANCE_4: u32 = 729; // (2*4+1)³ = 9³
pub const MAX_CHUNKS_VIEW_DISTANCE_5: u32 = 1331; // (2*5+1)³ = 11³

// ===== Alignment Requirements =====

/// WGSL storage buffer alignment
pub const STORAGE_BUFFER_ALIGNMENT: u64 = 16;

/// WGSL uniform buffer alignment
pub const UNIFORM_BUFFER_ALIGNMENT: u64 = 256;

/// Vertex buffer optimal alignment
pub const VERTEX_BUFFER_ALIGNMENT: u64 = 4;

// ===== Buffer Limits =====

/// Maximum instance count per buffer
pub const MAX_INSTANCES_PER_BUFFER: u32 = 100_000;

/// Maximum indirect draws per pass
pub const MAX_INDIRECT_DRAWS: u32 = 10_000;

/// Maximum vertices per mesh
pub const MAX_VERTICES_PER_MESH: u32 = 65_536;

/// Maximum indices per mesh
pub const MAX_INDICES_PER_MESH: u32 = 98_304; // 65536 * 1.5

// ===== Memory Budget Constants =====

/// Target GPU memory usage for world data (MB)
pub const WORLD_BUFFER_MEMORY_BUDGET_MB: u32 = 512;

/// Target GPU memory usage for instance data (MB)
pub const INSTANCE_BUFFER_MEMORY_BUDGET_MB: u32 = 128;

/// Target GPU memory usage for mesh data (MB)
pub const MESH_BUFFER_MEMORY_BUDGET_MB: u32 = 256;

// ===== Helper Functions =====

/// Calculate the number of chunks that fit in a given memory budget
pub fn chunks_per_memory_budget(budget_mb: u32) -> u32 {
    let budget_bytes = (budget_mb as u64) * 1024 * 1024;
    let chunks = budget_bytes / CHUNK_BUFFER_SLOT_SIZE;
    chunks.min(u32::MAX as u64) as u32
}

/// Calculate memory requirement for a given view distance
pub fn memory_for_view_distance(view_distance: u32) -> u64 {
    let diameter = 2 * view_distance + 1;
    let max_chunks = diameter * diameter * diameter;
    max_chunks as u64 * CHUNK_BUFFER_SLOT_SIZE
}

/// Get recommended view distance for available GPU memory
pub fn recommended_view_distance(available_memory_mb: u32) -> u32 {
    match available_memory_mb {
        0..=128 => 2,      // Very limited memory
        129..=256 => 3,    // ~45MB for world data
        257..=512 => 4,    // ~95MB for world data  
        513..=1024 => 5,   // ~173MB for world data
        1025..=2048 => 6,  // ~283MB for world data
        _ => 7,            // ~427MB for world data
    }
}