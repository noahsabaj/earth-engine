/// Data-oriented physics system using struct-of-arrays for cache efficiency
/// and GPU compatibility.

pub mod physics_tables;
pub mod collision_data;
pub mod spatial_hash;
pub mod preallocated_spatial_hash;
pub mod parallel_solver;
pub mod integration;
pub mod error;
pub mod gpu_physics_world;

pub use physics_tables::{PhysicsData, EntityId, MAX_ENTITIES, AABB};
pub use collision_data::{CollisionData, ContactPoint, ContactPair};
pub use spatial_hash::{SpatialHash, SpatialHashConfig};
pub use parallel_solver::{ParallelPhysicsSolver, SolverConfig};
pub use integration::{PhysicsIntegrator, WorldInterface, WorldAdapter};
pub use gpu_physics_world::GpuPhysicsWorld;

// Physics simulation constants
pub const GRAVITY: f32 = -9.81;
pub const TERMINAL_VELOCITY: f32 = -50.0;
pub const FIXED_TIMESTEP: f32 = 1.0 / 60.0; // 60 FPS physics

/// Physics configuration for data-oriented system
#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    pub max_entities: usize,
    pub spatial_hash_cell_size: f32,
    pub worker_threads: usize,
    pub enable_gpu_buffers: bool,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            max_entities: 65536, // 64k entities
            spatial_hash_cell_size: 4.0, // 4 meter cells
            worker_threads: num_cpus::get(),
            enable_gpu_buffers: false,
        }
    }
}