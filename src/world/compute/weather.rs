use crate::gpu::error_recovery::{GpuErrorRecovery, GpuRecoveryError, GpuResultExt};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

// Include constants from root constants.rs
include!("../../../constants.rs");

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
            temperature: 200,          // 20.0°C
            humidity: 5000,            // 50%
            wind_speed: 50,            // 5.0 m/s
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
    pub target_weather: WeatherData,
    /// Transition progress (0-65535 for 0.0-1.0)
    pub progress: u16,
    /// Transition speed per frame
    pub speed: u16,
    /// Time until next weather change (in frames)
    pub time_remaining: u32,
}

/// Individual precipitation particle
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PrecipitationParticle {
    /// World position
    pub position: [f32; 3],
    /// Particle type (0=rain, 1=snow, 2=hail, etc.)
    pub particle_type: u32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Time to live (frames)
    pub ttl: u32,
}

/// Weather configuration for GPU compute
#[derive(Clone, Debug)]
pub struct WeatherConfig {
    /// Number of weather regions
    pub region_count: u32,
    /// Size of each weather region in chunks
    pub region_size: u32,
    /// Maximum particles per region
    pub max_particles_per_region: u32,
    /// Weather update frequency (frames)
    pub update_frequency: u32,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            region_count: 256,
            region_size: 8, // 8x8 chunks per region
            max_particles_per_region: 10000,
            update_frequency: 60, // Update once per second at 60 FPS
        }
    }
}

/// GPU-based weather system
pub struct WeatherGpu {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: WeatherConfig,

    /// Weather data buffer
    weather_buffer: wgpu::Buffer,
    /// Particle buffer
    particle_buffer: wgpu::Buffer,
    /// Unified weather/particle compute pipeline
    compute_pipeline: wgpu::ComputePipeline,

    bind_group_layout: wgpu::BindGroupLayout,

    /// Error recovery
    error_recovery: Arc<GpuErrorRecovery>,
}

/// Weather GPU descriptor
pub struct WeatherGpuDescriptor {
    pub config: WeatherConfig,
}

impl WeatherGpu {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        desc: WeatherGpuDescriptor,
    ) -> Self {
        let config = desc.config;

        // Create error recovery system
        let error_recovery = Arc::new(GpuErrorRecovery::new(device.clone(), queue.clone()));

        // Create weather data buffer
        let weather_buffer_size =
            std::mem::size_of::<WeatherTransition>() * config.region_count as usize;
        let weather_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weather Data Buffer"),
            size: weather_buffer_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create particle buffer
        let particle_buffer_size = std::mem::size_of::<PrecipitationParticle>()
            * config.region_count as usize
            * config.max_particles_per_region as usize;
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: particle_buffer_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // Create shader through unified GPU system (Single Source of Truth)
        let shader_source = include_str!("../../shaders/compute/weather_compute.wgsl");
        let validated_shader =
            crate::gpu::automation::create_gpu_shader(&device, "weather_compute", shader_source)
                .expect("Failed to create weather shader through unified GPU system");

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Weather Bind Group Layout"),
            entries: &[
                // Weather data
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
                // Particles
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Weather Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::COMPUTE,
                range: 0..16, // frame_time, delta_time, seed, flags
            }],
        });

        // Create unified weather/particle pipeline (shader only has 'main' entry point)
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Weather Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &validated_shader.module,
            entry_point: "main",
        });

        Self {
            device,
            queue,
            config,
            weather_buffer,
            particle_buffer,
            compute_pipeline,
            bind_group_layout,
            error_recovery,
        }
    }

    /// Update weather simulation with error recovery
    pub fn update(&self, frame_time: f32, delta_time: f32) -> Result<(), GpuRecoveryError> {
        // Create safe command encoder
        let mut safe_encoder =
            self.error_recovery
                .create_safe_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Weather Update Encoder"),
                });

        let encoder = safe_encoder.encoder()?;
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Weather Update Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.weather_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.particle_buffer.as_entire_binding(),
                },
            ],
        });

        // Push constants
        let push_constants = [
            frame_time.to_bits(),
            delta_time.to_bits(),
            rand::random::<u32>(), // Random seed
            0,                     // Flags
        ];

        // Execute weather and particle updates with error recovery
        self.error_recovery.execute_with_recovery(|| {
            // Update weather
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Weather Update Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                compute_pass.set_push_constants(0, bytemuck::cast_slice(&push_constants));

                let workgroups = (self.config.region_count + 63) / 64;
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            }

            // Update particles
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Particle Update Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                compute_pass.set_push_constants(0, bytemuck::cast_slice(&push_constants));

                let total_particles =
                    self.config.region_count * self.config.max_particles_per_region;
                let workgroups = (total_particles + gpu_limits::MAX_WORKGROUP_SIZE - 1)
                    / gpu_limits::MAX_WORKGROUP_SIZE;
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            }

            Ok(())
        })?;

        // Submit commands with error recovery
        let command_buffer = safe_encoder.finish()?;
        self.error_recovery
            .submit_with_recovery(vec![command_buffer])?;

        Ok(())
    }

    /// Get particle buffer for rendering
    pub fn particle_buffer(&self) -> &wgpu::Buffer {
        &self.particle_buffer
    }

    /// Get weather buffer for reading current conditions
    pub fn weather_buffer(&self) -> &wgpu::Buffer {
        &self.weather_buffer
    }
}
