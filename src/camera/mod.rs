/// Camera Module
/// 
/// Sprint 35: Transitioning to data-oriented design.
/// The old Camera struct is deprecated in favor of data_camera module.

pub mod data_camera;

// Re-export data-oriented camera as the primary interface
pub use data_camera::{
    CameraData, CameraUniform, CameraTransformBatch,
    init_camera, update_aspect_ratio, build_view_matrix, 
    build_projection_matrix, build_camera_uniform,
    apply_transform_batch,
};

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

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
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

    pub fn move_forward(&mut self, amount: f32) {
        let forward = self.get_forward_vector();
        self.position += forward * amount;
    }

    pub fn move_right(&mut self, amount: f32) {
        let right = self.get_right_vector();
        self.position += right * amount;
    }

    pub fn move_up(&mut self, amount: f32) {
        self.position.y += amount;
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += Deg(delta_yaw);
        self.pitch += Deg(delta_pitch);
        
        // Clamp pitch to prevent camera flipping
        if self.pitch < Deg(-89.0) {
            self.pitch = Deg(-89.0);
        } else if self.pitch > Deg(89.0) {
            self.pitch = Deg(89.0);
        }
    }
}