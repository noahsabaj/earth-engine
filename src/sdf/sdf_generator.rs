use wgpu::{Device, ComputePipeline, BindGroupLayout};
use crate::world_gpu::WorldBuffer;
use crate::sdf::{SdfBuffer, SdfConstants};
use crate::sdf::error::{SdfResult, SdfErrorContext};
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};

/// SDF generation parameters
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SdfGenerationParams {
    /// Chunk offset in voxel space
    pub chunk_offset: [i32; 3],
    
    /// Chunk size in voxels
    pub chunk_size: [u32; 3],
    
    /// SDF grid size (with margins)
    pub sdf_size: [u32; 3],
    
    /// Resolution multiplier
    pub resolution: f32,
    
    /// Padding
    pub _padding: u32,
}

/// GPU-accelerated SDF generator
pub struct SdfGenerator {
    /// Initial voxel to SDF conversion pipeline
    voxel_to_sdf_pipeline: ComputePipeline,
    
    /// Distance propagation pipeline (jump flooding)
    propagation_pipeline: ComputePipeline,
    
    /// Gradient calculation pipeline
    gradient_pipeline: ComputePipeline,
    
    /// Smoothing pipeline
    smoothing_pipeline: ComputePipeline,
    
    /// Bind group layout
    bind_group_layout: BindGroupLayout,
    
    /// Constants buffer
    constants_buffer: wgpu::Buffer,
    
    /// Device reference
    device: Arc<Device>,
}

impl SdfGenerator {
    /// Create new SDF generator
    pub fn new(device: Arc<Device>) -> Self {
        let bind_group_layout = create_sdf_bind_group_layout(&device);
        
        let voxel_to_sdf_pipeline = create_voxel_to_sdf_pipeline(&device, &bind_group_layout);
        let propagation_pipeline = create_propagation_pipeline(&device, &bind_group_layout);
        let gradient_pipeline = create_gradient_pipeline(&device, &bind_group_layout);
        let smoothing_pipeline = create_smoothing_pipeline(&device, &bind_group_layout);
        
        // Create constants buffer
        let constants_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SDF Constants"),
            size: std::mem::size_of::<SdfConstants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            voxel_to_sdf_pipeline,
            propagation_pipeline,
            gradient_pipeline,
            smoothing_pipeline,
            bind_group_layout,
            constants_buffer,
            device,
        }
    }
    
    /// Generate SDF from voxel data
    pub fn generate(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &WorldBuffer,
        sdf_buffer: &SdfBuffer,
        params: &SdfGenerationParams,
    ) -> SdfResult<()> {
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SDF Generation Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: world_buffer.voxel_buffer(),
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: sdf_buffer.buffer.as_ref().sdf_context("sdf_buffer")?,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.constants_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        
        // Step 1: Convert voxels to initial SDF values
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Voxel to SDF Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.voxel_to_sdf_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.set_push_constants(0, bytemuck::bytes_of(params));
            
            let workgroups = calculate_workgroups(sdf_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Step 2: Jump flooding algorithm for distance propagation
        let jump_steps = calculate_jump_steps(sdf_buffer.size);
        for step_size in jump_steps {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(&format!("Jump Flooding Step {}", step_size)),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.propagation_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            // Push step size as constant
            compute_pass.set_push_constants(0, &step_size.to_ne_bytes());
            
            let workgroups = calculate_workgroups(sdf_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Step 3: Calculate gradients
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Gradient Calculation Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.gradient_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroups = calculate_workgroups(sdf_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Step 4: Apply smoothing
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SDF Smoothing Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.smoothing_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            let workgroups = calculate_workgroups(sdf_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        Ok(())
    }
    
    /// Update constants
    pub fn update_constants(&self, queue: &wgpu::Queue, constants: &SdfConstants) {
        queue.write_buffer(&self.constants_buffer, 0, bytemuck::bytes_of(constants));
    }
    
    /// Get bind group layout
    pub fn get_bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }
}

/// Create bind group layout for SDF generation
fn create_sdf_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("SDF Generation Bind Group Layout"),
        entries: &[
            // World voxel buffer (read-only)
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // SDF buffer (read-write)
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Constants
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/// Create voxel to SDF conversion pipeline
fn create_voxel_to_sdf_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Voxel to SDF Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/voxel_to_sdf.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Voxel to SDF Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::COMPUTE,
            range: 0..std::mem::size_of::<SdfGenerationParams>() as u32,
        }],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Voxel to SDF Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "voxel_to_sdf",
    })
}

/// Create distance propagation pipeline
fn create_propagation_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Jump Flooding Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/jump_flooding.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Jump Flooding Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[wgpu::PushConstantRange {
            stages: wgpu::ShaderStages::COMPUTE,
            range: 0..4, // Single u32 for step size
        }],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Jump Flooding Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "jump_flood",
    })
}

/// Create gradient calculation pipeline
fn create_gradient_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("SDF Gradient Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sdf_gradient.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("SDF Gradient Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("SDF Gradient Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "calculate_gradient",
    })
}

/// Create smoothing pipeline
fn create_smoothing_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("SDF Smoothing Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sdf_smooth.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("SDF Smoothing Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("SDF Smoothing Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "smooth_sdf",
    })
}

/// Calculate workgroup count
fn calculate_workgroups(size: (u32, u32, u32)) -> (u32, u32, u32) {
    (
        (size.0 + 7) / 8,
        (size.1 + 7) / 8,
        (size.2 + 7) / 8,
    )
}

/// Calculate jump flooding steps
fn calculate_jump_steps(size: (u32, u32, u32)) -> Vec<u32> {
    let max_dim = size.0.max(size.1).max(size.2);
    let mut steps = Vec::new();
    
    // Start with half the maximum dimension
    let mut step = max_dim / 2;
    while step > 0 {
        steps.push(step);
        step /= 2;
    }
    
    // Add final pass with step size 1
    steps.push(1);
    
    steps
}