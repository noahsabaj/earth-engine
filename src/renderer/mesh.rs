use crate::renderer::vertex::Vertex;

#[derive(Debug)]
pub struct ChunkMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl ChunkMesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn add_quad(&mut self, vertices: [Vertex; 4]) {
        let start_index = self.vertices.len() as u32;
        
        // Add vertices
        self.vertices.extend_from_slice(&vertices);
        
        // Add indices for two triangles
        self.indices.extend_from_slice(&[
            start_index,
            start_index + 1,
            start_index + 2,
            start_index,
            start_index + 2,
            start_index + 3,
        ]);
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}