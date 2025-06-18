pub mod aabb;
pub mod body;
pub mod gpu_physics_world;

pub use aabb::AABB;
pub use body::{PhysicsBody, RigidBody, PlayerBody, MovementState};
pub use gpu_physics_world::GpuPhysicsWorld;


use cgmath::Vector3;

pub type Vec3 = Vector3<f32>;

pub const GRAVITY: f32 = -20.0; // Roughly 2x real gravity for better game feel
pub const TERMINAL_VELOCITY: f32 = -50.0;
pub const PHYSICS_TICK_RATE: f32 = 50.0; // 50ms = 20Hz
pub const FIXED_TIMESTEP: f32 = 1.0 / 20.0; // 0.05 seconds

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaycastHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}