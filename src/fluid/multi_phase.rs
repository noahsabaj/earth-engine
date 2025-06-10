use wgpu::{Device, ComputePipeline, BindGroup, BindGroupLayout};
use crate::fluid::{FluidType, FluidBuffer};
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};

/// Phase information for multi-phase fluids
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FluidPhase {
    /// Primary fluid type
    pub primary_type: u32,
    
    /// Secondary fluid type (for mixing)
    pub secondary_type: u32,
    
    /// Mix ratio (0.0 = all primary, 1.0 = all secondary)
    pub mix_ratio: f32,
    
    /// Interface tension with other phases
    pub surface_tension: f32,
}

/// Multi-phase fluid system
pub struct PhaseSystem {
    /// Phase separation pipeline
    separation_pipeline: ComputePipeline,
    
    /// Interface reconstruction pipeline
    interface_pipeline: ComputePipeline,
    
    /// Phase mixing pipeline
    mixing_pipeline: ComputePipeline,
    
    /// Bind group layout
    bind_group_layout: BindGroupLayout,
    
    /// Phase properties buffer
    phase_properties: wgpu::Buffer,
    
    /// Device reference
    device: Arc<Device>,
}

impl PhaseSystem {
    /// Create new phase system
    pub fn new(device: Arc<Device>) -> Self {
        // Create phase properties buffer
        let phase_properties = create_phase_properties_buffer(&device);
        
        let bind_group_layout = create_phase_bind_group_layout(&device);
        
        let separation_pipeline = create_separation_pipeline(&device, &bind_group_layout);
        let interface_pipeline = create_interface_pipeline(&device, &bind_group_layout);
        let mixing_pipeline = create_mixing_pipeline(&device, &bind_group_layout);
        
        Self {
            separation_pipeline,
            interface_pipeline,
            mixing_pipeline,
            bind_group_layout,
            phase_properties,
            device,
        }
    }
    
    /// Update phase interactions
    pub fn update(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        fluid_buffer: &FluidBuffer,
        bind_group: &BindGroup,
    ) {
        // Step 1: Phase separation (immiscible fluids)
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Phase Separation Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.separation_pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            let workgroups = calculate_workgroups(fluid_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Step 2: Interface reconstruction
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Interface Reconstruction Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.interface_pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            
            let workgroups = calculate_workgroups(fluid_buffer.size);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        // Step 3: Phase mixing (miscible fluids)
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Phase Mixing Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.mixing_pipeline);
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

/// Phase interaction properties
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PhaseInteraction {
    /// Miscibility (0 = immiscible, 1 = fully miscible)
    pub miscibility: f32,
    
    /// Interface tension
    pub interface_tension: f32,
    
    /// Diffusion rate for miscible fluids
    pub diffusion_rate: f32,
    
    /// Temperature transfer rate
    pub heat_transfer: f32,
}

/// Phase properties for all fluid types
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PhaseProperties {
    /// Interaction matrix (6x6 for 6 fluid types)
    pub interactions: [[PhaseInteraction; 6]; 6],
}

impl Default for PhaseProperties {
    fn default() -> Self {
        let mut props = Self {
            interactions: [[PhaseInteraction {
                miscibility: 0.0,
                interface_tension: 0.0,
                diffusion_rate: 0.0,
                heat_transfer: 0.0,
            }; 6]; 6],
        };
        
        // Set up default interactions
        // Water-Air
        props.interactions[FluidType::Water as usize][FluidType::Air as usize] = PhaseInteraction {
            miscibility: 0.0,
            interface_tension: 0.072, // N/m
            diffusion_rate: 0.0,
            heat_transfer: 0.1,
        };
        
        // Water-Oil (immiscible)
        props.interactions[FluidType::Water as usize][FluidType::Oil as usize] = PhaseInteraction {
            miscibility: 0.0,
            interface_tension: 0.05,
            diffusion_rate: 0.0,
            heat_transfer: 0.05,
        };
        
        // Water-Lava (violent reaction)
        props.interactions[FluidType::Water as usize][FluidType::Lava as usize] = PhaseInteraction {
            miscibility: 0.0,
            interface_tension: 0.5,
            diffusion_rate: 0.0,
            heat_transfer: 10.0, // Rapid cooling/steam generation
        };
        
        // Make symmetric
        for i in 0..6 {
            for j in i+1..6 {
                props.interactions[j][i] = props.interactions[i][j];
            }
        }
        
        props
    }
}

/// Create phase properties buffer
fn create_phase_properties_buffer(device: &Device) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    
    let properties = PhaseProperties::default();
    
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Phase Properties Buffer"),
        contents: bytemuck::bytes_of(&properties),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}

/// Create bind group layout for phase system
fn create_phase_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Phase Bind Group Layout"),
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
            // Phase properties
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
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

/// Create phase separation pipeline
fn create_separation_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Phase Separation Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/phase_separation.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Phase Separation Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Phase Separation Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "separation_main",
    })
}

/// Create interface reconstruction pipeline
fn create_interface_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Interface Reconstruction Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/phase_interface.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Interface Reconstruction Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Interface Reconstruction Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "interface_main",
    })
}

/// Create phase mixing pipeline
fn create_mixing_pipeline(device: &Device, layout: &BindGroupLayout) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Phase Mixing Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/phase_mixing.wgsl").into()),
    });
    
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Phase Mixing Layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Phase Mixing Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "mixing_main",
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

/// Special reactions between fluid types
pub enum FluidReaction {
    /// Water + Lava = Steam + Obsidian
    WaterLava,
    
    /// Oil + Fire = Smoke + Fire spread
    OilFire,
    
    /// Steam condensation
    SteamCondensation,
}

impl FluidReaction {
    /// Check if two fluid types react
    pub fn check_reaction(type1: FluidType, type2: FluidType) -> Option<FluidReaction> {
        match (type1, type2) {
            (FluidType::Water, FluidType::Lava) | (FluidType::Lava, FluidType::Water) => {
                Some(FluidReaction::WaterLava)
            }
            (FluidType::Oil, FluidType::Lava) | (FluidType::Lava, FluidType::Oil) => {
                Some(FluidReaction::OilFire)
            }
            _ => None,
        }
    }
}