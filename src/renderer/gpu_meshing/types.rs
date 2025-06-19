//! GPU meshing data types - SOA layout for GPU efficiency
//! No methods, only data structures

use bytemuck::{Pod, Zeroable};

/// GPU mesh buffer - Structure of Arrays for cache efficiency
pub struct GpuMeshBuffer {
    /// Vertex positions (x, y, z interleaved)
    pub positions: wgpu::Buffer,
    /// Vertex normals (x, y, z interleaved)
    pub normals: wgpu::Buffer,
    /// Vertex UVs (u, v interleaved)
    pub uvs: wgpu::Buffer,
    /// Vertex colors (r, g, b, a interleaved)
    pub colors: wgpu::Buffer,
    /// Index buffer
    pub indices: wgpu::Buffer,
    /// Metadata buffer
    pub metadata: wgpu::Buffer,
    /// Buffer ID
    pub id: u32,
}

/// Mesh metadata for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuMeshMetadata {
    /// Chunk position
    pub chunk_pos: [i32; 3],
    /// Number of vertices
    pub vertex_count: u32,
    /// Number of indices
    pub index_count: u32,
    /// LOD level (0 = full detail)
    pub lod_level: u32,
    /// Mesh flags
    pub flags: u32,
    /// Generation timestamp
    pub timestamp: u32,
}

/// Indirect draw command for GPU-driven rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct IndirectDrawCommand {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

/// Mesh generation request
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct MeshRequest {
    /// Chunk position to mesh
    pub chunk_pos: [i32; 3],
    /// LOD level to generate
    pub lod_level: u32,
    /// Output buffer index
    pub buffer_index: u32,
    /// Mesh flags
    pub flags: u32,
    /// Padding
    pub _padding: [u32; 2],
}

/// Mesh generation parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct MeshingParams {
    /// Chunk size in voxels
    pub chunk_size: u32,
    /// Total request count
    pub request_count: u32,
    /// Enable greedy meshing
    pub enable_greedy: u32,
    /// Enable ambient occlusion
    pub enable_ao: u32,
    /// Maximum vertices per mesh
    pub max_vertices: u32,
    /// Maximum indices per mesh
    pub max_indices: u32,
    /// Padding
    pub _padding: [u32; 2],
}

/// Meshing statistics
#[derive(Default)]
pub struct MeshingStats {
    /// Total meshes generated
    pub total_meshes: u64,
    /// Total vertices generated
    pub total_vertices: u64,
    /// Total indices generated
    pub total_indices: u64,
    /// Average mesh generation time (microseconds)
    pub avg_generation_time: u32,
}

/// Face direction for culling
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum FaceDirection {
    PosX = 0,
    NegX = 1,
    PosY = 2,
    NegY = 3,
    PosZ = 4,
    NegZ = 5,
}

/// Vertex data for GPU (packed)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PackedVertex {
    /// Position and AO packed (x:10, y:10, z:10, ao:2)
    pub position_ao: u32,
    /// Normal and UV packed (nx:10, ny:10, u:6, v:6)
    pub normal_uv: u32,
    /// Color packed (r:8, g:8, b:8, light:8)
    pub color_light: u32,
}

/// Helper functions for data conversion (not methods!)

/// Pack position and AO into u32
pub fn pack_position_ao(x: f32, y: f32, z: f32, ao: f32) -> u32 {
    let x_bits = ((x * 1023.0) as u32) & 0x3FF;
    let y_bits = ((y * 1023.0) as u32) & 0x3FF;
    let z_bits = ((z * 1023.0) as u32) & 0x3FF;
    let ao_bits = ((ao * 3.0) as u32) & 0x3;
    
    (x_bits << 22) | (y_bits << 12) | (z_bits << 2) | ao_bits
}

/// Pack normal and UV into u32
pub fn pack_normal_uv(nx: f32, ny: f32, nz: f32, u: f32, v: f32) -> u32 {
    let nx_bits = (((nx + 1.0) * 511.5) as u32) & 0x3FF;
    let ny_bits = (((ny + 1.0) * 511.5) as u32) & 0x3FF;
    let u_bits = ((u * 63.0) as u32) & 0x3F;
    let v_bits = ((v * 63.0) as u32) & 0x3F;
    
    (nx_bits << 22) | (ny_bits << 12) | (u_bits << 6) | v_bits
}

/// Pack color and light into u32
pub fn pack_color_light(r: f32, g: f32, b: f32, light: f32) -> u32 {
    let r_bits = ((r * 255.0) as u32) & 0xFF;
    let g_bits = ((g * 255.0) as u32) & 0xFF;
    let b_bits = ((b * 255.0) as u32) & 0xFF;
    let light_bits = ((light * 255.0) as u32) & 0xFF;
    
    (r_bits << 24) | (g_bits << 16) | (b_bits << 8) | light_bits
}