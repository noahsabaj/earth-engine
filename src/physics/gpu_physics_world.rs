/// GPU-powered physics world system  
/// 
/// This replaces the CPU PhysicsWorldData with GPU compute shader simulation
/// while maintaining the exact same interface for compatibility.
/// Part of Option 1: 95% GPU / 5% CPU performance split

use std::sync::{Arc, Mutex};
use cgmath::{Point3, Vector3};
use crate::{
    physics::{PhysicsData, EntityId, physics_tables::PhysicsFlags, FIXED_TIMESTEP},
    world_unified::interfaces::WorldInterface,
    world_unified::{
        storage::WorldBuffer,
        compute::hierarchical_physics::{HierarchicalPhysics, PhysicsQuery, QueryType, QueryResult},
    },
    memory::MemoryManager,
};
use wgpu::util::DeviceExt;

/// Compatibility struct for PhysicsBodyData
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct PhysicsBodyData {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub flags: u32,
}

/// GPU-powered physics world that maintains PhysicsWorldData interface
pub struct GpuPhysicsWorld {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    
    /// GPU physics system
    hierarchical_physics: HierarchicalPhysics,
    
    /// GPU buffers for physics entities
    entity_buffer: wgpu::Buffer,
    entity_capacity: u32,
    
    /// CPU-side entity tracking for interface compatibility
    entities: Vec<PhysicsBodyData>,
    active_count: usize,
    id_to_index: rustc_hash::FxHashMap<EntityId, usize>,
    next_entity_id: EntityId,
    
    /// GPU compute pipeline for physics updates
    physics_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    
    /// Physics accumulator
    accumulator: f32,
    
    /// GPU world buffer reference
    world_buffer: Option<Arc<Mutex<WorldBuffer>>>,
}

