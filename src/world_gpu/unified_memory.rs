//! Unified memory management for GPU world systems
//! 
//! This module provides a safe API for managing a single large GPU buffer
//! that contains all world data. Instead of using dangerous lifetime transmutes,
//! we return buffer parameters that callers can use to create their own
//! buffer bindings with appropriate lifetimes.

use std::sync::Arc;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

/// Unified memory layout for all GPU world systems
/// This ensures all systems can access world data efficiently without copies
pub struct UnifiedMemoryLayout {
    /// Total world size in chunks
    pub world_size: u32,
    /// World height in voxels
    pub world_height: u32,
    /// Chunk size (32x32x32)
    pub chunk_size: u32,
    
    /// Offsets for different data regions in the unified buffer
    pub voxel_data_offset: u64,
    pub chunk_metadata_offset: u64,
    pub lighting_data_offset: u64,
    pub entity_data_offset: u64,
    pub particle_data_offset: u64,
    
    /// Sizes of each region
    pub voxel_data_size: u64,
    pub chunk_metadata_size: u64,
    pub lighting_data_size: u64,
    pub entity_data_size: u64,
    pub particle_data_size: u64,
    
    /// Total buffer size
    pub total_size: u64,
}

impl UnifiedMemoryLayout {
    pub fn new(world_size: u32, world_height: u32) -> Self {
        let chunk_size = 32u32;
        let chunks_per_dimension = world_size;
        let total_chunks = chunks_per_dimension * chunks_per_dimension * (world_height / chunk_size);
        let voxels_per_chunk = chunk_size * chunk_size * chunk_size;
        
        // Calculate region sizes
        let voxel_data_size = (total_chunks * voxels_per_chunk * 4) as u64; // 4 bytes per voxel
        let chunk_metadata_size = (total_chunks * 16) as u64; // 16 bytes per chunk metadata
        let lighting_data_size = (total_chunks * voxels_per_chunk) as u64; // 1 byte per voxel for propagated light
        let entity_data_size = 100 * 1024 * 1024; // 100MB for entities
        let particle_data_size = 50 * 1024 * 1024; // 50MB for particles
        
        // Calculate offsets (aligned to 256 bytes for GPU efficiency)
        let mut offset = 0u64;
        let voxel_data_offset = offset;
        offset += align_to(voxel_data_size, 256);
        
        let chunk_metadata_offset = offset;
        offset += align_to(chunk_metadata_size, 256);
        
        let lighting_data_offset = offset;
        offset += align_to(lighting_data_size, 256);
        
        let entity_data_offset = offset;
        offset += align_to(entity_data_size, 256);
        
        let particle_data_offset = offset;
        offset += align_to(particle_data_size, 256);
        
        let total_size = offset;
        
        Self {
            world_size,
            world_height,
            chunk_size,
            voxel_data_offset,
            chunk_metadata_offset,
            lighting_data_offset,
            entity_data_offset,
            particle_data_offset,
            voxel_data_size,
            chunk_metadata_size,
            lighting_data_size,
            entity_data_size,
            particle_data_size,
            total_size,
        }
    }
    
    /// Get the byte offset for a specific chunk's voxel data
    pub fn get_chunk_voxel_offset(&self, chunk_x: u32, chunk_y: u32, chunk_z: u32) -> u64 {
        let chunk_index = chunk_x + chunk_y * self.world_size + chunk_z * self.world_size * self.world_size;
        let voxels_per_chunk = self.chunk_size * self.chunk_size * self.chunk_size;
        self.voxel_data_offset + (chunk_index * voxels_per_chunk * 4) as u64
    }
    
    /// Get the byte offset for a specific chunk's metadata
    pub fn get_chunk_metadata_offset(&self, chunk_x: u32, chunk_y: u32, chunk_z: u32) -> u64 {
        let chunk_index = chunk_x + chunk_y * self.world_size + chunk_z * self.world_size * self.world_size;
        self.chunk_metadata_offset + (chunk_index * 16) as u64
    }
}

/// Manager for the unified GPU memory system
pub struct UnifiedMemoryManager {
    device: Arc<wgpu::Device>,
    layout: UnifiedMemoryLayout,
    
    /// The main unified buffer containing all world data
    pub unified_buffer: Arc<wgpu::Buffer>,
}

