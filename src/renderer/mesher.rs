use std::sync::Arc;
use parking_lot::RwLock;
use crate::{
    renderer::{mesh::ChunkMesh, vertex::Vertex},
    world::{BlockId, BlockRegistry, Chunk, ChunkPos},
    lighting::LightLevel,
};
use cgmath::Vector3;

pub struct ChunkMesher {
    // Temporary storage for mesh building
}

impl ChunkMesher {
    pub fn new() -> Self {
        Self {}
    }
    pub fn generate_mesh(chunk: &Chunk, registry: &BlockRegistry) -> ChunkMesh {
        let mut mesh = ChunkMesh::new();
        let size = chunk.size();

        for y in 0..size {
            for z in 0..size {
                for x in 0..size {
                    let block_id = chunk.get_block(x, y, z);
                    if block_id == BlockId::AIR {
                        continue;
                    }

                    // Get block data
                    let block = match registry.get_block(block_id) {
                        Some(b) => b,
                        None => continue,
                    };
                    let render_data = block.get_render_data();

                    let pos = Vector3::new(x as f32, y as f32, z as f32);

                    // Get light level at this position
                    let light_level = chunk.get_light(x, y, z);
                    
                    // Check each face
                    // Right face (+X)
                    if Self::is_face_visible(chunk, x + 1, y, z, size) {
                        Self::add_face(
                            &mut mesh,
                            pos,
                            Face::Right,
                            render_data.color,
                            light_level,
                            chunk,
                            x, y, z,
                        );
                    }

                    // Left face (-X)
                    if x == 0 || Self::is_face_visible(chunk, x - 1, y, z, size) {
                        Self::add_face(
                            &mut mesh,
                            pos,
                            Face::Left,
                            render_data.color,
                            light_level,
                            chunk,
                            x, y, z,
                        );
                    }

                    // Top face (+Y)
                    if Self::is_face_visible(chunk, x, y + 1, z, size) {
                        Self::add_face(
                            &mut mesh,
                            pos,
                            Face::Top,
                            render_data.color,
                            light_level,
                            chunk,
                            x, y, z,
                        );
                    }

                    // Bottom face (-Y)
                    if y == 0 || Self::is_face_visible(chunk, x, y - 1, z, size) {
                        Self::add_face(
                            &mut mesh,
                            pos,
                            Face::Bottom,
                            render_data.color,
                            light_level,
                            chunk,
                            x, y, z,
                        );
                    }

                    // Front face (+Z)
                    if Self::is_face_visible(chunk, x, y, z + 1, size) {
                        Self::add_face(
                            &mut mesh,
                            pos,
                            Face::Front,
                            render_data.color,
                            light_level,
                            chunk,
                            x, y, z,
                        );
                    }

                    // Back face (-Z)
                    if z == 0 || Self::is_face_visible(chunk, x, y, z - 1, size) {
                        Self::add_face(
                            &mut mesh,
                            pos,
                            Face::Back,
                            render_data.color,
                            light_level,
                            chunk,
                            x, y, z,
                        );
                    }
                }
            }
        }

        mesh
    }
    
    /// Build chunk mesh with neighbor information for proper face culling
    pub fn build_chunk_mesh(
        &mut self,
        chunk: &Chunk,
        chunk_pos: ChunkPos,
        chunk_size: u32,
        registry: &BlockRegistry,
        neighbors: &[Option<Arc<RwLock<Chunk>>>],
    ) -> ChunkMesh {
        // For now, just use the existing generate_mesh method
        // TODO: Implement proper neighbor-aware face culling
        Self::generate_mesh(chunk, registry)
    }

    fn is_face_visible(chunk: &Chunk, x: u32, y: u32, z: u32, size: u32) -> bool {
        if x >= size || y >= size || z >= size {
            return true; // Face is at chunk boundary
        }
        chunk.get_block(x, y, z) == BlockId::AIR
    }

    fn add_face(
        mesh: &mut ChunkMesh, 
        pos: Vector3<f32>, 
        face: Face, 
        color: [f32; 3],
        block_light: LightLevel,
        _chunk: &Chunk,
        _x: u32, _y: u32, _z: u32,
    ) {
        // Convert light level to 0.0-1.0 range
        let light_value = block_light.combined() as f32 / 15.0;
        
        // Simple directional lighting multiplier
        let face_brightness = match face {
            Face::Top => 1.0,     // Full brightness
            Face::Bottom => 0.5,  // Darker
            Face::Right | Face::Left => 0.8,
            Face::Front | Face::Back => 0.6,
        };
        
        let final_light = light_value * face_brightness;
        
        let vertices = match face {
            Face::Right => [
                Vertex::with_lighting([pos.x + 1.0, pos.y, pos.z], color, [1.0, 0.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y + 1.0, pos.z], color, [1.0, 0.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y + 1.0, pos.z + 1.0], color, [1.0, 0.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y, pos.z + 1.0], color, [1.0, 0.0, 0.0], final_light, 1.0),
            ],
            Face::Left => [
                Vertex::with_lighting([pos.x, pos.y, pos.z + 1.0], color, [-1.0, 0.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x, pos.y + 1.0, pos.z + 1.0], color, [-1.0, 0.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x, pos.y + 1.0, pos.z], color, [-1.0, 0.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x, pos.y, pos.z], color, [-1.0, 0.0, 0.0], final_light, 1.0),
            ],
            Face::Top => [
                Vertex::with_lighting([pos.x, pos.y + 1.0, pos.z], color, [0.0, 1.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x, pos.y + 1.0, pos.z + 1.0], color, [0.0, 1.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y + 1.0, pos.z + 1.0], color, [0.0, 1.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y + 1.0, pos.z], color, [0.0, 1.0, 0.0], final_light, 1.0),
            ],
            Face::Bottom => [
                Vertex::with_lighting([pos.x, pos.y, pos.z + 1.0], color, [0.0, -1.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x, pos.y, pos.z], color, [0.0, -1.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y, pos.z], color, [0.0, -1.0, 0.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y, pos.z + 1.0], color, [0.0, -1.0, 0.0], final_light, 1.0),
            ],
            Face::Front => [
                Vertex::with_lighting([pos.x, pos.y, pos.z + 1.0], color, [0.0, 0.0, 1.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y, pos.z + 1.0], color, [0.0, 0.0, 1.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y + 1.0, pos.z + 1.0], color, [0.0, 0.0, 1.0], final_light, 1.0),
                Vertex::with_lighting([pos.x, pos.y + 1.0, pos.z + 1.0], color, [0.0, 0.0, 1.0], final_light, 1.0),
            ],
            Face::Back => [
                Vertex::with_lighting([pos.x + 1.0, pos.y, pos.z], color, [0.0, 0.0, -1.0], final_light, 1.0),
                Vertex::with_lighting([pos.x, pos.y, pos.z], color, [0.0, 0.0, -1.0], final_light, 1.0),
                Vertex::with_lighting([pos.x, pos.y + 1.0, pos.z], color, [0.0, 0.0, -1.0], final_light, 1.0),
                Vertex::with_lighting([pos.x + 1.0, pos.y + 1.0, pos.z], color, [0.0, 0.0, -1.0], final_light, 1.0),
            ],
        };

        mesh.add_quad(vertices);
    }
}

enum Face {
    Right,
    Left,
    Top,
    Bottom,
    Front,
    Back,
}