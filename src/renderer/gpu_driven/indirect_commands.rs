use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

/// GPU indirect draw command structure
/// This matches wgpu's DrawIndirect command layout
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct IndirectDrawCommand {
    /// Number of vertices to draw
    pub vertex_count: u32,
    
    /// Number of instances to draw
    pub instance_count: u32,
    
    /// First vertex index
    pub first_vertex: u32,
    
    /// First instance index
    pub first_instance: u32,
}

impl IndirectDrawCommand {
    pub fn new(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }
}

/// GPU indirect draw indexed command structure
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct IndirectDrawIndexedCommand {
    /// Number of indices to draw
    pub index_count: u32,
    
    /// Number of instances to draw
    pub instance_count: u32,
    
    /// First index in the index buffer
    pub first_index: u32,
    
    /// Value added to each index
    pub base_vertex: i32,
    
    /// First instance index
    pub first_instance: u32,
}

/// Manages GPU buffers for indirect drawing commands
pub struct IndirectCommandBuffer {
    /// The GPU buffer storing commands
    buffer: wgpu::Buffer,
    
    /// Staging buffer for CPU updates
    staging_buffer: wgpu::Buffer,
    
    /// Maximum number of commands
    capacity: u32,
    
    /// Current number of active commands
    count: u32,
    
    /// Size of each command in bytes
    command_size: u32,
}

impl IndirectCommandBuffer {
    /// Create a new indirect command buffer
    pub fn new(
        device: &wgpu::Device,
        capacity: u32,
        indexed: bool,
    ) -> Self {
        let command_size = if indexed {
            std::mem::size_of::<IndirectDrawIndexedCommand>() as u32
        } else {
            std::mem::size_of::<IndirectDrawCommand>() as u32
        };
        
        let buffer_size = (command_size * capacity) as u64;
        
        // Create GPU buffer
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Indirect Command Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create staging buffer for updates
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Indirect Command Staging Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            mapped_at_creation: false,
        });
        
        Self {
            buffer,
            staging_buffer,
            capacity,
            count: 0,
            command_size,
        }
    }
    
    /// Update commands from CPU
    pub fn update_commands(
        &mut self,
        queue: &wgpu::Queue,
        commands: &[IndirectDrawCommand],
    ) {
        assert!(commands.len() as u32 <= self.capacity);
        
        // Write to staging buffer
        queue.write_buffer(
            &self.staging_buffer,
            0,
            bytemuck::cast_slice(commands),
        );
        
        self.count = commands.len() as u32;
    }
    
    /// Update indexed commands from CPU
    pub fn update_indexed_commands(
        &mut self,
        queue: &wgpu::Queue,
        commands: &[IndirectDrawIndexedCommand],
    ) {
        assert!(commands.len() as u32 <= self.capacity);
        
        // Write to staging buffer
        queue.write_buffer(
            &self.staging_buffer,
            0,
            bytemuck::cast_slice(commands),
        );
        
        self.count = commands.len() as u32;
    }
    
    /// Copy staging buffer to GPU buffer
    pub fn copy_to_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.count > 0 {
            encoder.copy_buffer_to_buffer(
                &self.staging_buffer,
                0,
                &self.buffer,
                0,
                (self.command_size * self.count) as u64,
            );
        }
    }
    
    /// Get the GPU buffer for binding
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    
    /// Get the number of active commands
    pub fn count(&self) -> u32 {
        self.count
    }
    
    /// Get buffer binding for compute shaders
    pub fn as_binding(&self) -> wgpu::BindingResource {
        self.buffer.as_entire_binding()
    }
}

/// Manages multiple indirect command buffers for different render passes
pub struct IndirectCommandManager {
    device: Arc<wgpu::Device>,
    
    /// Command buffers for different mesh types
    opaque_commands: IndirectCommandBuffer,
    transparent_commands: IndirectCommandBuffer,
    shadow_commands: IndirectCommandBuffer,
}

impl IndirectCommandManager {
    pub fn new(device: Arc<wgpu::Device>, max_draws_per_pass: u32) -> Self {
        Self {
            opaque_commands: IndirectCommandBuffer::new(&device, max_draws_per_pass, true),
            transparent_commands: IndirectCommandBuffer::new(&device, max_draws_per_pass, true),
            shadow_commands: IndirectCommandBuffer::new(&device, max_draws_per_pass, true),
            device,
        }
    }
    
    pub fn opaque_commands(&self) -> &IndirectCommandBuffer {
        &self.opaque_commands
    }
    
    pub fn opaque_commands_mut(&mut self) -> &mut IndirectCommandBuffer {
        &mut self.opaque_commands
    }
    
    pub fn transparent_commands(&self) -> &IndirectCommandBuffer {
        &self.transparent_commands
    }
    
    pub fn transparent_commands_mut(&mut self) -> &mut IndirectCommandBuffer {
        &mut self.transparent_commands
    }
    
    pub fn shadow_commands(&self) -> &IndirectCommandBuffer {
        &self.shadow_commands
    }
    
    pub fn shadow_commands_mut(&mut self) -> &mut IndirectCommandBuffer {
        &mut self.shadow_commands
    }
    
    /// Copy all staging buffers to GPU
    pub fn copy_all_to_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        self.opaque_commands.copy_to_gpu(encoder);
        self.transparent_commands.copy_to_gpu(encoder);
        self.shadow_commands.copy_to_gpu(encoder);
    }
}

/// Metadata for draw commands (used by compute shaders)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DrawMetadata {
    /// World-space bounding sphere (xyz = center, w = radius)
    pub bounding_sphere: [f32; 4],
    
    /// LOD information (x = min distance, y = max distance, z = LOD level, w = unused)
    pub lod_info: [f32; 4],
    
    /// Material properties (for sorting)
    pub material_id: u32,
    
    /// Mesh ID
    pub mesh_id: u32,
    
    /// Instance offset in instance buffer
    pub instance_offset: u32,
    
    /// Flags (visibility, shadows, etc.)
    pub flags: u32,
}