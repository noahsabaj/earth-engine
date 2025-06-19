//! GPU mesh generation pipeline - pure functions only

use crate::renderer::gpu_meshing::types::*;
use std::sync::Arc;

/// Create mesh generation compute pipeline
pub fn create_mesh_generation_pipeline(
    device: &wgpu::Device,
) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
    // Create shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("GPU Mesh Generation Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/mesh_generation.wgsl").into()),
    });
    
    // Create bind group layout using our macro
    let bind_group_layout = crate::create_bind_group_layout!(
        device,
        "Mesh Generation Bind Group Layout",
        0 => buffer(storage_read),  // World voxel data
        1 => buffer(storage_read),  // Mesh requests
        2 => buffer(storage),       // Vertex positions output
        3 => buffer(storage),       // Vertex normals output
        4 => buffer(storage),       // Vertex UVs output
        5 => buffer(storage),       // Vertex colors output
        6 => buffer(storage),       // Index buffer output
        7 => buffer(storage),       // Metadata output
        8 => buffer(storage),       // Indirect commands output
        9 => buffer(uniform)        // Meshing parameters
    );
    
    // Create pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh Generation Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    // Create compute pipeline
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Mesh Generation Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("generate_mesh"),
        compilation_options: Default::default(),
        cache: None,
    });
    
    (pipeline, bind_group_layout)
}

/// Create GPU mesh buffer
pub fn create_gpu_mesh_buffer(device: &wgpu::Device, buffer_id: u32) -> GpuMeshBuffer {
    // Position buffer (3 floats per vertex)
    let positions = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Mesh {} Positions", buffer_id)),
        size: (MAX_VERTICES_PER_CHUNK * 3 * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    // Normal buffer (3 floats per vertex)
    let normals = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Mesh {} Normals", buffer_id)),
        size: (MAX_VERTICES_PER_CHUNK * 3 * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    // UV buffer (2 floats per vertex)
    let uvs = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Mesh {} UVs", buffer_id)),
        size: (MAX_VERTICES_PER_CHUNK * 2 * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    // Color buffer (4 floats per vertex)
    let colors = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Mesh {} Colors", buffer_id)),
        size: (MAX_VERTICES_PER_CHUNK * 4 * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    // Index buffer
    let indices = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Mesh {} Indices", buffer_id)),
        size: (MAX_INDICES_PER_CHUNK * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    // Metadata buffer
    let metadata = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Mesh {} Metadata", buffer_id)),
        size: std::mem::size_of::<GpuMeshMetadata>() as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    GpuMeshBuffer {
        positions,
        normals,
        uvs,
        colors,
        indices,
        metadata,
        id: buffer_id,
    }
}

/// Create bind group for mesh generation
pub fn create_mesh_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    world_buffer: &wgpu::Buffer,
    request_buffer: &wgpu::Buffer,
    mesh_buffers: &[GpuMeshBuffer],
    indirect_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    // For simplicity, bind the first mesh buffer
    // In practice, you'd cycle through buffers
    let mesh = &mesh_buffers[0];
    
    crate::create_bind_group!(
        device,
        "Mesh Generation Bind Group",
        layout,
        0 => world_buffer.as_entire_binding(),
        1 => request_buffer.as_entire_binding(),
        2 => mesh.positions.as_entire_binding(),
        3 => mesh.normals.as_entire_binding(),
        4 => mesh.uvs.as_entire_binding(),
        5 => mesh.colors.as_entire_binding(),
        6 => mesh.indices.as_entire_binding(),
        7 => mesh.metadata.as_entire_binding(),
        8 => indirect_buffer.as_entire_binding(),
        9 => params_buffer.as_entire_binding()
    )
}

use super::{MAX_VERTICES_PER_CHUNK, MAX_INDICES_PER_CHUNK};