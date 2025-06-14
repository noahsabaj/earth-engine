use std::sync::Arc;
use std::collections::HashMap;
use bytemuck::{Pod, Zeroable};
use crate::morton::morton_encode;
use crate::world::ChunkPos;

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
    /// View distance in chunks (determines buffer size)
    pub view_distance: u32,
    /// Enable atomic operations for modifications
    pub enable_atomics: bool,
    /// Enable readback for debugging
    pub enable_readback: bool,
}

impl Default for WorldBufferDescriptor {
    fn default() -> Self {
        Self {
            // Use view distance to determine buffer size (safe for GPU limits)
            view_distance: 3, // Conservative: 7³=343 chunks, ~45MB (safe for 128MB GPU limit)
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
    
    /// Maximum chunks that can be loaded (based on view distance)
    max_chunks: u32,
    /// View distance in chunks
    view_distance: u32,
    total_voxels: u64,
    
    /// Chunk slot management: maps chunk position to buffer slot index
    chunk_slots: HashMap<ChunkPos, u32>,
    /// Next available slot (simple round-robin allocation)
    next_slot: u32,
}

impl WorldBuffer {
    pub fn new(device: Arc<wgpu::Device>, desc: &WorldBufferDescriptor) -> Self {
        let view_distance = desc.view_distance;
        
        // Calculate maximum chunks based on view distance
        // Use sphere approximation: chunks within view_distance radius
        // Conservative estimate: (2 * view_distance + 1)³ to ensure we have enough space
        let diameter = 2 * view_distance + 1;
        let max_chunks = diameter * diameter * diameter;
        
        // Safety check: prevent massive allocations
        let gb = max_chunks as u64 * VOXELS_PER_CHUNK as u64 * 4 / (1024 * 1024 * 1024);
        if gb > 4 {
            panic!("WorldBuffer: view_distance {} would require {} GB GPU memory (max 4GB recommended)", 
                   view_distance, gb);
        }
        
        log::info!("Creating WorldBuffer with view_distance {} ({} max chunks, {} MB)", 
                  view_distance, max_chunks, 
                  max_chunks as u64 * VOXELS_PER_CHUNK as u64 * 4 / (1024 * 1024));
        
        let total_voxels = max_chunks as u64 * VOXELS_PER_CHUNK as u64;
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
        let metadata_size = max_chunks as u64 * 16; // 16 bytes per chunk
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
            max_chunks,
            view_distance,
            total_voxels,
            chunk_slots: HashMap::new(),
            next_slot: 0,
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
    
    /// Get the view distance
    pub fn view_distance(&self) -> u32 {
        self.view_distance
    }
    
    /// Get the maximum chunks this buffer can hold
    pub fn max_chunks(&self) -> u32 {
        self.max_chunks
    }
    
    /// Get or allocate a buffer slot for a chunk position
    pub fn get_chunk_slot(&mut self, chunk_pos: ChunkPos) -> u32 {
        if let Some(&slot) = self.chunk_slots.get(&chunk_pos) {
            slot
        } else {
            // Allocate new slot
            let slot = self.next_slot % self.max_chunks;
            
            // If slot is occupied, remove the old chunk mapping
            let old_chunk = self.chunk_slots.iter()
                .find_map(|(pos, &s)| if s == slot { Some(*pos) } else { None });
            if let Some(old_pos) = old_chunk {
                self.chunk_slots.remove(&old_pos);
            }
            
            // Map new chunk to slot
            self.chunk_slots.insert(chunk_pos, slot);
            self.next_slot = (self.next_slot + 1) % self.max_chunks;
            
            slot
        }
    }
    
    /// Calculate buffer offset for a chunk slot
    pub fn slot_offset(&self, slot: u32) -> u64 {
        slot as u64 * VOXELS_PER_CHUNK as u64 * std::mem::size_of::<VoxelData>() as u64
    }
    
    /// Upload a single chunk from CPU (migration path)
    pub fn upload_chunk(&mut self, queue: &wgpu::Queue, chunk_pos: ChunkPos, voxels: &[VoxelData]) {
        assert_eq!(voxels.len(), VOXELS_PER_CHUNK as usize);
        
        let slot = self.get_chunk_slot(chunk_pos);
        let offset = self.slot_offset(slot);
        queue.write_buffer(&self.voxel_buffer, offset, bytemuck::cast_slice(voxels));
    }
    
    /// Clear a chunk to air
    pub fn clear_chunk(&mut self, encoder: &mut wgpu::CommandEncoder, chunk_pos: ChunkPos) {
        let slot = self.get_chunk_slot(chunk_pos);
        let offset = self.slot_offset(slot);
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