//! Centralized GPU buffer layout definitions
//!
//! This module provides a single source of truth for all GPU buffer layouts,
//! sizes, offsets, and binding indices used throughout the engine.

// Constants are now in the root constants.rs file
pub mod camera;
pub mod commands;
pub mod compute;
pub mod instance;
pub mod mesh;
pub mod terrain;
pub mod world;

#[cfg(test)]
mod tests;

// Re-export commonly used items
pub use camera::{CameraUniform, CullingCameraData};
pub use commands::{DrawMetadata, IndirectDrawCommand, IndirectDrawIndexedCommand};
pub use crate::buffer_layouts::*;
pub use instance::{CullingInstanceData, InstanceBufferLayout, InstanceData};
pub use mesh::{Vertex, VertexSOA};
pub use terrain::{BlockDistribution, TerrainParams, TerrainParamsSOA};
pub use world::{ChunkMetadata, VoxelData, WorldBufferLayout};

/// Buffer binding indices for consistency across shaders
pub mod bindings {
    /// World buffer bindings
    pub mod world {
        pub const VOXEL_BUFFER: u32 = 0;
        pub const METADATA_BUFFER: u32 = 1;
        pub const PARAMS_BUFFER: u32 = 2;
    }

    /// Rendering bindings
    pub mod render {
        pub const CAMERA_UNIFORM: u32 = 0;
        pub const INSTANCE_BUFFER: u32 = 1;
        pub const MATERIAL_BUFFER: u32 = 2;
        pub const LIGHT_BUFFER: u32 = 3;
    }

    /// Compute shader bindings
    pub mod compute {
        pub const INPUT_BUFFER: u32 = 0;
        pub const OUTPUT_BUFFER: u32 = 1;
        pub const PARAMS_BUFFER: u32 = 2;
        pub const ATOMIC_COUNTER: u32 = 3;
    }

    /// Culling pipeline bindings
    pub mod culling {
        pub const CAMERA_DATA: u32 = 0;
        pub const DRAW_METADATA: u32 = 1; // draw_metadata array
        pub const DRAW_COMMANDS: u32 = 2; // indirect_commands array
        pub const DRAW_COUNT: u32 = 3; // draw_count atomic
        pub const STATS_BUFFER: u32 = 4; // culling_stats
    }
}

/// Buffer group indices for bind group organization
pub mod groups {
    pub const CAMERA_GROUP: u32 = 0;
    pub const MATERIAL_GROUP: u32 = 1;
    pub const WORLD_DATA_GROUP: u32 = 2;
    pub const COMPUTE_GROUP: u32 = 3;
}

/// Helper functions for buffer calculations
pub mod calculations {
    use crate::buffer_layouts::*;

    /// Calculate offset for a chunk slot in the world buffer
    #[inline]
    pub fn chunk_slot_offset(slot: u32) -> u64 {
        slot as u64 * CHUNK_BUFFER_SLOT_SIZE
    }

    /// Calculate total size for instance buffer
    #[inline]
    pub fn instance_buffer_size(capacity: u32) -> u64 {
        capacity as u64 * INSTANCE_DATA_SIZE
    }

    /// Calculate offset for an instance in the buffer
    #[inline]
    pub fn instance_offset(index: u32) -> u64 {
        index as u64 * INSTANCE_DATA_SIZE
    }

    /// Calculate size for indirect command buffer
    #[inline]
    pub fn indirect_buffer_size(capacity: u32, indexed: bool) -> u64 {
        let command_size = if indexed {
            INDIRECT_INDEXED_COMMAND_SIZE
        } else {
            INDIRECT_COMMAND_SIZE
        };
        capacity as u64 * command_size
    }

    /// Align size to GPU requirements
    #[inline]
    pub fn align_buffer_size(size: u64, alignment: u64) -> u64 {
        (size + alignment - 1) & !(alignment - 1)
    }
}

/// Buffer usage patterns for optimization
pub mod usage {
    use wgpu::BufferUsages;

    /// Standard storage buffer usage
    pub const STORAGE: BufferUsages = BufferUsages::STORAGE.union(BufferUsages::COPY_DST);

    /// Storage buffer with readback capability
    pub const STORAGE_READ: BufferUsages = STORAGE.union(BufferUsages::COPY_SRC);

    /// Vertex buffer usage
    pub const VERTEX: BufferUsages = BufferUsages::VERTEX.union(BufferUsages::COPY_DST);

    /// Index buffer usage
    pub const INDEX: BufferUsages = BufferUsages::INDEX.union(BufferUsages::COPY_DST);

    /// Uniform buffer usage
    pub const UNIFORM: BufferUsages = BufferUsages::UNIFORM.union(BufferUsages::COPY_DST);

    /// Indirect drawing buffer usage
    pub const INDIRECT: BufferUsages = BufferUsages::INDIRECT
        .union(BufferUsages::STORAGE)
        .union(BufferUsages::COPY_DST);
}

/// Bind group layout descriptors
pub mod layouts {
    use wgpu::{
        BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType,
        ShaderStages,
    };

    /// Create a storage buffer binding entry
    pub fn storage_buffer_entry(
        binding: u32,
        read_only: bool,
        visibility: ShaderStages,
    ) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    /// Create a uniform buffer binding entry
    pub fn uniform_buffer_entry(binding: u32, visibility: ShaderStages) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
}
