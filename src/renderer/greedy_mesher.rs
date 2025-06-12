/// Greedy Meshing Algorithm for Voxel Chunks
/// 
/// Reduces triangle count by 10-100x by merging adjacent faces
/// with the same material into larger quads.
/// Part of Sprint 29: Mesh Optimization & Advanced LOD

use crate::world::{Chunk, BlockId, ChunkPos, BlockRegistry};
use crate::renderer::{Vertex, mesh::ChunkMesh};
use cgmath::Vector3;

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
#[derive(Debug, Clone)]
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
    pub fn generate_mesh(&self, chunk: &Chunk, chunk_pos: ChunkPos, registry: &BlockRegistry) -> ChunkMesh {
        let quads = self.extract_quads(chunk);
        
        // Log optimization statistics
        let total_quads = quads.len();
        let total_triangles = total_quads * 2;
        
        // Enhanced logging for debugging
        static mut MESH_GEN_COUNT: usize = 0;
        unsafe {
            if MESH_GEN_COUNT < 10 {
                let non_air_blocks = chunk.blocks().iter().filter(|&&b| b != BlockId::AIR).count();
                log::info!(
                    "[GreedyMesher::generate_mesh] Chunk {:?}: {} non-air blocks -> {} quads ({} triangles)",
                    chunk_pos, non_air_blocks, total_quads, total_triangles
                );
                MESH_GEN_COUNT += 1;
            }
        }
        
        self.quads_to_mesh(&quads, chunk_pos)
    }
    
    /// Build chunk mesh with neighbor information for proper face culling
    pub fn build_chunk_mesh(
        &mut self,
        chunk: &Chunk,
        chunk_pos: ChunkPos,
        chunk_size: u32,
        registry: &BlockRegistry,
        neighbors: &[Option<std::sync::Arc<parking_lot::RwLock<Chunk>>>],
    ) -> ChunkMesh {
        // For now, just use generate_mesh
        // TODO: Implement neighbor-aware face culling
        self.generate_mesh(chunk, chunk_pos, registry)
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
                            // Safety: u and v are guaranteed to be < size by the loop bounds
                            if let Some(row) = mask.get_mut(u as usize) {
                                if let Some(cell) = row.get_mut(v as usize) {
                                    *cell = Some(block);
                                }
                            }
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
                // Safe access to 2D arrays
                let is_used = used.get(start_u).and_then(|row| row.get(start_v).copied()).unwrap_or(true);
                let mask_value = mask.get(start_u).and_then(|row| row.get(start_v).copied()).flatten();
                
                if is_used || mask_value.is_none() {
                    continue;
                }
                
                let material = match mask_value {
                    Some(m) => m,
                    None => continue, // Should not happen due to is_none check above
                };
                
                // Find maximum width
                let mut width = 1;
                while start_u + width < size {
                    let is_used = used.get(start_u + width)
                        .and_then(|row| row.get(start_v).copied())
                        .unwrap_or(true);
                    let mask_val = mask.get(start_u + width)
                        .and_then(|row| row.get(start_v).copied())
                        .flatten();
                    
                    if is_used || mask_val != Some(material) {
                        break;
                    }
                    width += 1;
                }
                
                // Find maximum height that works for entire width
                let mut height = 1;
                'height_loop: while start_v + height < size {
                    for u in start_u..start_u + width {
                        let is_used = used.get(u)
                            .and_then(|row| row.get(start_v + height).copied())
                            .unwrap_or(true);
                        let mask_val = mask.get(u)
                            .and_then(|row| row.get(start_v + height).copied())
                            .flatten();
                        
                        if is_used || mask_val != Some(material) {
                            break 'height_loop;
                        }
                    }
                    height += 1;
                }
                
                // Mark area as used
                for u in start_u..start_u + width {
                    for v in start_v..start_v + height {
                        if let Some(row) = used.get_mut(u) {
                            if let Some(cell) = row.get_mut(v) {
                                *cell = true;
                            }
                        }
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
    
    /// Convert greedy quads to ChunkMesh with world position offset
    fn quads_to_mesh(&self, quads: &[GreedyQuad], chunk_pos: ChunkPos) -> ChunkMesh {
        let mut mesh = ChunkMesh::new();
        
        // Calculate world offset for this chunk
        let world_offset = Vector3::new(
            (chunk_pos.x * self.chunk_size as i32) as f32,
            (chunk_pos.y * self.chunk_size as i32) as f32,
            (chunk_pos.z * self.chunk_size as i32) as f32,
        );
        
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
                
                // Apply world offset to convert from chunk-local to world coordinates
                pos += world_offset;
                
                // Get block color - for now use simple colors based on block ID
                let color = match quad.material {
                    BlockId(1) => [0.3, 0.7, 0.2], // Grass - green
                    BlockId(2) => [0.5, 0.3, 0.1], // Dirt - brown
                    BlockId(3) => [0.6, 0.6, 0.6], // Stone - gray
                    BlockId(5) => [0.9, 0.8, 0.6], // Sand - yellow
                    BlockId(6) => [0.1, 0.4, 0.8], // Water - blue
                    BlockId(7) => [1.0, 0.8, 0.4], // Torch - orange
                    _ => [1.0, 1.0, 1.0], // Default - white
                };
                
                verts.push(Vertex {
                    position: [pos.x, pos.y, pos.z],
                    color,
                    normal,
                    light: 1.0, // TODO: Calculate proper lighting
                    ao: quad.ao_values.get(verts.len()).copied().unwrap_or(255) as f32 / 255.0,
                });
            }
            
            // Add quad to mesh
            let verts_array: [Vertex; 4] = verts.try_into().unwrap();
            mesh.add_quad(verts_array);
        }
        
        mesh
    }
    
    /// Generate mesh with statistics
    pub fn generate_mesh_with_stats(&self, chunk: &Chunk, chunk_pos: ChunkPos, registry: &BlockRegistry) -> (ChunkMesh, GreedyMeshStats) {
        let quads = self.extract_quads(chunk);
        
        // Calculate statistics
        let mut stats = GreedyMeshStats::default();
        stats.output_quads = quads.len() as u32;
        stats.output_triangles = stats.output_quads * 2;
        
        // Calculate theoretical input faces (without greedy meshing)
        let mut input_faces = 0u32;
        for y in 0..self.chunk_size {
            for z in 0..self.chunk_size {
                for x in 0..self.chunk_size {
                    if chunk.get_block(x, y, z) != BlockId::AIR {
                        // Count visible faces
                        if x == 0 || chunk.get_block(x - 1, y, z) == BlockId::AIR { input_faces += 1; }
                        if x == self.chunk_size - 1 || chunk.get_block(x + 1, y, z) == BlockId::AIR { input_faces += 1; }
                        if y == 0 || chunk.get_block(x, y - 1, z) == BlockId::AIR { input_faces += 1; }
                        if y == self.chunk_size - 1 || chunk.get_block(x, y + 1, z) == BlockId::AIR { input_faces += 1; }
                        if z == 0 || chunk.get_block(x, y, z - 1) == BlockId::AIR { input_faces += 1; }
                        if z == self.chunk_size - 1 || chunk.get_block(x, y, z + 1) == BlockId::AIR { input_faces += 1; }
                    }
                }
            }
        }
        
        stats.input_faces = input_faces;
        stats.reduction_ratio = if stats.output_quads > 0 {
            input_faces as f32 / stats.output_quads as f32
        } else {
            0.0
        };
        
        // Find largest quad
        stats.largest_quad = quads.iter()
            .map(|q| (q.size[0] * q.size[1] * q.size[2]) as u32)
            .max()
            .unwrap_or(0);
        
        (self.quads_to_mesh(&quads, chunk_pos), stats)
    }
}

/// Statistics for greedy meshing
#[derive(Debug, Default, Clone)]
pub struct GreedyMeshStats {
    pub input_faces: u32,
    pub output_quads: u32,
    pub output_triangles: u32,
    pub reduction_ratio: f32,
    pub largest_quad: u32,
}

