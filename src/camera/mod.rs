/// Camera Module
/// 
/// Sprint 35: Transitioning to data-oriented design.
/// The old Camera struct is deprecated in favor of data_camera module.

pub mod data_camera;

// Re-export data-oriented camera as the primary interface
pub use data_camera::{
    CameraData, CameraUniform, CameraTransformBatch,
    init_camera, init_camera_with_spawn, update_aspect_ratio, build_view_matrix, 
    build_projection_matrix, build_camera_uniform,
    apply_transform_batch,
};

// Re-export transform functions as primary DOP interface
pub use data_camera::transform::{
    move_forward as camera_move_forward,
    move_right as camera_move_right, 
    move_up as camera_move_up,
    rotate as camera_rotate,
};

// Export the DOP functions defined in this module
// camera_resize is exported as update_aspect_ratio from data_camera

// Export the standalone function
// calculate_forward_vector is defined at the bottom of this file

use cgmath::{perspective, Deg, InnerSpace, Matrix4, Point3, Vector3};

#[deprecated(since="0.35.0", note="Use data_camera::CameraData instead for data-oriented design")]
#[derive(Debug)]
pub struct Camera {
    pub position: Point3<f32>,
    pub yaw: Deg<f32>,
    pub pitch: Deg<f32>,
    aspect: f32,
    fovy: Deg<f32>,
    znear: f32,
    zfar: f32,
}

#[allow(deprecated)]
impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            position: Point3::new(0.0, 80.0, 0.0), // Start above typical terrain height (y=64)
            yaw: Deg(-90.0),
            pitch: Deg(0.0),
            aspect: width as f32 / height as f32,
            fovy: Deg(45.0),
            znear: 0.1,
            zfar: 1000.0,
        }
    }
    
    pub fn new_with_position(width: u32, height: u32, x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Point3::new(x, y, z),
            yaw: Deg(-90.0),
            pitch: Deg(0.0),
            aspect: width as f32 / height as f32,
            fovy: Deg(45.0),
            znear: 0.1,
            zfar: 1000.0,
        }
    }


    pub fn build_view_matrix(&self) -> Matrix4<f32> {
        let (sin_yaw, cos_yaw) = cgmath::Rad::from(self.yaw).0.sin_cos();
        let (sin_pitch, cos_pitch) = cgmath::Rad::from(self.pitch).0.sin_cos();

        let direction = Vector3::new(
            cos_pitch * cos_yaw,
            sin_pitch,
            cos_pitch * sin_yaw,
        );

        Matrix4::look_at_rh(
            self.position,
            self.position + direction,
            Vector3::unit_y(),
        )
    }

    pub fn build_projection_matrix(&self) -> Matrix4<f32> {
        perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }

    pub fn get_forward_vector(&self) -> Vector3<f32> {
        let (sin_yaw, cos_yaw) = cgmath::Rad::from(self.yaw).0.sin_cos();
        let (sin_pitch, cos_pitch) = cgmath::Rad::from(self.pitch).0.sin_cos();

        Vector3::new(
            cos_pitch * cos_yaw,
            sin_pitch,
            cos_pitch * sin_yaw,
        )
    }

    pub fn get_right_vector(&self) -> Vector3<f32> {
        let forward = self.get_forward_vector();
        forward.cross(Vector3::unit_y()).normalize()
    }




}

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

    Vector3::new(
        cos_pitch * cos_yaw,
        sin_pitch,
        cos_pitch * sin_yaw,
    )
}

/// Convert deprecated Camera to CameraData
/// This is a compatibility function during the transition to data-oriented design
#[allow(deprecated)]
pub fn camera_to_data(camera: &Camera) -> data_camera::CameraData {
    use cgmath::Rad;
    let mut camera_data = data_camera::init_camera(800, 600); // Get default with proper padding
    camera_data.position = [camera.position.x, camera.position.y, camera.position.z];
    camera_data.yaw_radians = Rad::from(camera.yaw).0;
    camera_data.pitch_radians = Rad::from(camera.pitch).0;
    camera_data.aspect_ratio = camera.aspect;
    camera_data.fovy_radians = Rad::from(camera.fovy).0;
    camera_data.znear = camera.znear;
    camera_data.zfar = camera.zfar;
    camera_data
}