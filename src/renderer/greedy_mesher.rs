/// Greedy Meshing Algorithm for Voxel Chunks
/// 
/// Reduces triangle count by 10-100x by merging adjacent faces
/// with the same material into larger quads.
/// Part of Sprint 29: Mesh Optimization & Advanced LOD

use crate::world::{Chunk, BlockId, ChunkPos};
use crate::renderer::Vertex;
use cgmath::Vector3;
use std::collections::HashMap;

/// Direction of a face
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaceDirection {
    PosX, NegX,
    PosY, NegY,
    PosZ, NegZ,
}

impl FaceDirection {
    /// Get normal vector for this face
    pub fn normal(&self) -> [f32; 3] {
        match self {
            FaceDirection::PosX => [1.0, 0.0, 0.0],
            FaceDirection::NegX => [-1.0, 0.0, 0.0],
            FaceDirection::PosY => [0.0, 1.0, 0.0],
            FaceDirection::NegY => [0.0, -1.0, 0.0],
            FaceDirection::PosZ => [0.0, 0.0, 1.0],
            FaceDirection::NegZ => [0.0, 0.0, -1.0],
        }
    }
    
    /// Get axis index (0=X, 1=Y, 2=Z)
    pub fn axis(&self) -> usize {
        match self {
            FaceDirection::PosX | FaceDirection::NegX => 0,
            FaceDirection::PosY | FaceDirection::NegY => 1,
            FaceDirection::PosZ | FaceDirection::NegZ => 2,
        }
    }
    
    /// Is this a positive direction?
    pub fn is_positive(&self) -> bool {
        matches!(self, FaceDirection::PosX | FaceDirection::PosY | FaceDirection::PosZ)
    }
}

/// A greedy quad representing multiple merged voxel faces
#[derive(Debug)]
pub struct GreedyQuad {
    pub position: Vector3<f32>,
    pub size: Vector3<f32>,
    pub face: FaceDirection,
    pub material: BlockId,
    pub ao_values: [u8; 4], // Ambient occlusion per vertex
}

/// Greedy mesher for optimized voxel rendering
pub struct GreedyMesher {
    chunk_size: u32,
}

impl GreedyMesher {
    pub fn new(chunk_size: u32) -> Self {
        Self { chunk_size }
    }
    
    /// Generate optimized mesh using greedy meshing
    pub fn generate_mesh(&self, chunk: &Chunk) -> Vec<Vertex> {
        let quads = self.extract_quads(chunk);
        self.quads_to_vertices(&quads)
    }
    
    /// Extract greedy quads from chunk
    pub fn extract_quads(&self, chunk: &Chunk) -> Vec<GreedyQuad> {
        let mut quads = Vec::new();
        
        // Process each face direction
        for face in [
            FaceDirection::PosX, FaceDirection::NegX,
            FaceDirection::PosY, FaceDirection::NegY,
            FaceDirection::PosZ, FaceDirection::NegZ,
        ] {
            quads.extend(self.extract_quads_for_face(chunk, face));
        }
        
        quads
    }
    
    /// Extract quads for a specific face direction
    fn extract_quads_for_face(&self, chunk: &Chunk, face: FaceDirection) -> Vec<GreedyQuad> {
        let mut quads = Vec::new();
        let size = self.chunk_size as i32;
        
        // Create a 2D mask for this slice
        let mut mask = vec![vec![None; size as usize]; size as usize];
        
        // Determine axes based on face direction
        let axis = face.axis();
        let u_axis = (axis + 1) % 3;
        let v_axis = (axis + 2) % 3;
        
        // Process each slice perpendicular to the face normal
        for slice in 0..size {
            // Clear mask
            for row in &mut mask {
                row.fill(None);
            }
            
            // Fill mask with visible faces
            for u in 0..size {
                for v in 0..size {
                    let mut pos = [0i32; 3];
                    pos[axis] = if face.is_positive() { slice } else { size - 1 - slice };
                    pos[u_axis] = u;
                    pos[v_axis] = v;
                    
                    let block = chunk.get_block(pos[0] as u32, pos[1] as u32, pos[2] as u32);
                    
                    if block != BlockId::AIR {
                        // Check if face is visible (neighbor is air or at chunk boundary)
                        let neighbor_pos = [
                            pos[0] + face.normal()[0] as i32,
                            pos[1] + face.normal()[1] as i32,
                            pos[2] + face.normal()[2] as i32,
                        ];
                        
                        let is_visible = if neighbor_pos[0] < 0 || neighbor_pos[0] >= size ||
                                           neighbor_pos[1] < 0 || neighbor_pos[1] >= size ||
                                           neighbor_pos[2] < 0 || neighbor_pos[2] >= size {
                            true // At chunk boundary
                        } else {
                            let neighbor = chunk.get_block(
                                neighbor_pos[0] as u32,
                                neighbor_pos[1] as u32,
                                neighbor_pos[2] as u32,
                            );
                            neighbor == BlockId::AIR
                        };
                        
                        if is_visible {
                            mask[u as usize][v as usize] = Some(block);
                        }
                    }
                }
            }
            
            // Extract rectangles from mask using greedy algorithm
            quads.extend(self.extract_rectangles_from_mask(&mask, slice, face));
        }
        
        quads
    }
    
