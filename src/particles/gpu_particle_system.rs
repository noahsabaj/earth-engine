use std::sync::Arc;
use std::time::Duration;
use wgpu::util::DeviceExt;
use glam::Vec3;
use anyhow::{Result, anyhow};

use crate::particles::{ParticleType, ParticleGPUData};
use crate::particles::particle_data::MAX_PARTICLES;

/// GPU-accelerated particle system
/// Offloads all particle updates to GPU compute shaders
pub struct GpuParticleSystem {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    
    // GPU buffers
    particle_buffer: wgpu::Buffer,
    emitter_buffer: wgpu::Buffer,
    spawn_queue_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
    
    // Compute pipelines
    update_pipeline: wgpu::ComputePipeline,
    spawn_pipeline: wgpu::ComputePipeline,
    force_pipeline: wgpu::ComputePipeline,
    
    // Bind groups
    update_bind_group: wgpu::BindGroup,
    spawn_bind_group: wgpu::BindGroup,
    
    // CPU-side data for rendering
    render_data: Vec<ParticleGPUData>,
    staging_buffer: wgpu::Buffer,
    
    // System state
    max_particles: u32,
    active_particles: u32,
    emitter_count: u32,
    next_emitter_id: u64,
    
    // Physics parameters
    wind_velocity: Vec3,
    gravity: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SimParams {
    dt: f32,
    time: f32,
    wind_velocity: [f32; 3],
    gravity: f32,
    particle_count: u32,
    _padding: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuParticleData {
    position: [f32; 3],
    size: f32,
    velocity: [f32; 3],
    lifetime: f32,
    acceleration: [f32; 3],
    max_lifetime: f32,
    color: [f32; 4],
    gravity_multiplier: f32,
    drag: f32,
    bounce: f32,
    rotation: f32,
    rotation_speed: f32,
    particle_type: u32,
    texture_frame: u32,
    size_curve_type: u32,
    color_curve_type: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuEmitterData {
    position: [f32; 3],
    emission_rate: f32,
    base_velocity: [f32; 3],
    velocity_variance: f32,
    particle_type: u32,
    shape_type: u32,
    shape_param1: f32,
    shape_param2: f32,
}

impl GpuParticleSystem {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        max_particles: usize,
    ) -> Self {
        let max_particles = (max_particles as u32).min(MAX_PARTICLES as u32);
        
        // Create GPU buffers
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: (std::mem::size_of::<GpuParticleData>() * max_particles as usize) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let emitter_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Emitter Buffer"),
            size: (std::mem::size_of::<GpuEmitterData>() * 128) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let spawn_queue_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spawn Queue Buffer"),
            size: (std::mem::size_of::<GpuParticleData>() * 1024) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Params Buffer"),
            size: std::mem::size_of::<SimParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Staging Buffer"),
            size: (std::mem::size_of::<GpuParticleData>() * max_particles as usize) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Update Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_update.wgsl").into()),
        });
        
        // Create bind group layouts
        let update_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Particle Update Bind Group Layout"),
            entries: &[
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
            ],
        });
        
        let spawn_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Particle Spawn Bind Group Layout"),
            entries: &[
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
            ],
        });
        
        // Create compute pipelines
        let update_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Particle Update Pipeline Layout"),
            bind_group_layouts: &[&update_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let update_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Particle Update Pipeline"),
            layout: Some(&update_pipeline_layout),
            module: &shader,
            entry_point: "update_particles",
        });
        
        let spawn_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Particle Spawn Pipeline Layout"),
            bind_group_layouts: &[&update_bind_group_layout, &spawn_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let spawn_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Particle Spawn Pipeline"),
            layout: Some(&spawn_pipeline_layout),
            module: &shader,
            entry_point: "spawn_from_emitters",
        });
        
        let force_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Particle Force Pipeline"),
            layout: Some(&update_pipeline_layout),
            module: &shader,
            entry_point: "apply_force_field",
        });
        
        // Create bind groups
        let update_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Update Bind Group"),
            layout: &update_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Create spawn counter buffer
        let spawn_count_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spawn Count Buffer"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let spawn_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Particle Spawn Bind Group"),
            layout: &spawn_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: emitter_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: spawn_queue_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: spawn_count_buffer.as_entire_binding(),
                },
            ],
        });
        
        Self {
            device,
            queue,
            particle_buffer,
            emitter_buffer,
            spawn_queue_buffer,
            params_buffer,
            update_pipeline,
            spawn_pipeline,
            force_pipeline,
            update_bind_group,
            spawn_bind_group,
            render_data: Vec::with_capacity(max_particles as usize),
            staging_buffer,
            max_particles,
            active_particles: 0,
            emitter_count: 0,
            next_emitter_id: 0,
            wind_velocity: Vec3::ZERO,
            gravity: 9.8,
        }
    }
    
    /// Update the particle system on GPU
    pub fn update(&mut self, dt: Duration, time: f32) {
        let dt_secs = dt.as_secs_f32();
        
        // Update simulation parameters
        let params = SimParams {
            dt: dt_secs,
            time,
            wind_velocity: self.wind_velocity.into(),
            gravity: self.gravity,
            particle_count: self.active_particles,
            _padding: [0.0; 3],
        };
        
        self.queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        
        // Create command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Particle Update Encoder"),
        });
        
        // Spawn new particles from emitters
        if self.emitter_count > 0 {
            let mut spawn_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Particle Spawn Pass"),
                timestamp_writes: None,
            });
            spawn_pass.set_pipeline(&self.spawn_pipeline);
            spawn_pass.set_bind_group(0, &self.update_bind_group, &[]);
            spawn_pass.set_bind_group(1, &self.spawn_bind_group, &[]);
            spawn_pass.dispatch_workgroups((self.emitter_count + 31) / 32, 1, 1);
        }
        
        // Update existing particles
        {
            let mut update_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Particle Update Pass"),
                timestamp_writes: None,
            });
            update_pass.set_pipeline(&self.update_pipeline);
            update_pass.set_bind_group(0, &self.update_bind_group, &[]);
            let workgroups = (self.active_particles + 63) / 64;
            update_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        // Copy particle data to staging buffer for CPU readback
        encoder.copy_buffer_to_buffer(
            &self.particle_buffer,
            0,
            &self.staging_buffer,
            0,
            (std::mem::size_of::<GpuParticleData>() * self.active_particles as usize) as u64,
        );
        
        // Submit GPU commands
        self.queue.submit(std::iter::once(encoder.finish()));
    }
    
    /// Add an emitter
    pub fn add_emitter(
        &mut self,
        position: Vec3,
        particle_type: ParticleType,
        emission_rate: f32,
    ) -> u64 {
        let id = self.next_emitter_id;
        self.next_emitter_id += 1;
        
        let emitter = GpuEmitterData {
            position: position.into(),
            emission_rate,
            base_velocity: [0.0, 0.0, 0.0],
            velocity_variance: 0.1,
            particle_type: particle_type.to_id(),
            shape_type: 0, // Point
            shape_param1: 0.0,
            shape_param2: 0.0,
        };
        
        // Upload emitter to GPU
        self.queue.write_buffer(
            &self.emitter_buffer,
            (std::mem::size_of::<GpuEmitterData>() * self.emitter_count as usize) as u64,
            bytemuck::cast_slice(&[emitter]),
        );
        
        self.emitter_count += 1;
        id
    }
    
    /// Read back particle data for rendering
    pub async fn read_render_data(&mut self) -> Result<&[ParticleGPUData]> {
        // Map staging buffer and read data
        let buffer_slice = self.staging_buffer.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            if let Err(_) = tx.send(result) {
                // Channel receiver was dropped - this is expected in some shutdown scenarios
            }
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        
        let map_result = rx.await
            .map_err(|_| anyhow!("Failed to receive GPU buffer mapping result - channel was closed"))?;
        map_result.map_err(|e| anyhow!("Failed to map GPU buffer for particle data reading: {:?}", e))?;
        
        {
            let data = buffer_slice.get_mapped_range();
            let gpu_particles: &[GpuParticleData] = bytemuck::cast_slice(&data);
            
            // Convert to render format
            self.render_data.clear();
            for particle in &gpu_particles[..self.active_particles as usize] {
                if particle.lifetime > 0.0 {
                    self.render_data.push(ParticleGPUData {
                        position: particle.position,
                        size: particle.size,
                        color: particle.color,
                        rotation: particle.rotation,
                        texture_index: particle.texture_frame,
                        _padding: [0.0, 0.0],
                    });
                }
            }
        }
        
        self.staging_buffer.unmap();
        Ok(&self.render_data)
    }
    
    /// Get particle count
    pub fn particle_count(&self) -> usize {
        self.active_particles as usize
    }
    
    /// Set wind velocity
    pub fn set_wind(&mut self, velocity: Vec3) {
        self.wind_velocity = velocity;
    }
    
    /// Set gravity
    pub fn set_gravity(&mut self, gravity: f32) {
        self.gravity = gravity;
    }
}