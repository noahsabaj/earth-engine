use std::sync::Arc;
use bytemuck::{Pod, Zeroable};

/// Weather data stored on GPU per chunk or region
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WeatherData {
    /// Packed weather type and intensity (0-7 type, 8-15 intensity)
    pub weather_type_intensity: u32,
    /// Temperature in Celsius * 10 (allows -3276.8 to 3276.7°C)
    pub temperature: i16,
    /// Humidity percentage * 100 (0-10000 for 0-100%)
    pub humidity: u16,
    /// Wind speed in m/s * 10
    pub wind_speed: u16,
    /// Wind direction in degrees (0-359)
    pub wind_direction: u16,
    /// Visibility factor * 1000 (0-1000 for 0.0-1.0)
    pub visibility: u16,
    /// Precipitation rate * 1000
    pub precipitation_rate: u16,
}

impl WeatherData {
    pub fn clear() -> Self {
        Self {
            weather_type_intensity: 0, // Clear weather, no intensity
            temperature: 200, // 20.0°C
            humidity: 5000, // 50%
            wind_speed: 50, // 5.0 m/s
            wind_direction: 0,
            visibility: 1000, // 1.0 (full)
            precipitation_rate: 0,
        }
    }
}

/// Transition data for smooth weather changes
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WeatherTransition {
    /// Current weather data
    pub current: WeatherData,
    /// Target weather data
    pub target: WeatherData,
    /// Transition progress (0-65535 for 0.0-1.0)
    pub progress: u16,
    /// Transition speed per frame
    pub speed: u16,
    /// Time until next weather change (in frames)
    pub change_timer: u32,
    /// Current biome type
    pub biome_type: u32,
}

/// Precipitation particle data for GPU simulation
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PrecipitationParticle {
    /// Position in world space
    pub position: [f32; 3],
    /// Particle type (0=rain, 1=snow, 2=sleet, 3=hail)
    pub particle_type: u32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Lifetime remaining (0-1)
    pub lifetime: f32,
    /// Size of particle
    pub size: f32,
    /// Reserved for alignment
    pub _padding: [f32; 3],
}

/// Configuration for weather compute shader
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WeatherConfig {
    /// Current frame number
    pub frame_number: u32,
    /// Delta time in milliseconds
    pub delta_time_ms: u32,
    /// Player position for precipitation bounds
    pub player_position: [f32; 3],
    /// Precipitation spawn radius
    pub precipitation_radius: f32,
    /// Maximum precipitation particles
    pub max_particles: u32,
    /// Current particle count
    pub particle_count: u32,
    /// Random seed for this frame
    pub random_seed: u32,
    /// System flags (enable/disable features)
    pub flags: u32,
}

pub struct WeatherGpuDescriptor {
    /// Number of weather regions (chunks or larger areas)
    pub region_count: u32,
    /// Maximum precipitation particles
    pub max_particles: u32,
    /// Enable particle simulation
    pub enable_particles: bool,
}

impl Default for WeatherGpuDescriptor {
    fn default() -> Self {
        Self {
            region_count: 4096, // 16x16x16 regions
            max_particles: 100000,
            enable_particles: true,
        }
    }
}

/// GPU-based weather system
pub struct WeatherGpu {
    device: Arc<wgpu::Device>,
    
    /// Weather data buffer (per region)
    weather_buffer: wgpu::Buffer,
    
    /// Weather transition buffer
    transition_buffer: wgpu::Buffer,
    
    /// Precipitation particle buffer
    particle_buffer: wgpu::Buffer,
    
    /// Configuration uniform buffer
    config_buffer: wgpu::Buffer,
    
    /// Compute pipeline for weather updates
    compute_pipeline: wgpu::ComputePipeline,
    
    /// Bind group
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    
    /// Descriptor
    desc: WeatherGpuDescriptor,
}

impl WeatherGpu {
    pub fn new(device: Arc<wgpu::Device>, desc: WeatherGpuDescriptor) -> Self {
        // Create weather data buffer
        let weather_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weather Data Buffer"),
            size: (desc.region_count as u64) * std::mem::size_of::<WeatherData>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create transition buffer
        let transition_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weather Transition Buffer"),
            size: (desc.region_count as u64) * std::mem::size_of::<WeatherTransition>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create particle buffer
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Precipitation Particle Buffer"),
            size: (desc.max_particles as u64) * std::mem::size_of::<PrecipitationParticle>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        
        // Create config buffer
        let config_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weather Config Buffer"),
            size: std::mem::size_of::<WeatherConfig>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Weather Bind Group Layout"),
            entries: &[
                // Weather data
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Transition data
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
                // Particle data
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Config
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
        });
        
        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Weather Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: weather_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: transition_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: config_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Load compute shader
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Weather Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/weather_compute.wgsl").into()),
        });
        
        // Create compute pipeline
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Weather Compute Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Weather Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            })),
            module: &shader_module,
            entry_point: "main",
        });
        
        Self {
            device,
            weather_buffer,
            transition_buffer,
            particle_buffer,
            config_buffer,
            compute_pipeline,
            bind_group,
            bind_group_layout,
            desc,
        }
    }
    
    /// Update weather configuration
    pub fn update_config(&self, queue: &wgpu::Queue, config: &WeatherConfig) {
        queue.write_buffer(&self.config_buffer, 0, bytemuck::cast_slice(&[*config]));
    }
    
    /// Run weather update compute pass
    pub fn update(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Weather Update Pass"),
            timestamp_writes: None,
        });
        
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        
        // Dispatch for weather transitions (one workgroup per 64 regions)
        let region_workgroups = (self.desc.region_count + 63) / 64;
        compute_pass.dispatch_workgroups(region_workgroups, 1, 1);
        
        // Dispatch for particle updates (one workgroup per 256 particles)
        if self.desc.enable_particles {
            let particle_workgroups = (self.desc.max_particles + 255) / 256;
            compute_pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }
    }
    
    /// Get bind group for rendering
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
    
    /// Get particle buffer for rendering
    pub fn particle_buffer(&self) -> &wgpu::Buffer {
        &self.particle_buffer
    }
}

#[cfg(test)]
#[path = "weather_gpu_tests.rs"]
mod tests;