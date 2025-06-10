use wgpu::{Device, Buffer};
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};

/// SDF value representing distance to nearest surface
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SdfValue {
    /// Signed distance to surface (negative = inside, positive = outside)
    pub distance: f32,
    
    /// Material ID of nearest voxel
    pub material: u16,
    
    /// Gradient magnitude (for edge detection)
    pub gradient_mag: u16,
}

impl SdfValue {
    pub fn empty() -> Self {
        Self {
            distance: SDF_MAX_DISTANCE,
            material: 0,
            gradient_mag: 0,
        }
    }
}

/// GPU buffer for SDF data
pub struct SdfBuffer {
    /// Current SDF values
    pub buffer: Option<Buffer>,
    
    /// Size of SDF grid (includes margins)
    pub size: (u32, u32, u32),
    
    /// Offset in world space
    pub world_offset: (i32, i32, i32),
    
    /// Device reference
    device: Arc<Device>,
}

impl SdfBuffer {
    /// Create new SDF buffer
    pub fn new(device: Arc<Device>, voxel_size: (u32, u32, u32)) -> Self {
        // Calculate SDF size with margins
        let sdf_factor = (1.0 / SDF_RESOLUTION_FACTOR) as u32;
        let size = (
            (voxel_size.0 + 2 * SDF_MARGIN) * sdf_factor,
            (voxel_size.1 + 2 * SDF_MARGIN) * sdf_factor,
            (voxel_size.2 + 2 * SDF_MARGIN) * sdf_factor,
        );
        
        let buffer_size = (size.0 * size.1 * size.2) as u64 * std::mem::size_of::<SdfValue>() as u64;
        
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SDF Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        Self {
            buffer: Some(buffer),
            size,
            world_offset: (0, 0, 0),
            device,
        }
    }
    
    /// Get buffer size in bytes
    pub fn size_bytes(&self) -> u64 {
        (self.size.0 * self.size.1 * self.size.2) as u64 * std::mem::size_of::<SdfValue>() as u64
    }
    
    /// Create staging buffer for CPU readback
    pub fn create_staging_buffer(&self) -> Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SDF Staging Buffer"),
            size: self.size_bytes(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}

/// SDF chunk data for spatial organization
pub struct SdfChunk {
    /// Position in chunk grid
    pub position: (i32, i32, i32),
    
    /// SDF buffer for this chunk
    pub sdf_buffer: SdfBuffer,
    
    /// Flags
    pub dirty: bool,
    pub has_surface: bool,
    
    /// Cached mesh (if generated)
    pub mesh_vertices: Option<Buffer>,
    pub mesh_indices: Option<Buffer>,
    pub vertex_count: u32,
    pub index_count: u32,
}

impl SdfChunk {
    /// Create new SDF chunk
    pub fn new(device: Arc<Device>, position: (i32, i32, i32), voxel_size: (u32, u32, u32)) -> Self {
        let mut sdf_buffer = SdfBuffer::new(device, voxel_size);
        sdf_buffer.world_offset = (
            position.0 * voxel_size.0 as i32,
            position.1 * voxel_size.1 as i32,
            position.2 * voxel_size.2 as i32,
        );
        
        Self {
            position,
            sdf_buffer,
            dirty: true,
            has_surface: false,
            mesh_vertices: None,
            mesh_indices: None,
            vertex_count: 0,
            index_count: 0,
        }
    }
    
    /// Mark chunk as needing update
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    
    /// Clear cached mesh
    pub fn clear_mesh(&mut self) {
        self.mesh_vertices = None;
        self.mesh_indices = None;
        self.vertex_count = 0;
        self.index_count = 0;
    }
}

/// SDF generation constants
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SdfConstants {
    /// Resolution factor (SDF cells per voxel)
    pub resolution_factor: f32,
    
    /// Maximum distance to propagate
    pub max_distance: f32,
    
    /// Surface threshold for marching cubes
    pub surface_threshold: f32,
    
    /// Smoothing factor for distance field
    pub smoothing_factor: f32,
    
    /// Voxel size in world units
    pub voxel_size: f32,
    
    /// Padding
    pub _padding: [f32; 3],
}

impl Default for SdfConstants {
    fn default() -> Self {
        Self {
            resolution_factor: 1.0 / SDF_RESOLUTION_FACTOR,
            max_distance: SDF_MAX_DISTANCE,
            surface_threshold: SDF_SURFACE_THRESHOLD,
            smoothing_factor: 0.5,
            voxel_size: 1.0,
            _padding: [0.0; 3],
        }
    }
}

/// Vertex format for smooth mesh
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SmoothVertex {
    /// Position in world space
    pub position: [f32; 3],
    
    /// Surface normal
    pub normal: [f32; 3],
    
    /// Material blend weights (up to 4 materials)
    pub material_weights: [f32; 4],
    
    /// Material IDs
    pub material_ids: [u16; 4],
}

use super::SDF_MAX_DISTANCE;
use super::SDF_RESOLUTION_FACTOR;
use super::SDF_MARGIN;
use super::SDF_SURFACE_THRESHOLD;