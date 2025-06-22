/// Camera Module
///
/// Sprint 35: Transitioning to data-oriented design.
/// The old Camera struct is deprecated in favor of data_camera module.
pub mod data_camera;

// Re-export data-oriented camera as the primary interface
pub use data_camera::{
    apply_transform_batch, build_camera_uniform, build_projection_matrix, build_view_matrix,
    init_camera, init_camera_with_spawn, update_aspect_ratio, CameraData, CameraTransformBatch,
    CameraUniform,
};

// Re-export transform functions as primary DOP interface
pub use data_camera::transform::{
    move_forward as camera_move_forward, move_right as camera_move_right,
    move_up as camera_move_up, rotate as camera_rotate,
};

// Export the DOP functions defined in this module
// camera_resize is exported as update_aspect_ratio from data_camera

// Export the standalone function
// calculate_forward_vector is defined at the bottom of this file

use cgmath::{perspective, Deg, InnerSpace, Matrix4, Point3, Vector3};

// DOP Functions for Camera Operations
// These are the primary interface for data-oriented camera operations

/// Resize camera (update aspect ratio) - DOP function
pub fn camera_resize(camera: &CameraData, width: u32, height: u32) -> CameraData {
    update_aspect_ratio(camera, width, height)
}

/// Calculate forward vector from camera data
/// This is a compatibility function for the data-oriented camera system
pub fn calculate_forward_vector(camera_data: &CameraData) -> Vector3<f32> {
    let (sin_yaw, cos_yaw) = camera_data.yaw_radians.sin_cos();
    let (sin_pitch, cos_pitch) = camera_data.pitch_radians.sin_cos();

    Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw)
}
