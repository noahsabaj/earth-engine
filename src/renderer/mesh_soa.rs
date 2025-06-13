use super::vertex_soa::{VertexBufferSoA, VertexBufferStats};
use wgpu::util::DeviceExt;

/// Mesh using Struct-of-Arrays for better cache efficiency
pub struct MeshSoA {
    pub vertices: VertexBufferSoA,
    pub indices: Vec<u32>,
    pub index_buffer: Option<wgpu::Buffer>,
}

impl MeshSoA {
    pub fn new() -> Self {
        Self {
            vertices: VertexBufferSoA::new(),
            indices: Vec::new(),
            index_buffer: None,
        }
    }
    
    /// Clear the mesh data
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.index_buffer = None;
    }
    
    /// Add a quad (two triangles) to the mesh
    pub fn add_quad(
        &mut self,
        positions: [[f32; 3]; 4],
        color: [f32; 3],
        normal: [f32; 3],
        light: f32,
        ao: [f32; 4], // AO for each vertex
    ) {
        let base_index = self.vertices.len() as u32;
        
        // Add vertices
        for i in 0..4 {
            let ao_value = match ao.get(i) {
                Some(&value) => value,
                None => {
                    log::warn!("AO value index {} out of bounds, using default", i);
                    1.0
                }
            };
            let position = match positions.get(i) {
                Some(&pos) => pos,
                None => {
                    log::warn!("Position index {} out of bounds, using origin", i);
                    [0.0, 0.0, 0.0]
                }
            };
            self.vertices.push(position, color, normal, light, ao_value);
        }
        
        // Add indices for two triangles
        self.indices.extend_from_slice(&[
            base_index, base_index + 1, base_index + 2,
            base_index, base_index + 2, base_index + 3,
        ]);
    }
    
    /// Upload mesh data to GPU
    pub fn upload(&mut self, device: &wgpu::Device) {
        if self.vertices.is_empty() {
            return;
        }
        
        // Upload vertex data (SoA handles this internally)
        self.vertices.upload(device);
        
        // Upload index data
        self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        }));
    }
    
    /// Bind mesh for rendering
    pub fn bind<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.vertices.bind(render_pass);
        
        if let Some(buffer) = &self.index_buffer {
            render_pass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
        }
    }
    
    /// Get the number of indices (for draw calls)
    pub fn index_count(&self) -> u32 {
        self.indices.len() as u32
    }
    
    /// Get memory statistics
    pub fn memory_stats(&self) -> MeshStats {
        let vertex_stats = self.vertices.memory_stats();
        let index_size = self.indices.len() * std::mem::size_of::<u32>();
        
        MeshStats {
            vertex_stats: vertex_stats.clone(),
            index_count: self.indices.len(),
            index_size,
            total_size: vertex_stats.total_size + index_size,
        }
    }
    
    /// Convert from traditional mesh for migration
    pub fn from_traditional_mesh(vertices: &[super::vertex::Vertex], indices: &[u32]) -> Self {
        let mut mesh = Self::new();
        mesh.vertices = VertexBufferSoA::from_aos(vertices);
        mesh.indices = indices.to_vec();
        mesh
    }
}

#[derive(Debug)]
pub struct MeshStats {
    pub vertex_stats: VertexBufferStats,
    pub index_count: usize,
    pub index_size: usize,
    pub total_size: usize,
}

impl std::fmt::Display for MeshStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Mesh: {} indices ({}B), Vertices: {}, Total: {}B",
            self.index_count,
            self.index_size,
            self.vertex_stats,
            self.total_size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mesh_creation() {
        let mut mesh = MeshSoA::new();
        
        // Add a simple quad
        mesh.add_quad(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
            1.0,
            [1.0, 1.0, 1.0, 1.0],
        );
        
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        assert_eq!(mesh.index_count(), 6);
    }
    
    #[test]
    fn test_memory_stats() {
        let mut mesh = MeshSoA::new();
        
        // Add 100 quads
        for i in 0..100 {
            let offset = i as f32;
            mesh.add_quad(
                [
                    [offset, 0.0, 0.0],
                    [offset + 1.0, 0.0, 0.0],
                    [offset + 1.0, 1.0, 0.0],
                    [offset, 1.0, 0.0],
                ],
                [1.0, 1.0, 1.0],
                [0.0, 0.0, 1.0],
                1.0,
                [1.0, 1.0, 1.0, 1.0],
            );
        }
        
        let stats = mesh.memory_stats();
        assert_eq!(stats.vertex_stats.vertex_count, 400); // 100 quads * 4 vertices
        assert_eq!(stats.index_count, 600); // 100 quads * 6 indices
    }
}