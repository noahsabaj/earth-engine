/// Data-Oriented Camera System
/// 
/// Sprint 35: Pure data structures with free functions.
/// No methods, no mutations through self, just data and transformations.

use cgmath::{perspective, Rad, InnerSpace, Matrix4, Point3, Vector3};
use bytemuck::{Pod, Zeroable};
use crate::ChunkPos;

// Import camera constants for voxel-scaled measurements
include!("../../constants.rs");
use camera::*;
use measurements::*;

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
        position: [0.0, DEFAULT_HEIGHT, DEFAULT_HEIGHT], // Use voxel-scaled height (100 voxels = 10m)
        yaw_radians: -std::f32::consts::FRAC_PI_2, // -90 degrees
        pitch_radians: 0.0,
        aspect_ratio: width as f32 / height as f32,
        fovy_radians: std::f32::consts::FRAC_PI_4, // 45 degrees
        znear: ZNEAR,  // 1.0 voxel minimum (10cm)
        zfar: ZFAR,    // 10,000 voxels maximum (1km)
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
        znear: ZNEAR,  // 1.0 voxel minimum (10cm)
        zfar: ZFAR,    // 10,000 voxels maximum (1km)
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

/// Diagnostic and logging functions
pub mod diagnostics {
    use super::*;
    
    /// Get camera position as chunk coordinates for spatial context
    /// Uses proper voxel-scaled chunk size (50 voxels = 5m per chunk)
    pub fn camera_chunk_position(camera: &CameraData) -> ChunkPos {
        let chunk_x = (camera.position[0] / CHUNK_SIZE_VOXELS).floor() as i32;
        let chunk_y = (camera.position[1] / CHUNK_SIZE_VOXELS).floor() as i32;
        let chunk_z = (camera.position[2] / CHUNK_SIZE_VOXELS).floor() as i32;
        ChunkPos::new(chunk_x, chunk_y, chunk_z)
    }
    
    /// Get camera position within chunk (0-49 range for 50 voxel chunks)
    pub fn camera_local_position(camera: &CameraData) -> (f32, f32, f32) {
        let local_x = camera.position[0] % CHUNK_SIZE_VOXELS;
        let local_y = camera.position[1] % CHUNK_SIZE_VOXELS;
        let local_z = camera.position[2] % CHUNK_SIZE_VOXELS;
        (local_x, local_y, local_z)
    }
    
    /// Calculate distance from camera to a chunk position
    pub fn distance_to_chunk(camera: &CameraData, chunk_pos: ChunkPos) -> f32 {
        let chunk_center_x = (chunk_pos.x as f32 * CHUNK_SIZE_VOXELS) + (CHUNK_SIZE_VOXELS / 2.0);
        let chunk_center_y = (chunk_pos.y as f32 * CHUNK_SIZE_VOXELS) + (CHUNK_SIZE_VOXELS / 2.0);
        let chunk_center_z = (chunk_pos.z as f32 * CHUNK_SIZE_VOXELS) + (CHUNK_SIZE_VOXELS / 2.0);
        
        let dx = camera.position[0] - chunk_center_x;
        let dy = camera.position[1] - chunk_center_y;
        let dz = camera.position[2] - chunk_center_z;
        
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
    
    /// Log camera spatial context for debugging
    pub fn log_camera_context(camera: &CameraData, context: &str) {
        let chunk_pos = camera_chunk_position(camera);
        let (local_x, local_y, local_z) = camera_local_position(camera);
        
        log::debug!(
            "[CAMERA_CONTEXT] {} - World position: ({:.1}, {:.1}, {:.1}), \
             Chunk: {:?}, Local: ({:.1}, {:.1}, {:.1}), \
             Yaw: {:.1}°, Pitch: {:.1}°",
            context,
            camera.position[0], camera.position[1], camera.position[2],
            chunk_pos,
            local_x, local_y, local_z,
            camera.yaw_radians.to_degrees(),
            camera.pitch_radians.to_degrees()
        );
    }
    
    /// Get chunks within view distance for debugging
    pub fn chunks_in_view_distance(camera: &CameraData, view_distance: u32) -> Vec<ChunkPos> {
        let camera_chunk = camera_chunk_position(camera);
        let mut chunks = Vec::new();
        
        let radius = view_distance as i32;
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                for dz in -radius..=radius {
                    let chunk_pos = ChunkPos::new(
                        camera_chunk.x + dx,
                        camera_chunk.y + dy,
                        camera_chunk.z + dz
                    );
                    
                    // Only include chunks within spherical distance
                    let distance = distance_to_chunk(camera, chunk_pos);
                    if distance <= (view_distance as f32 * CHUNK_SIZE_VOXELS) {
                        chunks.push(chunk_pos);
                    }
                }
            }
        }
        
        chunks
    }
    
    /// Log performance context with camera information
    pub fn log_performance_context(
        camera: &CameraData, 
        operation: &str, 
        duration_ms: f64,
        chunk_count: Option<usize>
    ) {
        let chunk_pos = camera_chunk_position(camera);
        
        match chunk_count {
            Some(count) => {
                log::info!(
                    "[PERFORMANCE] {} completed in {:.2}ms at camera chunk {:?} ({} chunks)",
                    operation, duration_ms, chunk_pos, count
                );
            }
            None => {
                log::info!(
                    "[PERFORMANCE] {} completed in {:.2}ms at camera chunk {:?}",
                    operation, duration_ms, chunk_pos
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_camera_initialization() {
        let camera = init_camera(1920, 1080);
        assert_eq!(camera.position, [0.0, 100.0, 100.0]);
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
    
    #[test]
    fn test_camera_diagnostics() {
        use super::diagnostics::*;
        
        let camera = init_camera_with_spawn(1920, 1080, 100.0, 50.0, 200.0);
        let chunk_pos = camera_chunk_position(&camera);
        
        // Camera at (100, 50, 200) should be in chunk (2, 1, 4) with 50-voxel chunks
        assert_eq!(chunk_pos.x, 2);
        assert_eq!(chunk_pos.y, 1);
        assert_eq!(chunk_pos.z, 4);
        
        let (local_x, local_y, local_z) = camera_local_position(&camera);
        assert!((local_x - 0.0).abs() < 0.001); // 100 % 50 = 0
        assert!((local_y - 0.0).abs() < 0.001); // 50 % 50 = 0
        assert!((local_z - 0.0).abs() < 0.001); // 200 % 50 = 0
    }
    
    #[test]
    fn test_distance_calculation() {
        use super::diagnostics::*;
        
        let camera = init_camera_with_spawn(1920, 1080, 16.0, 16.0, 16.0); // Center of chunk (0,0,0)
        let chunk_pos = ChunkPos::new(0, 0, 0);
        
        let distance = distance_to_chunk(&camera, chunk_pos);
        assert!(distance < 1.0); // Should be very close to chunk center
    }
}