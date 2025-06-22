//! Instance buffer layout definitions
//!
//! Defines GPU buffer structures for instanced rendering and culling.

use crate::buffer_layouts::*;
use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, Vector3};
use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

/// Per-instance data for GPU instanced rendering
/// Total size: 96 bytes (aligned)
///
/// Memory layout:
/// - Offset 0-63: Model matrix (4x4 floats)
/// - Offset 64-79: Color with alpha (4 floats)
/// - Offset 80-95: Custom data (4 floats)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceData {
    /// Model matrix (world transform)
    pub model_matrix: [[f32; 4]; 4],

    /// Color tint and alpha (RGBA)
    pub color: [f32; 4],

    /// Custom data for shader use
    /// Can store: texture indices, animation time, material properties, etc.
    pub custom_data: [f32; 4],
}

impl InstanceData {
    /// Create instance data with position and uniform scale
    pub fn new(position: Vector3<f32>, scale: f32, color: [f32; 4]) -> Self {
        let model_matrix = Matrix4::from_translation(position) * Matrix4::from_scale(scale);

        Self {
            model_matrix: model_matrix.into(),
            color,
            custom_data: [0.0; 4],
        }
    }

    /// Create instance data with full transform matrix
    pub fn from_matrix(model_matrix: Matrix4<f32>, color: [f32; 4]) -> Self {
        Self {
            model_matrix: model_matrix.into(),
            color,
            custom_data: [0.0; 4],
        }
    }

    /// Get the position from the model matrix
    #[inline]
    pub fn position(&self) -> Vector3<f32> {
        Vector3::new(
            self.model_matrix[3][0],
            self.model_matrix[3][1],
            self.model_matrix[3][2],
        )
    }
}

/// Compact instance data for GPU culling
/// Total size: 32 bytes (aligned)
///
/// Used in compute shaders for efficient frustum and occlusion culling
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CullingInstanceData {
    /// World position (xyz)
    pub position: [f32; 3],

    /// Bounding sphere radius
    pub radius: f32,

    /// Instance ID (index into full instance buffer)
    pub instance_id: u32,

    /// Flags: bit 0 = visible, bit 1 = cast shadows, bit 2-31 = reserved
    pub flags: u32,

    /// Padding for 16-byte alignment
    pub _padding: [u32; 2],
}

impl CullingInstanceData {
    /// Create culling data from full instance data
    pub fn from_instance(instance: &InstanceData, radius: f32, id: u32) -> Self {
        let position = [
            instance.model_matrix[3][0],
            instance.model_matrix[3][1],
            instance.model_matrix[3][2],
        ];

        Self {
            position,
            radius,
            instance_id: id,
            flags: 0x3, // visible | cast shadows
            _padding: [0; 2],
        }
    }

    /// Check if instance is visible
    #[inline]
    pub fn is_visible(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Check if instance casts shadows
    #[inline]
    pub fn casts_shadows(&self) -> bool {
        (self.flags & 2) != 0
    }
}

/// Instance buffer layout information
pub struct InstanceBufferLayout;

impl InstanceBufferLayout {
    /// Get vertex buffer layout for instance data
    pub fn vertex_layout() -> VertexBufferLayout<'static> {
        const ATTRIBUTES: &[VertexAttribute] = &[
            // Model matrix row 0
            VertexAttribute {
                offset: 0,
                shader_location: 5,
                format: VertexFormat::Float32x4,
            },
            // Model matrix row 1
            VertexAttribute {
                offset: 16,
                shader_location: 6,
                format: VertexFormat::Float32x4,
            },
            // Model matrix row 2
            VertexAttribute {
                offset: 32,
                shader_location: 7,
                format: VertexFormat::Float32x4,
            },
            // Model matrix row 3
            VertexAttribute {
                offset: 48,
                shader_location: 8,
                format: VertexFormat::Float32x4,
            },
            // Color
            VertexAttribute {
                offset: 64,
                shader_location: 9,
                format: VertexFormat::Float32x4,
            },
            // Custom data
            VertexAttribute {
                offset: 80,
                shader_location: 10,
                format: VertexFormat::Float32x4,
            },
        ];

        VertexBufferLayout {
            array_stride: INSTANCE_DATA_SIZE,
            step_mode: VertexStepMode::Instance,
            attributes: ATTRIBUTES,
        }
    }

    /// Calculate buffer size for a given capacity
    #[inline]
    pub fn buffer_size(capacity: u32) -> u64 {
        super::calculations::instance_buffer_size(capacity)
    }

    /// Calculate culling buffer size for a given capacity
    #[inline]
    pub fn culling_buffer_size(capacity: u32) -> u64 {
        capacity as u64 * CULLING_INSTANCE_SIZE
    }
}

/// Preset instance buffer capacities
pub mod presets {
    /// Small scenes (development/testing)
    pub const SMALL_CAPACITY: u32 = 10_000;

    /// Medium scenes (typical gameplay)
    pub const MEDIUM_CAPACITY: u32 = 50_000;

    /// Large scenes (open world)
    pub const LARGE_CAPACITY: u32 = 100_000;

    /// Massive scenes (stress testing)
    pub const MASSIVE_CAPACITY: u32 = 500_000;
}
