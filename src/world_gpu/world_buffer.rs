use std::sync::Arc;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use crate::morton::MortonEncoder3D;

/// Maximum world size in chunks per dimension
pub const MAX_WORLD_SIZE: u32 = 512; // 512³ chunks = 134M chunks max
pub const CHUNK_SIZE: u32 = 32;
pub const VOXELS_PER_CHUNK: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// Packed voxel data format for GPU storage
/// Uses 32 bits per voxel:
/// - Bits 0-15: Block ID (64K block types)
/// - Bits 16-19: Light level (0-15)
/// - Bits 20-23: Sky light level (0-15)
/// - Bits 24-27: Metadata (flags, rotation, etc)
/// - Bits 28-31: Reserved
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VoxelData(pub u32);

impl VoxelData {
    pub const AIR: Self = Self(0);
    
    #[inline]
    pub fn new(block_id: u16, light: u8, sky_light: u8, metadata: u8) -> Self {
        let packed = (block_id as u32) 
            | ((light as u32 & 0xF) << 16)
            | ((sky_light as u32 & 0xF) << 20)
            | ((metadata as u32 & 0xF) << 24);
        Self(packed)
    }
    
    #[inline]
    pub fn block_id(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
    
    #[inline]
    pub fn light_level(&self) -> u8 {
        ((self.0 >> 16) & 0xF) as u8
    }
    
    #[inline]
    pub fn sky_light_level(&self) -> u8 {
        ((self.0 >> 20) & 0xF) as u8
    }
    
    #[inline]
    pub fn metadata(&self) -> u8 {
        ((self.0 >> 24) & 0xF) as u8
    }
}

/// Descriptor for creating a WorldBuffer
pub struct WorldBufferDescriptor {
    /// Size of the world in chunks per dimension
    pub world_size: u32,
    /// Enable atomic operations for modifications
    pub enable_atomics: bool,
    /// Enable readback for debugging
    pub enable_readback: bool,
}

impl Default for WorldBufferDescriptor {
    fn default() -> Self {
        Self {
            world_size: 256, // 256³ chunks by default
            enable_atomics: true,
            enable_readback: cfg!(debug_assertions),
        }
    }
}

/// GPU-resident world buffer containing all voxel data
pub struct WorldBuffer {
    device: Arc<wgpu::Device>,
    
    /// Main voxel storage buffer
    voxel_buffer: wgpu::Buffer,
    
    /// Chunk metadata buffer (loaded/generated flags, timestamps, etc)
    metadata_buffer: wgpu::Buffer,
    
    /// Staging buffer for CPU->GPU uploads (if needed)
    staging_buffer: Option<wgpu::Buffer>,
    
    /// Bind group for compute shaders
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    
    /// World dimensions
    world_size: u32,
    total_voxels: u64,
}

impl WorldBuffer {
    pub fn new(device: Arc<wgpu::Device>, desc: &WorldBufferDescriptor) -> Self {
        let world_size = desc.world_size;
        let chunks_total = world_size * world_size * world_size;
        let total_voxels = chunks_total as u64 * VOXELS_PER_CHUNK as u64;
        let buffer_size = total_voxels * std::mem::size_of::<VoxelData>() as u64;
        
        // Main voxel buffer
        let mut usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        if desc.enable_readback {
            usage |= wgpu::BufferUsages::COPY_SRC;
        }
        
        let voxel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("World Voxel Buffer"),
            size: buffer_size,
            usage,
            mapped_at_creation: false,
        });
        
        // Chunk metadata buffer
        let metadata_size = chunks_total as u64 * 16; // 16 bytes per chunk
        let metadata_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Metadata Buffer"),
            size: metadata_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Optional staging buffer for uploads
        let staging_buffer = if desc.enable_readback {
            Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("World Staging Buffer"),
                size: VOXELS_PER_CHUNK as u64 * std::mem::size_of::<VoxelData>() as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }))
        } else {
            None
        };
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("World Buffer Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("World Buffer Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: voxel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: metadata_buffer.as_entire_binding(),
                },
            ],
        });
        
        Self {
            device,
            voxel_buffer,
            metadata_buffer,
            staging_buffer,
            bind_group,
            bind_group_layout,
            world_size,
            total_voxels,
        }
    }
    
    /// Get the bind group for use in compute/render passes
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
    
    /// Get the bind group layout for pipeline creation
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
    
    /// Get the voxel buffer (for custom bind groups)
    pub fn voxel_buffer(&self) -> &wgpu::Buffer {
        &self.voxel_buffer
    }
    
    /// Get the metadata buffer (for custom bind groups)
    pub fn metadata_buffer(&self) -> &wgpu::Buffer {
        &self.metadata_buffer
    }
    
    /// Get the world size
    pub fn world_size(&self) -> u32 {
        self.world_size
    }
    
    /// Calculate buffer offset for a chunk position using Morton encoding
    pub fn chunk_offset(&self, chunk_x: u32, chunk_y: u32, chunk_z: u32) -> u64 {
        let morton_encoder = MortonEncoder3D::new();
        let chunk_morton = morton_encoder.encode(chunk_x, chunk_y, chunk_z);
        chunk_morton * VOXELS_PER_CHUNK as u64 * std::mem::size_of::<VoxelData>() as u64
    }
    
    /// Upload a single chunk from CPU (migration path)
    pub fn upload_chunk(&self, queue: &wgpu::Queue, chunk_pos: [u32; 3], voxels: &[VoxelData]) {
        assert_eq!(voxels.len(), VOXELS_PER_CHUNK as usize);
        
        let offset = self.chunk_offset(chunk_pos[0], chunk_pos[1], chunk_pos[2]);
        queue.write_buffer(&self.voxel_buffer, offset, bytemuck::cast_slice(voxels));
    }
    
    /// Clear a chunk to air
    pub fn clear_chunk(&self, encoder: &mut wgpu::CommandEncoder, chunk_pos: [u32; 3]) {
        let offset = self.chunk_offset(chunk_pos[0], chunk_pos[1], chunk_pos[2]);
        let size = VOXELS_PER_CHUNK as u64 * std::mem::size_of::<VoxelData>() as u64;
        
        encoder.clear_buffer(&self.voxel_buffer, offset, Some(size));
    }
    
    
    /// Get total voxel count
    pub fn total_voxels(&self) -> u64 {
        self.total_voxels
    }
    
    /// Get buffer size in bytes
    pub fn buffer_size(&self) -> u64 {
        self.total_voxels * std::mem::size_of::<VoxelData>() as u64
    }
}