    /// Extract maximal rectangles from a 2D mask
    fn extract_rectangles_from_mask(
        &self,
        mask: &[Vec<Option<BlockId>>],
        slice: i32,
        face: FaceDirection,
    ) -> Vec<GreedyQuad> {
        let mut quads = Vec::new();
        let size = mask.len();
        let mut used = vec![vec![false; size]; size];
        
        // Find rectangles greedily
        for start_u in 0..size {
            for start_v in 0..size {
                if used[start_u][start_v] || mask[start_u][start_v].is_none() {
                    continue;
                }
                
                let material = mask[start_u][start_v].unwrap();
                
                // Find maximum width
                let mut width = 1;
                while start_u + width < size &&
                      !used[start_u + width][start_v] &&
                      mask[start_u + width][start_v] == Some(material) {
                    width += 1;
                }
                
                // Find maximum height that works for entire width
                let mut height = 1;
                'height_loop: while start_v + height < size {
                    for u in start_u..start_u + width {
                        if used[u][start_v + height] ||
                           mask[u][start_v + height] != Some(material) {
                            break 'height_loop;
                        }
                    }
                    height += 1;
                }
                
                // Mark area as used
                for u in start_u..start_u + width {
                    for v in start_v..start_v + height {
                        used[u][v] = true;
                    }
                }
                
                // Create quad
                let axis = face.axis();
                let u_axis = (axis + 1) % 3;
                let v_axis = (axis + 2) % 3;
                
                let mut position = [0.0; 3];
                position[axis] = if face.is_positive() {
                    slice as f32 + 1.0
                } else {
                    slice as f32
                };
                position[u_axis] = start_u as f32;
                position[v_axis] = start_v as f32;
                
                let mut size = [0.0; 3];
                size[u_axis] = width as f32;
                size[v_axis] = height as f32;
                
                quads.push(GreedyQuad {
                    position: Vector3::from(position),
                    size: Vector3::from(size),
                    face,
                    material,
                    ao_values: [255; 4], // TODO: Calculate actual AO
                });
            }
        }
        
        quads
    }
    
    /// Convert greedy quads to vertex data
    fn quads_to_vertices(&self, quads: &[GreedyQuad]) -> Vec<Vertex> {
        let mut vertices = Vec::with_capacity(quads.len() * 6); // 2 triangles per quad
        
        for quad in quads {
            let normal = quad.face.normal();
            let axis = quad.face.axis();
            let u_axis = (axis + 1) % 3;
            let v_axis = (axis + 2) % 3;
            
            // Generate 4 vertices for the quad
            let mut verts = Vec::with_capacity(4);
            
            for &(u_offset, v_offset) in &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
                let mut pos = quad.position;
                pos[u_axis] += u_offset * quad.size[u_axis];
                pos[v_axis] += v_offset * quad.size[v_axis];
                
                // Calculate texture coordinates based on quad size
                let tex_u = u_offset * quad.size[u_axis];
                let tex_v = v_offset * quad.size[v_axis];
                
                verts.push(Vertex {
                    position: [pos.x, pos.y, pos.z],
                    normal,
                    tex_coords: [tex_u, tex_v],
                    color: [1.0, 1.0, 1.0, 1.0], // TODO: Material color
                    ao: quad.ao_values[verts.len()] as f32 / 255.0,
                });
            }
            
            // Create two triangles from the quad
            vertices.push(verts[0]);
            vertices.push(verts[1]);
            vertices.push(verts[2]);
            
            vertices.push(verts[0]);
            vertices.push(verts[2]);
            vertices.push(verts[3]);
        }
        
        vertices
    }
}

/// Statistics for greedy meshing
#[derive(Debug, Default)]
pub struct GreedyMeshStats {
    pub input_faces: u32,
    pub output_quads: u32,
    pub output_triangles: u32,
    pub reduction_ratio: f32,
    pub largest_quad: u32,
}