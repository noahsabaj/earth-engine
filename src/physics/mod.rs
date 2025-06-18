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

// Import and re-export physics constants from single source of truth
include!("../../constants.rs");
pub use physics::*;

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
            spatial_hash_cell_size: SPATIAL_HASH_CELL_SIZE, // 40 voxel cells (4m in voxel units)
            worker_threads: num_cpus::get(),
            enable_gpu_buffers: false,
        }
    }
}