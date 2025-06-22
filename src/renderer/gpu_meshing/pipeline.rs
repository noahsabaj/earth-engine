//! GPU mesh generation pipeline - pure functions only

use crate::renderer::gpu_meshing::types::*;
use std::sync::Arc;

/// Create mesh generation compute pipeline
pub fn create_mesh_generation_pipeline(
    device: &wgpu::Device,
) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
    // Process shader with includes
    let shader_source = include_str!("../../shaders/mesh/mesh_generation.wgsl");
    let base_path = std::path::Path::new("src/shaders/mesh/mesh_generation.wgsl");

    log::info!("[GPU Meshing] Starting mesh generation pipeline creation");

    let processed_source =
        match crate::gpu::preprocessor::preprocess_shader_content(shader_source, base_path) {
            Ok(content) => {
                log::info!(
                    "[GPU Meshing] Successfully preprocessed shader ({} bytes)",
                    content.len()
                );
                content
            }
            Err(e) => {
                log::error!(
                    "[GPU Meshing] Failed to preprocess mesh generation shader: {}",
                    e
                );
                shader_source.to_string()
            }
        };

    // Create shader through unified GPU system
    log::info!("[GPU Meshing] Creating shader through unified GPU system");
    log::debug!(
        "[GPU Meshing] Shader source length: {} bytes",
        processed_source.len()
    );

    // Log first few lines of shader for debugging
    let shader_lines: Vec<&str> = processed_source.lines().take(10).collect();
    log::debug!(
        "[GPU Meshing] First 10 lines of shader:\n{}",
        shader_lines.join("\n")
    );

    let validated_shader = match crate::gpu::automation::create_gpu_shader(
        device,
        "mesh_generation",
        &processed_source,
    ) {
        Ok(shader) => shader,
        Err(e) => {
            log::error!("[GPU Meshing] Failed to create shader: {:?}", e);
            panic!("[GPU Meshing] Shader creation failed: {:?}", e);
        }
    };

    // Create bind group layout using our macro
    let bind_group_layout = crate::create_bind_group_layout!(
        device,
        "Mesh Generation Bind Group Layout",
        0 => buffer(storage_read),  // World voxel data
        1 => buffer(storage_read),  // Mesh requests
        2 => buffer(storage),       // Vertices output (interleaved)
        3 => buffer(storage),       // Index buffer output
        4 => buffer(storage),       // Metadata output
        5 => buffer(storage),       // Indirect commands output
        6 => buffer(uniform)        // Meshing parameters
    );

    // Create pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh Generation Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // Create compute pipeline
    log::info!("[GPU Meshing] Creating compute pipeline with entry point 'generate_mesh'");

    // Note: create_compute_pipeline can fail if the shader has validation errors
    // The error will be reported through the device error handler
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Mesh Generation Pipeline"),
        layout: Some(&pipeline_layout),
        module: &validated_shader.module,
        entry_point: "generate_mesh",
    });

    log::info!("[GPU Meshing] Successfully created mesh generation pipeline");
    (pipeline, bind_group_layout)
}

/// Create GPU mesh buffer
pub fn create_gpu_mesh_buffer(device: &wgpu::Device, buffer_id: u32) -> GpuMeshBuffer {
    // Vertex buffer (interleaved: position + color + normal + light + ao)
    // Size calculation: 11 floats per vertex (3 + 3 + 3 + 1 + 1)
    let vertex_size = std::mem::size_of::<crate::renderer::vertex::Vertex>();
    let vertices = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Mesh {} Vertices", buffer_id)),
        size: (MAX_VERTICES_PER_CHUNK * vertex_size) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Index buffer
    let indices = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Mesh {} Indices", buffer_id)),
        size: (MAX_INDICES_PER_CHUNK * std::mem::size_of::<u32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::INDEX
            | wgpu::BufferUsages::COPY_SRC,
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
        vertices,
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
        2 => mesh.vertices.as_entire_binding(),
        3 => mesh.indices.as_entire_binding(),
        4 => mesh.metadata.as_entire_binding(),
        5 => indirect_buffer.as_entire_binding(),
        6 => params_buffer.as_entire_binding()
    )
}

use super::{MAX_INDICES_PER_CHUNK, MAX_VERTICES_PER_CHUNK};
