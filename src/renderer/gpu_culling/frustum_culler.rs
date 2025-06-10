/// GPU Frustum Culling Implementation
/// 
/// Performs frustum culling entirely on GPU using compute shaders.
/// Part of Sprint 28: GPU-Driven Rendering Optimization

use wgpu::{Device, Buffer, BindGroup, ComputePipeline};
use wgpu::util::DeviceExt;
use super::{GpuCamera, ChunkInstance, DrawCommand, CullingStats};

pub struct FrustumCuller {
    pipeline: ComputePipeline,
    clear_pipeline: ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    
    // Buffers
    camera_buffer: Buffer,
    draw_commands_buffer: Buffer,
    visible_instances_buffer: Buffer,
    draw_count_buffer: Buffer,
    
    max_chunks: usize,
}

impl FrustumCuller {
    pub fn new(device: &Device, max_chunks: usize) -> Self {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Frustum Cull Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("frustum_cull.wgsl").into()),
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Frustum Cull Bind Group Layout"),
            entries: &[
                // Camera uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Chunk instances (read)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Draw commands (read_write)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Visible instances (read_write)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Culling stats (read_write)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Draw count (atomic)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Frustum Cull Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create main culling pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Frustum Cull Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });
        
        // Create clear counters pipeline
        let clear_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Clear Counters Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "clear_counters",
        });
        
        // Create buffers
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<GpuCamera>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let draw_commands_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Draw Commands Buffer"),
            size: (std::mem::size_of::<DrawCommand>() * max_chunks) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });
        
        let visible_instances_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Visible Instances Buffer"),
            size: (std::mem::size_of::<u32>() * max_chunks) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        
        let draw_count_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Draw Count Buffer"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        
        Self {
            pipeline,
            clear_pipeline,
            bind_group_layout,
            camera_buffer,
            draw_commands_buffer,
            visible_instances_buffer,
            draw_count_buffer,
            max_chunks,
        }
    }
    
    /// Perform frustum culling
    pub fn cull(
        &self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        camera: &GpuCamera,
        chunk_instances: &Buffer,
        chunk_count: u32,
        stats_buffer: &Buffer,
    ) -> &Buffer {
        // Update camera uniform
        encoder.copy_buffer_to_buffer(
            &self.create_temp_camera_buffer(device, camera),
            0,
            &self.camera_buffer,
            0,
            std::mem::size_of::<GpuCamera>() as u64,
        );
        
        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frustum Cull Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: chunk_instances.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.draw_commands_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.visible_instances_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: stats_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.draw_count_buffer.as_entire_binding(),
                },
            ],
        });
        
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Frustum Culling Pass"),
            timestamp_writes: None,
        });
        
        // Clear counters
        compute_pass.set_pipeline(&self.clear_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(1, 1, 1);
        
        // Perform culling
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        
        let workgroups = (chunk_count + 127) / 128; // WORKGROUP_SIZE = 128
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
        
        drop(compute_pass);
        
        // Return buffer containing visible instance indices
        &self.visible_instances_buffer
    }
    
    fn create_temp_camera_buffer(&self, device: &Device, camera: &GpuCamera) -> Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Temp Camera Buffer"),
            contents: bytemuck::cast_slice(&[*camera]),
            usage: wgpu::BufferUsages::COPY_SRC,
        })
    }
    
    /// Get draw commands buffer for indirect rendering
    pub fn get_draw_commands_buffer(&self) -> &Buffer {
        &self.draw_commands_buffer
    }
    
    /// Get draw count for indirect multi-draw
    pub fn get_draw_count_buffer(&self) -> &Buffer {
        &self.draw_count_buffer
    }
}