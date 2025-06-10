use wgpu::{Device, ComputePipeline, BindGroup, BindGroupLayout, Buffer};
use crate::fluid::FluidBuffer;
use crate::world_gpu::WorldBuffer;
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};

/// Erosion parameters
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ErosionParams {
    /// Erosion rate for water
    pub water_erosion_rate: f32,
    
    /// Erosion rate for lava
    pub lava_erosion_rate: f32,
    
    /// Sediment capacity factor
    pub sediment_capacity: f32,
    
    /// Deposition rate
    pub deposition_rate: f32,
    
    /// Minimum flow velocity for erosion
    pub erosion_threshold: f32,
    
    /// Evaporation rate
    pub evaporation_rate: f32,
    
    /// Padding
    pub _padding: [f32; 2],
}

impl Default for ErosionParams {
    fn default() -> Self {
        Self {
            water_erosion_rate: 0.01,
            lava_erosion_rate: 0.001,
            sediment_capacity: 0.1,
            deposition_rate: 0.02,
            erosion_threshold: 0.5,
            evaporation_rate: 0.0001,
            _padding: [0.0; 2],
        }
    }
}

/// Fluid-terrain interaction system
pub struct TerrainInteraction {
    /// Terrain collision pipeline
    collision_pipeline: ComputePipeline,
    
    /// Erosion simulation pipeline
    erosion_pipeline: ComputePipeline,
    
    /// Sediment transport pipeline
    sediment_pipeline: ComputePipeline,
    
    /// Bind group layout
    bind_group_layout: BindGroupLayout,
    
    /// Erosion parameters buffer
    erosion_params_buffer: Buffer,
    
    /// Sediment buffer
    sediment_buffer: Option<Buffer>,
    
    /// Device reference
    device: Arc<Device>,
}

impl TerrainInteraction {
    /// Create new terrain interaction system
    pub fn new(device: Arc<Device>) -> Self {
        let bind_group_layout = create_terrain_bind_group_layout(&device);
        
        let collision_pipeline = create_collision_pipeline(&device, &bind_group_layout);
        let erosion_pipeline = create_erosion_pipeline(&device, &bind_group_layout);
        let sediment_pipeline = create_sediment_pipeline(&device, &bind_group_layout);
        
        // Create erosion parameters buffer
        let erosion_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Erosion Parameters"),
            size: std::mem::size_of::<ErosionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            collision_pipeline,
            erosion_pipeline,
            sediment_pipeline,
            bind_group_layout,
            erosion_params_buffer,
            sediment_buffer: None,
            device,
        }
    }
    
    /// Initialize sediment buffer
    pub fn init_sediment_buffer(&mut self, size: (u32, u32, u32)) {
        let buffer_size = (size.0 * size.1 * size.2 * std::mem::size_of::<f32>() as u32) as u64;
        
        self.sediment_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sediment Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
    
    /// Update erosion parameters
    pub fn update_erosion_params(&self, queue: &wgpu::Queue, params: &ErosionParams) {
        queue.write_buffer(&self.erosion_params_buffer, 0, bytemuck::bytes_of(params));
    }
    
    /// Process terrain interactions
    pub fn update(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        fluid_buffer: &FluidBuffer,
        world_buffer: &WorldBuffer,
        bind_group: &BindGroup,
    ) {
        // Step 1: Terrain collision detection and response
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Terrain Collision Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.collision_pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            let workgroups = calculate_workgroups(fluid_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Step 2: Erosion simulation
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Erosion Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.erosion_pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            let workgroups = calculate_workgroups(fluid_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Step 3: Sediment transport
        if self.sediment_buffer.is_some() {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Sediment Transport Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.sediment_pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            let workgroups = calculate_workgroups(fluid_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
    }
    
    /// Get bind group layout
    pub fn get_bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }
}

/// Create bind group layout for terrain interaction
fn create_terrain_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Terrain Bind Group Layout"),
        entries: &[
            // Fluid buffer
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // World buffer (terrain voxels)
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
            // Erosion parameters
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
            // Sediment buffer
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
        ],
    })
}

/// Create terrain collision pipeline
fn create_collision_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Terrain Collision Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/terrain_collision.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Terrain Collision Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Terrain Collision Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "collision_main",
    })
}

/// Create erosion pipeline
fn create_erosion_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Erosion Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fluid_erosion.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Erosion Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Erosion Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "erosion_main",
    })
}

/// Create sediment transport pipeline
fn create_sediment_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Sediment Transport Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sediment_transport.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Sediment Transport Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Sediment Transport Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "sediment_main",
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

/// Terrain modification types from fluid interaction
#[derive(Debug, Clone, Copy)]
pub enum TerrainModification {
    /// Erosion removes terrain
    Erosion { amount: f32 },
    
    /// Deposition adds terrain
    Deposition { material: u16 },
    
    /// Transformation changes terrain type
    Transformation { from: u16, to: u16 },
}

/// Track erosion statistics
#[derive(Debug, Default, Clone)]
pub struct ErosionStats {
    pub total_eroded: f32,
    pub total_deposited: f32,
    pub active_sediment: f32,
    pub erosion_events: u64,
}