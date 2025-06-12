use crate::renderer::vertex::Vertex;

/// Data structure for chunk mesh - no methods, following DOP
#[derive(Debug)]
pub struct ChunkMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// Operations on ChunkMesh data - pure functions, no self
pub mod chunk_mesh_ops {
    use super::*;
    
    pub fn create_empty() -> ChunkMesh {
        ChunkMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
    
    pub fn clear(mesh: &mut ChunkMesh) {
        mesh.vertices.clear();
        mesh.indices.clear();
    }
    
    pub fn add_quad(mesh: &mut ChunkMesh, vertices: [Vertex; 4]) {
        let start_index = mesh.vertices.len() as u32;
        
        // Add vertices
        mesh.vertices.extend_from_slice(&vertices);
        
        // Add indices for two triangles
        mesh.indices.extend_from_slice(&[
            start_index,
            start_index + 1,
            start_index + 2,
            start_index,
            start_index + 2,
            start_index + 3,
        ]);
    }
    
    pub fn is_empty(mesh: &ChunkMesh) -> bool {
        mesh.vertices.is_empty()
    }
}