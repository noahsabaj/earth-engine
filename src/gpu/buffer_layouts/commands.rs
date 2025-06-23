//! Indirect command buffer layout definitions
//!
//! Defines GPU buffer structures for indirect drawing and compute dispatch.

use crate::constants::buffer_layouts::*;
use bytemuck::{Pod, Zeroable};

/// GPU indirect draw command structure
/// Matches wgpu's DrawIndirect command layout exactly
/// Total size: 16 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct IndirectDrawCommand {
    /// Number of vertices to draw
    pub vertex_count: u32,

    /// Number of instances to draw
    pub instance_count: u32,

    /// Offset into the vertex buffer
    pub first_vertex: u32,

    /// Offset into the instance buffer
    pub first_instance: u32,
}

impl IndirectDrawCommand {
    /// Create a new draw command
    pub fn new(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }

    /// Create a draw command with offsets
    pub fn with_offsets(
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        }
    }
}

/// GPU indirect draw indexed command structure
/// Matches wgpu's DrawIndexedIndirect command layout exactly
/// Total size: 20 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct IndirectDrawIndexedCommand {
    /// Number of indices to draw
    pub index_count: u32,

    /// Number of instances to draw
    pub instance_count: u32,

    /// Offset into the index buffer
    pub first_index: u32,

    /// Value added to each index before fetching vertex
    pub base_vertex: i32,

    /// Offset into the instance buffer
    pub first_instance: u32,
}

impl IndirectDrawIndexedCommand {
    /// Create a new indexed draw command
    pub fn new(index_count: u32, instance_count: u32) -> Self {
        Self {
            index_count,
            instance_count,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        }
    }

    /// Create an indexed draw command with offsets
    pub fn with_offsets(
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        base_vertex: i32,
        first_instance: u32,
    ) -> Self {
        Self {
            index_count,
            instance_count,
            first_index,
            base_vertex,
            first_instance,
        }
    }
}

/// Metadata for draw commands used by GPU culling
/// Total size: 32 bytes (aligned)
///
/// This structure is used by compute shaders to make culling decisions
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DrawMetadata {
    /// World-space bounding sphere (xyz = center, w = radius)
    pub bounding_sphere: [f32; 4],

    /// LOD information:
    /// - x: minimum view distance for this LOD
    /// - y: maximum view distance for this LOD
    /// - z: LOD level (0 = highest detail)
    /// - w: reserved
    pub lod_info: [f32; 4],

    /// Material ID for sorting and batching
    pub material_id: u32,

    /// Mesh ID for buffer lookups
    pub mesh_id: u32,

    /// Starting offset in instance buffer
    pub instance_offset: u32,

    /// Flags:
    /// - bit 0: is visible
    /// - bit 1: cast shadows
    /// - bit 2: receive shadows
    /// - bit 3: is transparent
    /// - bit 4-31: reserved
    pub flags: u32,
}

impl DrawMetadata {
    /// Flag constants
    pub const FLAG_VISIBLE: u32 = 1 << 0;
    pub const FLAG_CAST_SHADOWS: u32 = 1 << 1;
    pub const FLAG_RECEIVE_SHADOWS: u32 = 1 << 2;
    pub const FLAG_TRANSPARENT: u32 = 1 << 3;

    /// Create new draw metadata
    pub fn new(center: [f32; 3], radius: f32, material_id: u32, mesh_id: u32) -> Self {
        Self {
            bounding_sphere: [center[0], center[1], center[2], radius],
            lod_info: [0.0, f32::MAX, 0.0, 0.0],
            material_id,
            mesh_id,
            instance_offset: 0,
            flags: Self::FLAG_VISIBLE | Self::FLAG_CAST_SHADOWS | Self::FLAG_RECEIVE_SHADOWS,
        }
    }

    /// Check if drawable
    #[inline]
    pub fn is_visible(&self) -> bool {
        (self.flags & Self::FLAG_VISIBLE) != 0
    }

    /// Check if casts shadows
    #[inline]
    pub fn casts_shadows(&self) -> bool {
        (self.flags & Self::FLAG_CAST_SHADOWS) != 0
    }

    /// Check if transparent
    #[inline]
    pub fn is_transparent(&self) -> bool {
        (self.flags & Self::FLAG_TRANSPARENT) != 0
    }

    /// Set LOD range
    pub fn with_lod_range(mut self, min_distance: f32, max_distance: f32, lod_level: u32) -> Self {
        self.lod_info = [min_distance, max_distance, lod_level as f32, 0.0];
        self
    }
}

/// GPU dispatch indirect command for compute shaders
/// Total size: 12 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct DispatchIndirectCommand {
    /// Number of workgroups in X dimension
    pub workgroups_x: u32,

    /// Number of workgroups in Y dimension
    pub workgroups_y: u32,

    /// Number of workgroups in Z dimension
    pub workgroups_z: u32,
}

impl DispatchIndirectCommand {
    /// Create a new dispatch command
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            workgroups_x: x,
            workgroups_y: y,
            workgroups_z: z,
        }
    }

    /// Create a 1D dispatch
    pub fn dispatch_1d(workgroups: u32) -> Self {
        Self::new(workgroups, 1, 1)
    }

    /// Create a 2D dispatch
    pub fn dispatch_2d(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }
}

/// Command buffer layout information
pub struct CommandBufferLayout;

impl CommandBufferLayout {
    /// Calculate buffer size for draw commands
    #[inline]
    pub fn draw_buffer_size(capacity: u32) -> u64 {
        capacity as u64 * INDIRECT_COMMAND_SIZE
    }

    /// Calculate buffer size for indexed draw commands
    #[inline]
    pub fn indexed_buffer_size(capacity: u32) -> u64 {
        capacity as u64 * INDIRECT_INDEXED_COMMAND_SIZE
    }

    /// Calculate buffer size for dispatch commands
    #[inline]
    pub fn dispatch_buffer_size(capacity: u32) -> u64 {
        capacity as u64 * std::mem::size_of::<DispatchIndirectCommand>() as u64
    }

    /// Calculate buffer size for draw metadata
    #[inline]
    pub fn metadata_buffer_size(capacity: u32) -> u64 {
        capacity as u64 * DRAW_METADATA_SIZE
    }
}