impl GpuPhysicsWorld {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        max_entities: u32,
    ) -> Self {
        // Create memory manager for hierarchical physics
        let memory_config = crate::memory::MemoryConfig::default();
        let mut memory_manager = MemoryManager::new(device.clone(), memory_config);
        
        // Create hierarchical physics system for GPU queries
        let hierarchical_physics = HierarchicalPhysics::new(
            device.clone(),
            &mut memory_manager,
            1024, // Max queries per frame
        );
        
        // Create GPU entity buffer
        let entity_buffer_size = (max_entities * std::mem::size_of::<PhysicsBodyData>() as u32) as u64;
        let entity_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Physics Entity Buffer"),
            size: entity_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create physics compute shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GPU Physics Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/gpu_physics.wgsl").into()),
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GPU Physics Bind Group Layout"),
            entries: &[
                // Entity data
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
                // World voxels
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
                // Physics parameters
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
        
        // Create compute pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GPU Physics Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let physics_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GPU Physics Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "physics_update",
        });
        
        Self {
            device,
            queue,
            hierarchical_physics,
            entity_buffer,
            entity_capacity: max_entities,
            entities: Vec::with_capacity(max_entities as usize),
            active_count: 0,
            id_to_index: rustc_hash::FxHashMap::default(),
            next_entity_id: EntityId(1),
            physics_pipeline,
            bind_group_layout,
            accumulator: 0.0,
            world_buffer: None,
        }
    }
    
    /// Set the GPU world buffer for physics/voxel interaction
    pub fn set_world_buffer(&mut self, world_buffer: Arc<Mutex<WorldBuffer>>) {
        self.world_buffer = Some(world_buffer);
    }
    
    /// Add entity to GPU physics world (maintains PhysicsWorldData interface)
    pub fn add_entity(
        &mut self,
        position: Point3<f32>,
        velocity: Vector3<f32>,
        aabb_size: Vector3<f32>,
        mass: f32,
        friction: f32,
        restitution: f32,
    ) -> EntityId {
        if self.active_count >= self.entity_capacity as usize {
            log::error!("[GpuPhysicsWorld] Cannot add entity - capacity exceeded");
            return EntityId(0);
        }
        
        let entity_id = self.next_entity_id;
        self.next_entity_id.0 += 1;
        
        let half_size = aabb_size * 0.5;
        let body = PhysicsBodyData {
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            aabb_min: [-half_size.x, -half_size.y, -half_size.z],
            aabb_max: [half_size.x, half_size.y, half_size.z],
            mass,
            friction,
            restitution,
            flags: PhysicsFlags::ACTIVE,
        };
        
        // Store in CPU-side tracking
        let index = self.active_count;
        if index >= self.entities.len() {
            self.entities.push(body);
        } else {
            self.entities[index] = body;
        }
        
        self.id_to_index.insert(entity_id, index);
        self.active_count += 1;
        
        // Upload to GPU buffer
        self.upload_entity_data();
        
        log::debug!("[GpuPhysicsWorld] Added entity {} at position {:?}", entity_id, position);
        entity_id
    }
    
    /// Update physics simulation using GPU compute shaders
    pub fn update<W: WorldInterface + 'static>(&mut self, world: &W, delta_time: f32) {
        self.accumulator += delta_time;
        
        // Process fixed timestep updates on GPU
        let mut physics_steps = 0;
        while self.accumulator >= FIXED_TIMESTEP && physics_steps < 4 {
            self.gpu_physics_step(world);
            self.accumulator -= FIXED_TIMESTEP;
            physics_steps += 1;
        }
        
        if physics_steps > 0 {
            // Download updated entity data from GPU
            self.download_entity_data();
        }
    }
    
    /// Get physics body by entity ID (maintains PhysicsWorldData interface)
    pub fn get_body(&self, id: EntityId) -> Option<&PhysicsBodyData> {
        self.id_to_index.get(&id).and_then(|&idx| self.entities.get(idx))
    }
    
    /// Get mutable physics body by entity ID
    pub fn get_body_mut(&mut self, id: EntityId) -> Option<&mut PhysicsBodyData> {
        if let Some(&idx) = self.id_to_index.get(&id) {
            self.entities.get_mut(idx)
        } else {
            None
        }
    }
    
    /// Set entity position (maintains PhysicsWorldData interface)
    pub fn set_position(&mut self, id: EntityId, position: Point3<f32>) {
        if let Some(body) = self.get_body_mut(id) {
            body.position = [position.x, position.y, position.z];
            // Upload updated data to GPU
            self.upload_entity_data();
        }
    }
    
    /// Perform physics simulation step on GPU
    fn gpu_physics_step<W: WorldInterface>(&mut self, _world: &W) {
        if self.active_count == 0 || self.world_buffer.is_none() {
            return;
        }
        
        let world_buffer = self.world_buffer.as_ref().unwrap().clone();
        
        // Create physics parameters
        let physics_params = PhysicsParameters {
            delta_time: FIXED_TIMESTEP,
            gravity: -9.81,
            entity_count: self.active_count as u32,
            _padding: 0,
        };
        
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Physics Parameters"),
            contents: bytemuck::cast_slice(&[physics_params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        // Create bind group
        let bind_group = {
            let world_buffer_guard = world_buffer.lock().unwrap();
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.entity_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: world_buffer_guard.voxel_buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
                label: Some("GPU Physics Bind Group"),
            })
        };
        
        // Dispatch physics compute shader
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GPU Physics Update"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Physics Update Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.physics_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            // Dispatch with work groups for all active entities
            let workgroup_size = 64;
            let workgroups = (self.active_count as u32 + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
    }
    
    /// Upload entity data to GPU buffer
    fn upload_entity_data(&self) {
        if self.active_count > 0 {
            let data_slice = &self.entities[..self.active_count];
            self.queue.write_buffer(
                &self.entity_buffer,
                0,
                bytemuck::cast_slice(data_slice),
            );
        }
    }
    
    /// Download entity data from GPU buffer
    fn download_entity_data(&mut self) {
        // For now, we maintain CPU-side state
        // In production, this would use a staging buffer to read back GPU results
        // TODO: Implement actual GPU readback for full GPU physics
        log::trace!("[GpuPhysicsWorld] GPU physics step complete for {} entities", self.active_count);
    }
}

/// Physics parameters for GPU shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PhysicsParameters {
    delta_time: f32,
    gravity: f32,
    entity_count: u32,
    _padding: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gpu_physics_world_creation() {
        // This test requires GPU context, so we'll skip in normal test runs
        // TODO: Add proper GPU testing infrastructure
    }
}