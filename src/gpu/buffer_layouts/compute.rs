//! Compute shader buffer layout definitions
//! 
//! Defines buffer structures for various compute operations.

use bytemuck::{Pod, Zeroable};

/// Atomic counter for GPU operations
/// Used for draw call counting, statistics, etc.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct AtomicCounter {
    /// The counter value (must use atomic operations)
    pub count: u32,
}

/// GPU culling statistics
/// Total size: 16 bytes (aligned)
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct CullingStats {
    /// Total objects tested
    pub total_tested: u32,
    
    /// Objects culled by frustum test
    pub frustum_culled: u32,
    
    /// Objects culled by distance
    pub distance_culled: u32,
    
    /// Objects that passed all tests
    pub drawn: u32,
}

/// Particle simulation parameters
/// Total size: 32 bytes (aligned)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ParticleParams {
    /// Gravity acceleration
    pub gravity: [f32; 3],
    
    /// Delta time for simulation
    pub delta_time: f32,
    
    /// Wind force
    pub wind: [f32; 3],
    
    /// Damping factor
    pub damping: f32,
}

/// Individual particle data
/// Total size: 32 bytes (aligned)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ParticleData {
    /// World position
    pub position: [f32; 3],
    
    /// Lifetime remaining (0-1)
    pub lifetime: f32,
    
    /// Velocity vector
    pub velocity: [f32; 3],
    
    /// Particle size
    pub size: f32,
}

/// Physics constraint for GPU solver
/// Total size: 32 bytes (aligned)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PhysicsConstraint {
    /// First particle index
    pub particle_a: u32,
    
    /// Second particle index
    pub particle_b: u32,
    
    /// Rest length of constraint
    pub rest_length: f32,
    
    /// Constraint stiffness (0-1)
    pub stiffness: f32,
    
    /// Constraint type flags
    pub constraint_type: u32,
    
    /// Padding
    pub _padding: [u32; 3],
}

/// GPU BVH node for spatial queries
/// Total size: 32 bytes (aligned)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BVHNode {
    /// Minimum bounds
    pub min_bounds: [f32; 3],
    
    /// Left child index (or first primitive for leaf)
    pub left_first: u32,
    
    /// Maximum bounds
    pub max_bounds: [f32; 3],
    
    /// Primitive count (0 for internal nodes)
    pub primitive_count: u32,
}

impl BVHNode {
    /// Check if this is a leaf node
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.primitive_count > 0
    }
}

/// GPU work queue entry for parallel processing
/// Total size: 16 bytes (aligned)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WorkQueueEntry {
    /// Work item type/command
    pub work_type: u32,
    
    /// Associated data index
    pub data_index: u32,
    
    /// Priority (higher = more important)
    pub priority: u32,
    
    /// Status flags
    pub status: u32,
}

/// Compute dispatch parameters
/// Total size: 32 bytes (aligned)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ComputeDispatchParams {
    /// Total work items
    pub total_items: u32,
    
    /// Items per workgroup
    pub items_per_workgroup: u32,
    
    /// Number of workgroups
    pub workgroup_count: [u32; 3],
    
    /// Global offset
    pub global_offset: [u32; 3],
    
    /// Padding
    pub _padding: u32,
}

impl ComputeDispatchParams {
    /// Calculate dispatch parameters for 1D workload
    pub fn calculate_1d(total_items: u32, workgroup_size: u32) -> Self {
        let workgroups = (total_items + workgroup_size - 1) / workgroup_size;
        
        Self {
            total_items,
            items_per_workgroup: workgroup_size,
            workgroup_count: [workgroups, 1, 1],
            global_offset: [0; 3],
            _padding: 0,
        }
    }
    
    /// Calculate dispatch parameters for 2D workload
    pub fn calculate_2d(width: u32, height: u32, workgroup_size: u32) -> Self {
        let workgroups_x = (width + workgroup_size - 1) / workgroup_size;
        let workgroups_y = (height + workgroup_size - 1) / workgroup_size;
        
        Self {
            total_items: width * height,
            items_per_workgroup: workgroup_size * workgroup_size,
            workgroup_count: [workgroups_x, workgroups_y, 1],
            global_offset: [0; 3],
            _padding: 0,
        }
    }
}

/// Compute buffer layout information
pub struct ComputeBufferLayout;

impl ComputeBufferLayout {
    /// Atomic counter size
    pub const ATOMIC_SIZE: u64 = 4;
    
    /// Culling stats size
    pub const STATS_SIZE: u64 = 16;
    
    /// Particle data size
    pub const PARTICLE_SIZE: u64 = 32;
    
    /// BVH node size
    pub const BVH_NODE_SIZE: u64 = 32;
    
    /// Work queue entry size
    pub const WORK_ENTRY_SIZE: u64 = 16;
    
    /// Calculate particle buffer size
    #[inline]
    pub fn particle_buffer_size(count: u32) -> u64 {
        count as u64 * Self::PARTICLE_SIZE
    }
    
    /// Calculate BVH buffer size
    #[inline]
    pub fn bvh_buffer_size(node_count: u32) -> u64 {
        node_count as u64 * Self::BVH_NODE_SIZE
    }
}

/// Common compute shader workgroup sizes
pub mod workgroup_sizes {
    /// Small workgroup for simple kernels
    pub const SMALL: u32 = 64;
    
    /// Medium workgroup for balanced work
    pub const MEDIUM: u32 = 128;
    
    /// Large workgroup for memory-bound kernels
    pub const LARGE: u32 = 256;
    
    /// 2D tile size for image processing
    pub const TILE_2D: u32 = 16;
    
    /// 3D tile size for volume processing
    pub const TILE_3D: u32 = 8;
}