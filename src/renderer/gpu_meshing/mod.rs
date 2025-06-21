//! GPU Mesh Generation System - Pure DOP design
//! 
//! All mesh generation happens on GPU with zero CPU involvement

pub mod types;
pub mod pipeline;
pub mod dispatch;

pub use types::*;
pub use pipeline::*;
pub use dispatch::*;

use std::sync::Arc;

/// GPU meshing state - pure data, no methods
pub struct GpuMeshingState {
    /// GPU device
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    
    /// Compute pipeline for mesh generation
    pub mesh_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    
    /// Pre-allocated mesh output buffers
    pub mesh_buffers: Vec<GpuMeshBuffer>,
    
    /// Indirect draw command buffer
    pub indirect_buffer: wgpu::Buffer,
    
    /// Mesh generation statistics
    pub stats: MeshingStats,
    
    /// Track buffer allocation (wrapped in Mutex for interior mutability)
    pub allocator: std::sync::Mutex<BufferAllocator>,
}

/// Buffer allocation tracker
pub struct BufferAllocator {
    /// Track which buffer slots are in use (chunk_pos -> buffer_index)
    pub allocated_buffers: std::collections::HashMap<crate::ChunkPos, u32>,
    /// Track free buffer indices
    pub free_buffers: Vec<u32>,
}

/// Initialize GPU meshing system
pub fn create_gpu_meshing_state(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
) -> GpuMeshingState {
    // Create compute pipeline
    let (mesh_pipeline, bind_group_layout) = pipeline::create_mesh_generation_pipeline(&device);
    
    // Pre-allocate mesh buffers
    let mesh_buffers = (0..MAX_CONCURRENT_MESHES)
        .map(|i| create_gpu_mesh_buffer(&device, i as u32))
        .collect();
    
    // Create indirect command buffer
    let indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Indirect Mesh Commands"),
        size: (std::mem::size_of::<IndirectDrawCommand>() * MAX_CONCURRENT_MESHES) as u64,
        usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    // Initialize allocator
    let allocator = std::sync::Mutex::new(BufferAllocator {
        allocated_buffers: std::collections::HashMap::new(),
        free_buffers: (0..MAX_CONCURRENT_MESHES as u32).collect(),
    });
    
    GpuMeshingState {
        device,
        queue,
        mesh_pipeline,
        bind_group_layout,
        mesh_buffers,
        indirect_buffer,
        stats: MeshingStats::default(),
        allocator,
    }
}

/// Constants for GPU meshing
pub const MAX_CONCURRENT_MESHES: usize = 256;
pub const MAX_VERTICES_PER_CHUNK: usize = 65536;
pub const MAX_INDICES_PER_CHUNK: usize = 98304; // 1.5x vertices
pub const WORKGROUP_SIZE: u32 = 64; // 4x4x4 voxels per workgroup