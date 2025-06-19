//! GPU mesh generation dispatch - pure functions for executing mesh generation

use crate::renderer::gpu_meshing::{
    GpuMeshingState, MeshRequest, MeshingParams, GpuMeshBuffer,
    WORKGROUP_SIZE, MAX_CONCURRENT_MESHES
};
use crate::world::core::ChunkPos;

/// Generate meshes for a batch of chunks
pub fn generate_chunk_meshes(
    state: &GpuMeshingState,
    world_buffer: &wgpu::Buffer,
    chunk_positions: &[ChunkPos],
    lod_level: u32,
) -> Vec<u32> {
    if chunk_positions.is_empty() {
        return Vec::new();
    }
    
    let batch_size = chunk_positions.len().min(MAX_CONCURRENT_MESHES);
    let chunks = &chunk_positions[..batch_size];
    
    // Create mesh requests
    let requests: Vec<MeshRequest> = chunks.iter()
        .enumerate()
        .map(|(i, pos)| MeshRequest {
            chunk_pos: [pos.x, pos.y, pos.z],
            lod_level,
            buffer_index: i as u32,
            flags: 0,
            _padding: [0; 2],
        })
        .collect();
    
    // Create request buffer
    let request_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Mesh Request Buffer"),
        size: (std::mem::size_of::<MeshRequest>() * requests.len()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    // Upload requests
    state.queue.write_buffer(&request_buffer, 0, bytemuck::cast_slice(&requests));
    
    // Create parameters
    let params = MeshingParams {
        chunk_size: 32,
        request_count: requests.len() as u32,
        enable_greedy: 1,
        enable_ao: 1,
        max_vertices: super::MAX_VERTICES_PER_CHUNK as u32,
        max_indices: super::MAX_INDICES_PER_CHUNK as u32,
        _padding: [0; 2],
    };
    
    let params_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Meshing Parameters"),
        size: std::mem::size_of::<MeshingParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    state.queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));
    
    // Create bind group
    let bind_group = super::pipeline::create_mesh_bind_group(
        &state.device,
        &state.bind_group_layout,
        world_buffer,
        &request_buffer,
        &state.mesh_buffers,
        &state.indirect_buffer,
        &params_buffer,
    );
    
    // Create command encoder
    let mut encoder = state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Mesh Generation Encoder"),
    });
    
    // Dispatch compute
    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Mesh Generation Pass"),
            timestamp_writes: None,
        });
        
        compute_pass.set_pipeline(&state.mesh_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        
        // One workgroup per chunk
        let workgroups = requests.len() as u32;
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }
    
    // Submit
    state.queue.submit(std::iter::once(encoder.finish()));
    
    // Return buffer indices that were used
    (0..batch_size as u32).collect()
}

/// Get mesh statistics from GPU
pub fn update_mesh_statistics(
    state: &mut GpuMeshingState,
    generated_count: u32,
) {
    state.stats.total_meshes += generated_count as u64;
    // TODO: Read back actual vertex/index counts from GPU
}

/// Check if mesh buffer is ready
pub fn is_mesh_ready(
    state: &GpuMeshingState,
    buffer_index: u32,
) -> bool {
    // In a real implementation, would check GPU fence/query
    buffer_index < state.mesh_buffers.len() as u32
}

/// Get mesh buffer for rendering
pub fn get_mesh_buffer<'a>(
    state: &'a GpuMeshingState,
    buffer_index: u32,
) -> Option<&'a GpuMeshBuffer> {
    state.mesh_buffers.get(buffer_index as usize)
}

/// Clear mesh buffer pool
pub fn clear_mesh_buffers(state: &mut GpuMeshingState) {
    // GPU buffers remain allocated, just mark as available
    // In real implementation, would reset metadata
}