impl UnifiedMemoryManager {
    pub fn new(device: Arc<wgpu::Device>, world_size: u32, world_height: u32) -> Self {
        let layout = UnifiedMemoryLayout::new(world_size, world_height);
        
        // Create the unified buffer
        let unified_buffer = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Unified World Buffer"),
            size: layout.total_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        
        Self {
            device,
            layout,
            unified_buffer,
        }
    }
    
    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> MemoryStats {
        MemoryStats {
            total_allocated: self.layout.total_size,
            voxel_data: self.layout.voxel_data_size,
            chunk_metadata: self.layout.chunk_metadata_size,
            lighting_data: self.layout.lighting_data_size,
            entity_data: self.layout.entity_data_size,
            particle_data: self.layout.particle_data_size,
        }
    }
    
    /// Create buffer binding parameters for a specific region
    /// Returns (buffer_arc, offset, size) to be used when creating bind groups
    pub fn get_buffer_binding_params(&self, offset: u64, size: u64) -> (Arc<wgpu::Buffer>, u64, Option<wgpu::BufferSize>) {
        (self.unified_buffer.clone(), offset, wgpu::BufferSize::new(size))
    }
    
    /// Get buffer regions for a specific system
    /// Returns a list of (binding_index, offset, size) tuples
    pub fn get_system_buffer_regions(&self, system: SystemType) -> Vec<(u32, u64, u64)> {
        match system {
            SystemType::TerrainGeneration => vec![
                (0, self.layout.voxel_data_offset, self.layout.voxel_data_size),
                (1, self.layout.chunk_metadata_offset, self.layout.chunk_metadata_size),
            ],
            SystemType::Modification => vec![
                (0, self.layout.voxel_data_offset, self.layout.voxel_data_size),
            ],
            SystemType::Lighting => vec![
                (0, self.layout.voxel_data_offset, self.layout.voxel_data_size),
                (1, self.layout.lighting_data_offset, self.layout.lighting_data_size),
            ],
            SystemType::Rendering => vec![
                (0, self.layout.voxel_data_offset, self.layout.voxel_data_size),
                (1, self.layout.chunk_metadata_offset, self.layout.chunk_metadata_size),
                (2, self.layout.lighting_data_offset, self.layout.lighting_data_size),
            ],
            SystemType::Physics => vec![
                (0, self.layout.voxel_data_offset, self.layout.voxel_data_size),
                (1, self.layout.entity_data_offset, self.layout.entity_data_size),
            ],
            SystemType::Particles => vec![
                (0, self.layout.particle_data_offset, self.layout.particle_data_size),
            ],
        }
    }
    
    /// Helper method to create bind group entries with the unified buffer
    /// The caller must ensure the buffer reference remains valid for the bind group's lifetime
    pub fn create_bind_group_layout_entries(&self, system: SystemType) -> Vec<wgpu::BindGroupLayoutEntry> {
        let regions = self.get_system_buffer_regions(system);
        regions.iter().map(|(binding, _, _)| {
            wgpu::BindGroupLayoutEntry {
                binding: *binding,
                visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        }).collect()
    }
}

/// System types that access the unified memory
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemType {
    TerrainGeneration,
    Modification,
    Lighting,
    Rendering,
    Physics,
    Particles,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_allocated: u64,
    pub voxel_data: u64,
    pub chunk_metadata: u64,
    pub lighting_data: u64,
    pub entity_data: u64,
    pub particle_data: u64,
}

impl MemoryStats {
    pub fn print_summary(&self) {
        println!("=== GPU Memory Usage ===");
        println!("Total: {:.2} GB", self.total_allocated as f64 / (1024.0 * 1024.0 * 1024.0));
        println!("  Voxel Data: {:.2} GB", self.voxel_data as f64 / (1024.0 * 1024.0 * 1024.0));
        println!("  Chunk Metadata: {:.2} MB", self.chunk_metadata as f64 / (1024.0 * 1024.0));
        println!("  Lighting Data: {:.2} GB", self.lighting_data as f64 / (1024.0 * 1024.0 * 1024.0));
        println!("  Entity Data: {:.2} MB", self.entity_data as f64 / (1024.0 * 1024.0));
        println!("  Particle Data: {:.2} MB", self.particle_data as f64 / (1024.0 * 1024.0));
    }
}

/// Align a size to a boundary
fn align_to(size: u64, alignment: u64) -> u64 {
    (size + alignment - 1) & !(alignment - 1)
}