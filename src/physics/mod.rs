pub mod collision_data;
pub mod error;
pub mod gpu_physics_world;
pub mod integration;
pub mod parallel_solver;
/// Data-oriented physics system using struct-of-arrays for cache efficiency
/// and GPU compatibility.
pub mod physics_tables;
pub mod preallocated_spatial_hash;
pub mod spatial_hash;

pub use collision_data::{CollisionData, ContactPair, ContactPoint};
pub use gpu_physics_world::GpuPhysicsWorld;
pub use integration::{PhysicsIntegrator, WorldAdapter, WorldInterface};
pub use parallel_solver::{ParallelPhysicsSolver, SolverConfig};
pub use physics_tables::{EntityId, PhysicsData, AABB, MAX_ENTITIES};
pub use spatial_hash::{SpatialHash, SpatialHashConfig};

// Re-export physics constants from single source of truth
pub use crate::constants::physics_constants::*;

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
            max_entities: 65536,                            // 64k entities
            spatial_hash_cell_size: SPATIAL_HASH_CELL_SIZE, // 40 voxel cells (4m in voxel units)
            worker_threads: num_cpus::get(),
            enable_gpu_buffers: false,
        }
    }
}
