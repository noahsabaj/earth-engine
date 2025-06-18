use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use crate::gpu::buffer_layouts::{
    calculations,
    usage,
    constants::INSTANCE_DATA_SIZE,
};

// Re-export InstanceData for public use
pub use crate::gpu::buffer_layouts::InstanceData;

/// Manages GPU buffers for instance data
pub struct InstanceBuffer {
    /// The GPU buffer storing instance data
    buffer: wgpu::Buffer,
    
    /// CPU-side copy of instance data for updates
    instances: Vec<InstanceData>,
    
    /// Maximum number of instances
    capacity: u32,
    
    /// Current number of active instances
    count: u32,
    
    /// Dirty flag for updates
    dirty: bool,
    
    /// Reference to device that created this buffer
    device: Arc<wgpu::Device>,
}

impl InstanceBuffer {
    /// Create a new instance buffer
    pub fn new(device: Arc<wgpu::Device>, capacity: u32) -> Self {
        let buffer_size = calculations::instance_buffer_size(capacity);
        
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: buffer_size,
            usage: usage::VERTEX | usage::STORAGE,
            mapped_at_creation: false,
        });
        
        Self {
            buffer,
            instances: Vec::with_capacity(capacity as usize),
            capacity,
            count: 0,
            dirty: false,
            device,
        }
    }
    
    /// Add an instance
    pub fn add_instance(&mut self, instance: InstanceData) -> Option<u32> {
        if self.count >= self.capacity {
            return None;
        }
        
        let index = self.count;
        self.instances.push(instance);
        self.count += 1;
        self.dirty = true;
        
        Some(index)
    }
    
    /// Add multiple instances in batch (DOP compliant)
    pub fn add_instances_batch(&mut self, instances: &[InstanceData]) -> Vec<Option<u32>> {
        let mut indices = Vec::with_capacity(instances.len());
        let available_space = (self.capacity - self.count) as usize;
        let instances_to_add = instances.len().min(available_space);
        
        if instances_to_add > 0 {
            let start_index = self.count;
            self.instances.extend_from_slice(&instances[..instances_to_add]);
            self.count += instances_to_add as u32;
            self.dirty = true;
            
            // Generate indices for successfully added instances
            for i in 0..instances_to_add {
                indices.push(Some(start_index + i as u32));
            }
            
            // Mark remaining instances as rejected
            for _ in instances_to_add..instances.len() {
                indices.push(None);
            }
        } else {
            // All instances rejected
            indices.resize(instances.len(), None);
        }
        
        indices
    }
    
    /// Update an instance
    pub fn update_instance(&mut self, index: u32, instance: InstanceData) {
        if index < self.count {
            if let Some(inst) = self.instances.get_mut(index as usize) {
                *inst = instance;
                self.dirty = true;
            }
        }
    }
    
    /// Remove an instance (swap with last)
    pub fn remove_instance(&mut self, index: u32) -> Option<u32> {
        if index >= self.count {
            return None;
        }
        
        self.count -= 1;
        
        // Swap with last instance
        if index < self.count {
            self.instances.swap(index as usize, self.count as usize);
            self.dirty = true;
            Some(self.count) // Return the index that was moved
        } else {
            self.instances.pop();
            self.dirty = true;
            None
        }
    }
    
    /// Clear all instances
    pub fn clear(&mut self) {
        self.instances.clear();
        self.count = 0;
        self.dirty = true;
    }
    
    /// Upload dirty data to GPU
    pub fn upload_to_gpu(&mut self, queue: &wgpu::Queue) {
        if self.dirty && self.count > 0 {
            queue.write_buffer(
                &self.buffer,
                0,
                bytemuck::cast_slice(&self.instances[..self.count as usize]),
            );
            self.dirty = false;
        }
    }
    
    /// Get the GPU buffer
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    
    /// Get the number of active instances
    pub fn count(&self) -> u32 {
        self.count
    }
    
    /// Get vertex buffer layout for instances
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // Model matrix - 4 vec4s
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Custom data
                wgpu::VertexAttribute {
                    offset: 80,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Manages multiple instance buffers for different object types
pub struct InstanceManager {
    device: Arc<wgpu::Device>,
    
    /// Instance buffers by mesh type
    chunk_instances: InstanceBuffer,
    entity_instances: InstanceBuffer,
    particle_instances: InstanceBuffer,
}

impl InstanceManager {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        Self {
            chunk_instances: InstanceBuffer::new(device.clone(), 100000),
            entity_instances: InstanceBuffer::new(device.clone(), 50000),
            particle_instances: InstanceBuffer::new(device.clone(), 100000),
            device,
        }
    }
    
    pub fn chunk_instances(&self) -> &InstanceBuffer {
        &self.chunk_instances
    }
    
    pub fn chunk_instances_mut(&mut self) -> &mut InstanceBuffer {
        &mut self.chunk_instances
    }
    
    pub fn entity_instances(&self) -> &InstanceBuffer {
        &self.entity_instances
    }
    
    pub fn entity_instances_mut(&mut self) -> &mut InstanceBuffer {
        &mut self.entity_instances
    }
    
    pub fn particle_instances(&self) -> &InstanceBuffer {
        &self.particle_instances
    }
    
    pub fn particle_instances_mut(&mut self) -> &mut InstanceBuffer {
        &mut self.particle_instances
    }
    
    /// Upload all dirty buffers to GPU
    pub fn upload_all(&mut self, queue: &wgpu::Queue) {
        self.chunk_instances.upload_to_gpu(queue);
        self.entity_instances.upload_to_gpu(queue);
        self.particle_instances.upload_to_gpu(queue);
    }
    
    /// Clear all instance buffers
    pub fn clear_all(&mut self) {
        self.chunk_instances.clear();
        self.entity_instances.clear();
        self.particle_instances.clear();
        log::debug!(
            "[InstanceManager::clear_all] Cleared all instance buffers - Chunk: {}, Entity: {}, Particle: {}",
            self.chunk_instances.count(),
            self.entity_instances.count(),
            self.particle_instances.count()
        );
    }
}

/// Compact instance data for GPU culling (smaller than full InstanceData)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CullingInstanceData {
    /// World position
    pub position: [f32; 3],
    
    /// Bounding radius
    pub radius: f32,
    
    /// Instance ID (index into full instance buffer)
    pub instance_id: u32,
    
    /// Flags and metadata
    pub flags: u32,
    
    /// Padding for alignment
    pub _padding: [u32; 2],
}

impl CullingInstanceData {
    pub fn from_instance(instance: &InstanceData, radius: f32, id: u32) -> Self {
        // Extract position from model matrix
        let position = [
            instance.model_matrix[3][0],
            instance.model_matrix[3][1],
            instance.model_matrix[3][2],
        ];
        
        Self {
            position,
            radius,
            instance_id: id,
            flags: 0,
            _padding: [0; 2],
        }
    }
}