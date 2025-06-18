//! Mesh buffer layout definitions
//! 
//! Defines vertex and index buffer structures for mesh rendering.

use bytemuck::{Pod, Zeroable};
use wgpu::{VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

/// Standard vertex format for mesh rendering
/// Total size: 32 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// Position in model space
    pub position: [f32; 3],
    
    /// Normal vector (normalized)
    pub normal: [f32; 3],
    
    /// Texture coordinates
    pub tex_coords: [f32; 2],
}

impl Vertex {
    /// Create a new vertex
    pub fn new(position: [f32; 3], normal: [f32; 3], tex_coords: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            tex_coords,
        }
    }
    
    /// Get vertex buffer layout descriptor
    pub fn layout() -> VertexBufferLayout<'static> {
        const ATTRIBUTES: &[VertexAttribute] = &[
            // Position
            VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            },
            // Normal
            VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: VertexFormat::Float32x3,
            },
            // Texture coordinates
            VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: VertexFormat::Float32x2,
            },
        ];
        
        VertexBufferLayout {
            array_stride: 32,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

/// Extended vertex format with tangent space
/// Total size: 48 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VertexExtended {
    /// Position in model space
    pub position: [f32; 3],
    
    /// Normal vector (normalized)
    pub normal: [f32; 3],
    
    /// Tangent vector (normalized)
    pub tangent: [f32; 3],
    
    /// Bitangent sign (-1 or 1)
    pub bitangent_sign: f32,
    
    /// Texture coordinates
    pub tex_coords: [f32; 2],
}

impl VertexExtended {
    /// Get vertex buffer layout descriptor
    pub fn layout() -> VertexBufferLayout<'static> {
        const ATTRIBUTES: &[VertexAttribute] = &[
            // Position
            VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            },
            // Normal
            VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: VertexFormat::Float32x3,
            },
            // Tangent
            VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: VertexFormat::Float32x3,
            },
            // Bitangent sign
            VertexAttribute {
                offset: 36,
                shader_location: 3,
                format: VertexFormat::Float32,
            },
            // Texture coordinates
            VertexAttribute {
                offset: 40,
                shader_location: 4,
                format: VertexFormat::Float32x2,
            },
        ];
        
        VertexBufferLayout {
            array_stride: 48,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

/// Vertex format for terrain/voxel rendering (compressed)
/// Total size: 16 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TerrainVertex {
    /// Packed position (3 bytes) + AO (1 byte)
    pub position_ao: u32,
    
    /// Packed normal (2 bytes) + texture ID (2 bytes)
    pub normal_tex: u32,
    
    /// Texture coordinates (16-bit fixed point)
    pub tex_coords: [u16; 2],
}

impl TerrainVertex {
    /// Pack position and ambient occlusion
    pub fn pack_position_ao(x: u8, y: u8, z: u8, ao: u8) -> u32 {
        (x as u32) | ((y as u32) << 8) | ((z as u32) << 16) | ((ao as u32) << 24)
    }
    
    /// Pack normal and texture ID
    pub fn pack_normal_tex(normal_index: u8, tex_id: u16) -> u32 {
        (normal_index as u32) | ((tex_id as u32) << 16)
    }
}

/// Structure of Arrays vertex format for better GPU cache utilization
/// Used for large mesh batches
#[repr(C)]
#[derive(Clone, Debug)]
pub struct VertexSOA {
    /// All X positions
    pub positions_x: Vec<f32>,
    /// All Y positions
    pub positions_y: Vec<f32>,
    /// All Z positions
    pub positions_z: Vec<f32>,
    
    /// All X normals
    pub normals_x: Vec<f32>,
    /// All Y normals
    pub normals_y: Vec<f32>,
    /// All Z normals
    pub normals_z: Vec<f32>,
    
    /// All U texture coordinates
    pub tex_coords_u: Vec<f32>,
    /// All V texture coordinates
    pub tex_coords_v: Vec<f32>,
}

impl VertexSOA {
    /// Create empty SOA vertex buffer
    pub fn new() -> Self {
        Self {
            positions_x: Vec::new(),
            positions_y: Vec::new(),
            positions_z: Vec::new(),
            normals_x: Vec::new(),
            normals_y: Vec::new(),
            normals_z: Vec::new(),
            tex_coords_u: Vec::new(),
            tex_coords_v: Vec::new(),
        }
    }
    
    /// Get vertex count
    pub fn len(&self) -> usize {
        self.positions_x.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.positions_x.is_empty()
    }
    
    /// Add vertex from AOS format
    pub fn push_vertex(&mut self, vertex: &Vertex) {
        self.positions_x.push(vertex.position[0]);
        self.positions_y.push(vertex.position[1]);
        self.positions_z.push(vertex.position[2]);
        self.normals_x.push(vertex.normal[0]);
        self.normals_y.push(vertex.normal[1]);
        self.normals_z.push(vertex.normal[2]);
        self.tex_coords_u.push(vertex.tex_coords[0]);
        self.tex_coords_v.push(vertex.tex_coords[1]);
    }
}

/// Mesh buffer layout information
pub struct MeshBufferLayout;

impl MeshBufferLayout {
    /// Standard vertex size
    pub const VERTEX_SIZE: u64 = 32;
    
    /// Extended vertex size
    pub const VERTEX_EXTENDED_SIZE: u64 = 48;
    
    /// Terrain vertex size
    pub const TERRAIN_VERTEX_SIZE: u64 = 16;
    
    /// Index size (u32)
    pub const INDEX_SIZE: u64 = 4;
    
    /// Calculate vertex buffer size
    #[inline]
    pub fn vertex_buffer_size(vertex_count: u32) -> u64 {
        vertex_count as u64 * Self::VERTEX_SIZE
    }
    
    /// Calculate index buffer size
    #[inline]
    pub fn index_buffer_size(index_count: u32) -> u64 {
        index_count as u64 * Self::INDEX_SIZE
    }
}