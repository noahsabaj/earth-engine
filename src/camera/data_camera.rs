/// Data-Oriented Camera System
/// 
/// Sprint 35: Pure data structures with free functions.
/// No methods, no mutations through self, just data and transformations.

use cgmath::{perspective, Deg, Rad, InnerSpace, Matrix4, Point3, Vector3};
use bytemuck::{Pod, Zeroable};

/// Camera data as a plain old data structure
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraData {
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub pitch_radians: f32,
    pub aspect_ratio: f32,
    pub fovy_radians: f32,
    pub znear: f32,
    pub zfar: f32,
    _padding: [f32; 3], // Align to 16 bytes
}

/// Camera uniform buffer for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_matrix: [[f32; 4]; 4],
    pub projection_matrix: [[f32; 4]; 4],
    pub view_projection_matrix: [[f32; 4]; 4],
    pub position: [f32; 3],
    _padding: f32,
}

// Pure functions for camera operations

/// Initialize camera data with default values
pub fn init_camera(width: u32, height: u32) -> CameraData {
    CameraData {
        position: [0.0, 10.0, 10.0],
        yaw_radians: -std::f32::consts::FRAC_PI_2, // -90 degrees
        pitch_radians: 0.0,
        aspect_ratio: width as f32 / height as f32,
        fovy_radians: std::f32::consts::FRAC_PI_4, // 45 degrees
        znear: 0.1,
        zfar: 1000.0,
        _padding: [0.0; 3],
    }
}

/// Initialize camera data with a safe spawn position
pub fn init_camera_with_spawn(width: u32, height: u32, spawn_x: f32, spawn_y: f32, spawn_z: f32) -> CameraData {
    CameraData {
        position: [spawn_x, spawn_y, spawn_z],
        yaw_radians: -std::f32::consts::FRAC_PI_2, // -90 degrees
        pitch_radians: 0.0,
        aspect_ratio: width as f32 / height as f32,
        fovy_radians: std::f32::consts::FRAC_PI_4, // 45 degrees
        znear: 0.1,
        zfar: 1000.0,
        _padding: [0.0; 3],
    }
}

/// Update camera aspect ratio for window resize
pub fn update_aspect_ratio(camera: &CameraData, width: u32, height: u32) -> CameraData {
    let mut updated = *camera;
    updated.aspect_ratio = width as f32 / height as f32;
    updated
}

/// Calculate forward vector from camera orientation
pub fn calculate_forward_vector(yaw_rad: f32, pitch_rad: f32) -> Vector3<f32> {
    let (sin_yaw, cos_yaw) = yaw_rad.sin_cos();
    let (sin_pitch, cos_pitch) = pitch_rad.sin_cos();
    
    Vector3::new(
        cos_pitch * cos_yaw,
        sin_pitch,
        cos_pitch * sin_yaw,
    )
}

/// Calculate right vector from forward vector
pub fn calculate_right_vector(forward: Vector3<f32>) -> Vector3<f32> {
    forward.cross(Vector3::unit_y()).normalize()
}

/// Build view matrix from camera data
pub fn build_view_matrix(camera: &CameraData) -> Matrix4<f32> {
    let position = Point3::new(camera.position[0], camera.position[1], camera.position[2]);
    let forward = calculate_forward_vector(camera.yaw_radians, camera.pitch_radians);
    
    Matrix4::look_at_rh(
        position,
        position + forward,
        Vector3::unit_y(),
    )
}

/// Build projection matrix from camera data
pub fn build_projection_matrix(camera: &CameraData) -> Matrix4<f32> {
    perspective(
        Rad(camera.fovy_radians),
        camera.aspect_ratio,
        camera.znear,
        camera.zfar,
    )
}

/// Build camera uniform buffer for GPU
pub fn build_camera_uniform(camera: &CameraData) -> CameraUniform {
    let view = build_view_matrix(camera);
    let proj = build_projection_matrix(camera);
    let view_proj = proj * view;
    
    CameraUniform {
        view_matrix: view.into(),
        projection_matrix: proj.into(),
        view_projection_matrix: view_proj.into(),
        position: camera.position,
        _padding: 0.0,
    }
}

