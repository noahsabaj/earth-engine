use std::sync::Arc;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use super::world_buffer::{WorldBuffer, VoxelData, CHUNK_SIZE};
// use crate::memory::PersistentBuffer; // Not needed anymore
use crate::world_gpu::error::{WorldGpuResult, WorldGpuErrorContext};

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
    
    /// Persistent count buffer to avoid allocations
    count_buffer: wgpu::Buffer,
    
    /// Pre-allocated bind groups for different scenarios
    cached_bind_groups: std::sync::Mutex<std::collections::HashMap<u64, wgpu::BindGroup>>,
    
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
        
        // Create persistent count buffer
        let count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Modification Count Buffer"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            device,
            modify_pipeline,
            explode_pipeline,
            command_buffer,
            command_capacity,
            count_buffer,
            cached_bind_groups: std::sync::Mutex::new(std::collections::HashMap::new()),
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
            let _ = self.apply_block_modifications(encoder, queue, world_buffer, &block_mods);
        }
        
        // Process explosions
        if !explosions.is_empty() {
            let _ = self.apply_explosions(encoder, queue, world_buffer, &explosions);
        }
    }
    
    fn apply_block_modifications(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        world_buffer: &WorldBuffer,
        commands: &[ModificationCommand],
    ) -> WorldGpuResult<()> {
        // Upload commands
        queue.write_buffer(
            &self.command_buffer,
            0,
            bytemuck::cast_slice(commands),
        );
        
        // Update count in persistent buffer
        let count_data = [commands.len() as u32];
        queue.write_buffer(&self.count_buffer, 0, bytemuck::cast_slice(&count_data));
        
        // Get or create cached bind group
        let cache_key = world_buffer.voxel_buffer() as *const _ as u64;
        let mut cached_groups = self.cached_bind_groups.lock().world_gpu_context("cached_bind_groups")?;
        
        let bind_group = cached_groups.entry(cache_key).or_insert_with(|| {
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Block Modification Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: world_buffer.voxel_buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.command_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.count_buffer.as_entire_binding(),
                    },
                ],
            })
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
        
        Ok(())
    }
    
    fn apply_explosions(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        world_buffer: &WorldBuffer,
        explosions: &[ModificationCommand],
    ) -> WorldGpuResult<()> {
        // Upload explosion commands
        queue.write_buffer(
            &self.command_buffer,
            0,
            bytemuck::cast_slice(explosions),
        );
        
        // Update count in persistent buffer
        let count_data = [explosions.len() as u32];
        queue.write_buffer(&self.count_buffer, 0, bytemuck::cast_slice(&count_data));
        
        // Use cached bind group (same as block modifications)
        let cache_key = world_buffer.voxel_buffer() as *const _ as u64;
        let cached_groups = self.cached_bind_groups.lock().world_gpu_context("cached_bind_groups")?;
        
        let bind_group = cached_groups.get(&cache_key).expect("Bind group should be cached from block modifications");
        
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
        
        Ok(())
    }
}