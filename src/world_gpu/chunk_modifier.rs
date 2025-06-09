use std::sync::Arc;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use super::world_buffer::{WorldBuffer, VoxelData, CHUNK_SIZE};

/// Command for modifying voxels
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ModificationCommand {
    /// World position to modify
    pub position: [i32; 3],
    /// New block ID (u32 for alignment)
    pub block_id: u32,
    /// Modification type (0=set, 1=break, 2=explode)
    pub mod_type: u32,
    /// Radius for area effects (explosions)
    pub radius: f32,
    /// Padding for alignment
    pub _padding: [u32; 2],
}

impl ModificationCommand {
    pub fn set_block(x: i32, y: i32, z: i32, block_id: u16) -> Self {
        Self {
            position: [x, y, z],
            block_id: block_id as u32,
            mod_type: 0,
            radius: 0.0,
            _padding: [0; 2],
        }
    }
    
    pub fn break_block(x: i32, y: i32, z: i32) -> Self {
        Self {
            position: [x, y, z],
            block_id: 0,
            mod_type: 1,
            radius: 0.0,
            _padding: [0; 2],
        }
    }
    
    pub fn explode(x: i32, y: i32, z: i32, radius: f32) -> Self {
        Self {
            position: [x, y, z],
            block_id: 0,
            mod_type: 2,
            radius,
            _padding: [0; 2],
        }
    }
}

/// GPU-based chunk modification system
pub struct ChunkModifier {
    device: Arc<wgpu::Device>,
    
    /// Pipeline for single block modifications
    modify_pipeline: wgpu::ComputePipeline,
    
    /// Pipeline for explosion effects
    explode_pipeline: wgpu::ComputePipeline,
    
    /// Command buffer for batching modifications
    command_buffer: wgpu::Buffer,
    command_capacity: usize,
    
    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ChunkModifier {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Chunk Modification Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/chunk_modification.wgsl").into()),
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Chunk Modifier Bind Group Layout"),
            entries: &[
                // World buffer
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
                // Modification commands
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
                // Command count
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
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Chunk Modifier Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create pipelines
        let modify_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Block Modification Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "modify_blocks",
        });
        
        let explode_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Explosion Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "explode_blocks",
        });
        
        // Create command buffer
        let command_capacity = 10000;
        let command_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Modification Commands Buffer"),
            size: (command_capacity * std::mem::size_of::<ModificationCommand>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            device,
            modify_pipeline,
            explode_pipeline,
            command_buffer,
            command_capacity,
            bind_group_layout,
        }
    }
    
    /// Apply a batch of modifications to the world
    pub fn apply_modifications(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        world_buffer: &WorldBuffer,
        commands: &[ModificationCommand],
    ) {
        if commands.is_empty() {
            return;
        }
        
        // Split commands by type
        let mut block_mods = Vec::new();
        let mut explosions = Vec::new();
        
        for cmd in commands {
            match cmd.mod_type {
                0 | 1 => block_mods.push(*cmd),
                2 => explosions.push(*cmd),
                _ => {}
            }
        }
        
        // Process block modifications
        if !block_mods.is_empty() {
            self.apply_block_modifications(encoder, queue, world_buffer, &block_mods);
        }
        
        // Process explosions
        if !explosions.is_empty() {
            self.apply_explosions(encoder, queue, world_buffer, &explosions);
        }
    }
    
    fn apply_block_modifications(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        world_buffer: &WorldBuffer,
        commands: &[ModificationCommand],
    ) {
        // Upload commands
        queue.write_buffer(
            &self.command_buffer,
            0,
            bytemuck::cast_slice(commands),
        );
        
        // Create count buffer
        let count_data = [commands.len() as u32];
        let count_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Command Count Buffer"),
            contents: bytemuck::cast_slice(&count_data),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Block Modification Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: world_buffer.voxel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.command_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: count_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Dispatch compute shader
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Block Modification Pass"),
            timestamp_writes: None,
        });
        
        compute_pass.set_pipeline(&self.modify_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        
        // One thread per command
        let workgroups = ((commands.len() + 63) / 64) as u32;
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }
    
    fn apply_explosions(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        world_buffer: &WorldBuffer,
        explosions: &[ModificationCommand],
    ) {
        // Upload explosion commands
        queue.write_buffer(
            &self.command_buffer,
            0,
            bytemuck::cast_slice(explosions),
        );
        
        // Create count buffer
        let count_data = [explosions.len() as u32];
        let count_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Explosion Count Buffer"),
            contents: bytemuck::cast_slice(&count_data),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Explosion Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: world_buffer.voxel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.command_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: count_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Dispatch compute shader
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Explosion Pass"),
            timestamp_writes: None,
        });
        
        compute_pass.set_pipeline(&self.explode_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        
        // Process explosions with enough threads to cover blast radius
        // Each explosion uses multiple workgroups
        compute_pass.dispatch_workgroups(
            explosions.len() as u32 * 8, // 8 workgroups per explosion
            8,
            8,
        );
    }
}