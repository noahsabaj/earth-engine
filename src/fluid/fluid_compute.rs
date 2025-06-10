use wgpu::{Device, ComputePipeline, BindGroup, BindGroupLayout, CommandEncoder};
use crate::fluid::{FluidBuffer, FluidConstants, BoundaryConditions};
use std::sync::Arc;

/// Fluid compute pipeline for GPU simulation
pub struct FluidCompute {
    /// Advection pipeline (move fluid by velocity)
    advection_pipeline: ComputePipeline,
    
    /// External forces pipeline (gravity, sources)
    forces_pipeline: ComputePipeline,
    
    /// Viscosity solver pipeline
    viscosity_pipeline: ComputePipeline,
    
    /// Divergence calculation pipeline
    divergence_pipeline: ComputePipeline,
    
    /// Bind group layout
    bind_group_layout: BindGroupLayout,
    
    /// Device reference
    device: Arc<Device>,
}

impl FluidCompute {
    /// Create new fluid compute pipeline
    pub fn new(device: Arc<Device>) -> Self {
        let bind_group_layout = create_fluid_bind_group_layout(&device);
        
        // Create compute pipelines
        let advection_pipeline = create_advection_pipeline(&device, &bind_group_layout);
        let forces_pipeline = create_forces_pipeline(&device, &bind_group_layout);
        let viscosity_pipeline = create_viscosity_pipeline(&device, &bind_group_layout);
        let divergence_pipeline = create_divergence_pipeline(&device, &bind_group_layout);
        
        Self {
            advection_pipeline,
            forces_pipeline,
            viscosity_pipeline,
            divergence_pipeline,
            bind_group_layout,
            device,
        }
    }
    
    /// Run one fluid simulation step
    pub fn step(
        &self,
        encoder: &mut CommandEncoder,
        fluid_buffer: &FluidBuffer,
        constants: &FluidConstants,
        boundaries: &BoundaryConditions,
        bind_group: &BindGroup,
    ) {
        // Step 1: Apply external forces (gravity, sources)
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fluid Forces Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.forces_pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            let workgroups = calculate_workgroups(fluid_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Step 2: Advection (semi-Lagrangian)
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fluid Advection Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.advection_pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            let workgroups = calculate_workgroups(fluid_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Copy temp buffer back to main buffer
        encoder.copy_buffer_to_buffer(
            &fluid_buffer.temp_buffer,
            0,
            &fluid_buffer.voxel_buffer,
            0,
            fluid_buffer.voxel_buffer.size(),
        );
        
        // Step 3: Viscosity (if needed)
        if constants.viscosity_damping < 1.0 {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fluid Viscosity Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.viscosity_pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            let workgroups = calculate_workgroups(fluid_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Step 4: Calculate divergence for pressure solve
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Fluid Divergence Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.divergence_pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            let workgroups = calculate_workgroups(fluid_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Note: Pressure solve is handled by PressureSolver
    }
    
    /// Get bind group layout
    pub fn get_bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }
}

/// Fluid simulation pipeline combining all steps
pub struct FluidPipeline {
    /// Fluid compute shaders
    fluid_compute: FluidCompute,
    
    /// Current bind group
    bind_group: Option<BindGroup>,
    
    /// Constants buffer
    constants_buffer: wgpu::Buffer,
    
    /// Boundaries buffer
    boundaries_buffer: wgpu::Buffer,
}

impl FluidPipeline {
    /// Create new fluid pipeline
    pub fn new(device: Arc<Device>) -> Self {
        let fluid_compute = FluidCompute::new(device.clone());
        
        // Create constants buffer
        let constants_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fluid Constants"),
            size: std::mem::size_of::<FluidConstants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create boundaries buffer
        let boundaries_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fluid Boundaries"),
            size: std::mem::size_of::<BoundaryConditions>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            fluid_compute,
            bind_group: None,
            constants_buffer,
            boundaries_buffer,
        }
    }
    
    /// Initialize with fluid buffer
    pub fn init(&mut self, device: &Device, fluid_buffer: &FluidBuffer) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fluid Bind Group"),
            layout: self.fluid_compute.get_bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: fluid_buffer.voxel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: fluid_buffer.temp_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.constants_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.boundaries_buffer.as_entire_binding(),
                },
            ],
        });
        
        self.bind_group = Some(bind_group);
    }
    
    /// Update constants
    pub fn update_constants(&self, queue: &wgpu::Queue, constants: &FluidConstants) {
        queue.write_buffer(&self.constants_buffer, 0, bytemuck::bytes_of(constants));
    }
    
    /// Update boundaries
    pub fn update_boundaries(&self, queue: &wgpu::Queue, boundaries: &BoundaryConditions) {
        queue.write_buffer(&self.boundaries_buffer, 0, bytemuck::bytes_of(boundaries));
    }
    
    /// Run simulation step
    pub fn step(
        &self,
        encoder: &mut CommandEncoder,
        fluid_buffer: &FluidBuffer,
        constants: &FluidConstants,
        boundaries: &BoundaryConditions,
    ) {
        if let Some(bind_group) = &self.bind_group {
            self.fluid_compute.step(
                encoder,
                fluid_buffer,
                constants,
                boundaries,
                bind_group,
            );
        }
    }
}

/// Create bind group layout for fluid simulation
fn create_fluid_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Fluid Bind Group Layout"),
        entries: &[
            // Fluid voxel buffer (read)
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
            // Temp buffer (write)
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
            // Boundaries
            wgpu::BindGroupLayoutEntry {
                binding: 3,
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

/// Create advection pipeline
fn create_advection_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Fluid Advection Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fluid_advection.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Fluid Advection Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Fluid Advection Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "advection_main",
    })
}

/// Create forces pipeline
fn create_forces_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Fluid Forces Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fluid_forces.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Fluid Forces Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Fluid Forces Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "forces_main",
    })
}

/// Create viscosity pipeline
fn create_viscosity_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Fluid Viscosity Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fluid_viscosity.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Fluid Viscosity Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Fluid Viscosity Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "viscosity_main",
    })
}

/// Create divergence pipeline
fn create_divergence_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Fluid Divergence Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fluid_divergence.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Fluid Divergence Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Fluid Divergence Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "divergence_main",
    })
}

/// Calculate workgroup count for dispatch
fn calculate_workgroups(size: (u32, u32, u32)) -> (u32, u32, u32) {
    (
        (size.0 + 7) / 8,
        (size.1 + 7) / 8,
        (size.2 + 7) / 8,
    )
}