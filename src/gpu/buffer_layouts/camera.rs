//! Camera buffer layout definitions
//!
//! Defines GPU buffer structures for camera uniforms and culling data.

use crate::constants::buffer_layouts::*;
use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, SquareMatrix, Vector3};

/// Standard camera uniform buffer for rendering
/// Total size: 256 bytes (aligned to uniform buffer requirements)
///
/// This structure matches the layout expected by most shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    /// View matrix (world to camera space)
    pub view_matrix: [[f32; 4]; 4],

    /// Projection matrix (camera to clip space)
    pub projection_matrix: [[f32; 4]; 4],

    /// Combined view-projection matrix
    pub view_projection_matrix: [[f32; 4]; 4],

    /// Inverse view-projection matrix (for screen to world)
    pub inverse_view_projection: [[f32; 4]; 4],

    /// Camera world position
    pub position: [f32; 3],
    /// Time since start (seconds)
    pub time: f32,

    /// Camera forward direction (normalized)
    pub forward: [f32; 3],
    /// Delta time (seconds)
    pub delta_time: f32,

    /// Near and far plane distances
    pub near_far: [f32; 2],
    /// Screen dimensions (width, height)
    pub screen_size: [f32; 2],

    /// Additional padding to reach 256 bytes
    pub _padding: [f32; 8],
}

impl CameraUniform {
    /// Create a new camera uniform
    pub fn new(
        view: Matrix4<f32>,
        projection: Matrix4<f32>,
        position: Vector3<f32>,
        forward: Vector3<f32>,
        near: f32,
        far: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> Self {
        let view_proj = projection * view;
        let inverse_view_proj = view_proj.invert().unwrap_or(Matrix4::from_scale(1.0));

        Self {
            view_matrix: view.into(),
            projection_matrix: projection.into(),
            view_projection_matrix: view_proj.into(),
            inverse_view_projection: inverse_view_proj.into(),
            position: position.into(),
            time: 0.0,
            forward: forward.into(),
            delta_time: 0.0,
            near_far: [near, far],
            screen_size: [screen_width, screen_height],
            _padding: [0.0; 8],
        }
    }

    /// Update time values
    pub fn update_time(&mut self, time: f32, delta_time: f32) {
        self.time = time;
        self.delta_time = delta_time;
    }
}

/// Extended camera data for GPU culling
/// Total size: 256 bytes (aligned)
///
/// Includes frustum planes for efficient culling
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CullingCameraData {
    /// View-projection matrix
    pub view_proj: [[f32; 4]; 4],

    /// Camera world position
    pub position: [f32; 3],
    /// Padding for alignment
    pub _padding0: f32,

    /// Frustum planes for GPU culling
    /// Order: [left, right, bottom, top, near, far]
    /// Each plane: [a, b, c, d] where ax + by + cz + d = 0
    pub frustum_planes: [[f32; 4]; 6],

    /// Culling distances
    pub cull_distance_near: f32,
    pub cull_distance_far: f32,
    /// LOD transition distances
    pub lod_distance_0: f32,
    pub lod_distance_1: f32,

    /// Additional padding to reach 256 bytes
    pub _padding1: [f32; 8],
}

impl CullingCameraData {
    /// Create from standard camera uniform
    pub fn from_camera_uniform(uniform: &CameraUniform, cull_far: f32) -> Self {
        Self {
            view_proj: uniform.view_projection_matrix,
            position: uniform.position,
            _padding0: 0.0,
            frustum_planes: Self::calculate_frustum_planes(&uniform.view_projection_matrix),
            cull_distance_near: uniform.near_far[0],
            cull_distance_far: cull_far,
            lod_distance_0: 50.0, // Default LOD distances
            lod_distance_1: 100.0,
            _padding1: [0.0; 8],
        }
    }

    /// Calculate frustum planes from view-projection matrix
    fn calculate_frustum_planes(view_proj: &[[f32; 4]; 4]) -> [[f32; 4]; 6] {
        let m = view_proj;
        let mut planes = [[0.0f32; 4]; 6];

        // Left plane: m[3] + m[0]
        planes[0] = [
            m[3][0] + m[0][0],
            m[3][1] + m[0][1],
            m[3][2] + m[0][2],
            m[3][3] + m[0][3],
        ];

        // Right plane: m[3] - m[0]
        planes[1] = [
            m[3][0] - m[0][0],
            m[3][1] - m[0][1],
            m[3][2] - m[0][2],
            m[3][3] - m[0][3],
        ];

        // Bottom plane: m[3] + m[1]
        planes[2] = [
            m[3][0] + m[1][0],
            m[3][1] + m[1][1],
            m[3][2] + m[1][2],
            m[3][3] + m[1][3],
        ];

        // Top plane: m[3] - m[1]
        planes[3] = [
            m[3][0] - m[1][0],
            m[3][1] - m[1][1],
            m[3][2] - m[1][2],
            m[3][3] - m[1][3],
        ];

        // Near plane: m[3] + m[2]
        planes[4] = [
            m[3][0] + m[2][0],
            m[3][1] + m[2][1],
            m[3][2] + m[2][2],
            m[3][3] + m[2][3],
        ];

        // Far plane: m[3] - m[2]
        planes[5] = [
            m[3][0] - m[2][0],
            m[3][1] - m[2][1],
            m[3][2] - m[2][2],
            m[3][3] - m[2][3],
        ];

        // Normalize planes
        for plane in &mut planes {
            let len = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
            if len > 0.0 {
                plane[0] /= len;
                plane[1] /= len;
                plane[2] /= len;
                plane[3] /= len;
            }
        }

        planes
    }

    /// Set LOD transition distances
    pub fn with_lod_distances(mut self, lod0: f32, lod1: f32) -> Self {
        self.lod_distance_0 = lod0;
        self.lod_distance_1 = lod1;
        self
    }
}

/// Light camera data for shadow mapping
/// Total size: 128 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LightCameraData {
    /// Light view-projection matrix
    pub view_proj: [[f32; 4]; 4],

    /// Light position (point/spot) or direction (directional)
    pub position_or_direction: [f32; 3],
    /// Light type: 0 = directional, 1 = point, 2 = spot
    pub light_type: f32,

    /// Shadow map dimensions and bias
    pub shadow_map_size: f32,
    pub shadow_bias: f32,
    pub shadow_normal_bias: f32,
    pub _padding: f32,
}

/// Camera buffer layout information
pub struct CameraBufferLayout;

impl CameraBufferLayout {
    /// Standard camera uniform size
    pub const UNIFORM_SIZE: u64 = CAMERA_UNIFORM_SIZE;

    /// Culling camera data size
    pub const CULLING_SIZE: u64 = CULLING_CAMERA_SIZE;

    /// Light camera data size
    pub const LIGHT_SIZE: u64 = 128;
}