/// Camera movement transformations (returns new camera data)
pub mod transform {
    use super::*;
    
    /// Move camera forward by amount
    pub fn move_forward(camera: &CameraData, amount: f32) -> CameraData {
        let forward = calculate_forward_vector(camera.yaw_radians, camera.pitch_radians);
        let mut updated = *camera;
        updated.position[0] += forward.x * amount;
        updated.position[1] += forward.y * amount;
        updated.position[2] += forward.z * amount;
        updated
    }
    
    /// Move camera right by amount
    pub fn move_right(camera: &CameraData, amount: f32) -> CameraData {
        let forward = calculate_forward_vector(camera.yaw_radians, camera.pitch_radians);
        let right = calculate_right_vector(forward);
        let mut updated = *camera;
        updated.position[0] += right.x * amount;
        updated.position[1] += right.y * amount;
        updated.position[2] += right.z * amount;
        updated
    }
    
    /// Move camera up by amount
    pub fn move_up(camera: &CameraData, amount: f32) -> CameraData {
        let mut updated = *camera;
        updated.position[1] += amount;
        updated
    }
    
    /// Rotate camera by delta yaw and pitch (in radians)
    pub fn rotate(camera: &CameraData, delta_yaw: f32, delta_pitch: f32) -> CameraData {
        let mut updated = *camera;
        updated.yaw_radians += delta_yaw;
        updated.pitch_radians += delta_pitch;
        
        // Clamp pitch to prevent camera flipping
        const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
        updated.pitch_radians = updated.pitch_radians.clamp(-MAX_PITCH, MAX_PITCH);
        
        updated
    }
}

/// Batch camera operations for efficiency
pub struct CameraTransformBatch {
    pub forward: f32,
    pub right: f32,
    pub up: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraTransformBatch {
    fn default() -> Self {
        Self {
            forward: 0.0,
            right: 0.0,
            up: 0.0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

/// Apply batch of transforms to camera in one operation
pub fn apply_transform_batch(camera: &CameraData, batch: &CameraTransformBatch) -> CameraData {
    // Calculate vectors once
    let forward = calculate_forward_vector(camera.yaw_radians, camera.pitch_radians);
    let right = calculate_right_vector(forward);
    
    // Apply all transformations
    let mut updated = *camera;
    
    // Position updates
    updated.position[0] += forward.x * batch.forward + right.x * batch.right;
    updated.position[1] += forward.y * batch.forward + right.y * batch.right + batch.up;
    updated.position[2] += forward.z * batch.forward + right.z * batch.right;
    
    // Rotation updates
    updated.yaw_radians += batch.yaw;
    updated.pitch_radians += batch.pitch;
    
    // Clamp pitch
    const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
    updated.pitch_radians = updated.pitch_radians.clamp(-MAX_PITCH, MAX_PITCH);
    
    updated
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_camera_initialization() {
        let camera = init_camera(1920, 1080);
        assert_eq!(camera.position, [0.0, 10.0, 10.0]);
        assert!((camera.aspect_ratio - 1920.0 / 1080.0).abs() < 0.001);
    }
    
    #[test]
    fn test_camera_movement() {
        let camera = init_camera(1920, 1080);
        let moved = transform::move_forward(&camera, 1.0);
        
        // Camera should have moved forward
        assert_ne!(camera.position, moved.position);
    }
    
    #[test]
    fn test_batch_transforms() {
        let camera = init_camera(1920, 1080);
        let batch = CameraTransformBatch {
            forward: 1.0,
            right: 0.5,
            up: 0.2,
            yaw: 0.1,
            pitch: 0.05,
        };
        
        let transformed = apply_transform_batch(&camera, &batch);
        assert_ne!(camera.position, transformed.position);
        assert_ne!(camera.yaw_radians, transformed.yaw_radians);
        assert_ne!(camera.pitch_radians, transformed.pitch_radians);
    }
}