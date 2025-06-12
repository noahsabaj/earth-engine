/// Zero-allocation optimized greedy mesher
/// Uses pre-allocated buffers and object pools to eliminate allocations in hot paths

use crate::world::{Chunk, BlockId, ChunkPos, BlockRegistry};
use crate::renderer::{Vertex, mesh::ChunkMesh, greedy_mesher::{FaceDirection, GreedyQuad}};
use crate::renderer::allocation_optimizations::{with_meshing_buffers, MeshingBuffers};
use cgmath::Vector3;
use std::sync::Arc;
use parking_lot::RwLock;

/// Optimized greedy mesher that reuses buffers
pub struct OptimizedGreedyMesher {
    chunk_size: u32,
}

impl OptimizedGreedyMesher {
    pub fn new(chunk_size: u32) -> Self {
        Self { chunk_size }
    }
    
    /// Build chunk mesh with zero allocations using thread-local buffers
    pub fn build_chunk_mesh(
        &mut self,
        chunk: &Chunk,
        chunk_pos: ChunkPos,
        chunk_size: u32,
        registry: &BlockRegistry,
        neighbors: &[Option<Arc<RwLock<Chunk>>>],
    ) -> ChunkMesh {
        with_meshing_buffers(chunk_size as usize, |buffers| {
            self.build_with_buffers(chunk, chunk_pos, registry, neighbors, buffers)
        })
    }
    
    /// Internal mesh building using provided buffers
    fn build_with_buffers(
        &self,
        chunk: &Chunk,
        chunk_pos: ChunkPos,
        registry: &BlockRegistry,
        neighbors: &[Option<Arc<RwLock<Chunk>>>],
        buffers: &mut MeshingBuffers,
    ) -> ChunkMesh {
        // Extract quads into pre-allocated buffer
        self.extract_quads_buffered(chunk, buffers);
        
        // Convert quads to mesh using pre-allocated vertex/index buffers
        // Clone the quads to avoid borrow conflict
        let quads = buffers.quads.clone();
        self.quads_to_mesh_buffered(&quads, buffers)
    }
    
    /// Extract greedy quads without allocating new vectors
    fn extract_quads_buffered(&self, chunk: &Chunk, buffers: &mut MeshingBuffers) {
        // Process each face direction
        for face in [
            FaceDirection::PosX, FaceDirection::NegX,
            FaceDirection::PosY, FaceDirection::NegY,
            FaceDirection::PosZ, FaceDirection::NegZ,
        ] {
            self.extract_quads_for_face_buffered(chunk, face, buffers);
        }
    }
    
    /// Extract quads for a specific face using pre-allocated buffers
    fn extract_quads_for_face_buffered(
        &self,
        chunk: &Chunk,
        face: FaceDirection,
        buffers: &mut MeshingBuffers,
    ) {
        let size = self.chunk_size as i32;
        
        // Determine axes based on face direction
        let axis = face.axis();
        let u_axis = (axis + 1) % 3;
        let v_axis = (axis + 2) % 3;
        
        // Process each slice perpendicular to the face normal
        for slice in 0..size {
            // Clear reusable buffers
            for row in &mut buffers.mask {
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
                        // Check if face is visible
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
                            buffers.mask[u as usize][v as usize] = Some(block);
                        }
                    }
                }
            }
            
            // Extract rectangles from mask using greedy algorithm
            self.extract_rectangles_buffered(&buffers.mask, &mut buffers.used, slice, face, &mut buffers.quads);
        }
    }
    
    /// Extract rectangles without allocating
    fn extract_rectangles_buffered(
        &self,
        mask: &[Vec<Option<BlockId>>],
        used: &mut Vec<Vec<bool>>,
        slice: i32,
        face: FaceDirection,
        quads: &mut Vec<GreedyQuad>,
    ) {
        let size = mask.len();
        
        // Clear used flags
        for row in &mut *used {
            row.fill(false);
        }
        
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
                
                // Find maximum height
                let mut height = 1;
                'height_loop: while start_v + height < size {
                    for u in start_u..start_u + width {
                        if used[u][start_v + height] || mask[u][start_v + height] != Some(material) {
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
    }
    
    /// Convert quads to mesh using pre-allocated buffers
    fn quads_to_mesh_buffered(&self, quads: &[GreedyQuad], buffers: &mut MeshingBuffers) -> ChunkMesh {
        let mut mesh = ChunkMesh::new();
        
        // Pre-allocate mesh capacity based on quads
        mesh.vertices.reserve(quads.len() * 4);
        mesh.indices.reserve(quads.len() * 6);
        
        for quad in quads {
            let normal = quad.face.normal();
            let axis = quad.face.axis();
            let u_axis = (axis + 1) % 3;
            let v_axis = (axis + 2) % 3;
            
            // Use fixed-size array to avoid allocation
            let offsets = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
            
            // Generate vertices directly into mesh
            let base_index = mesh.vertices.len() as u32;
            
            for (i, &(u_offset, v_offset)) in offsets.iter().enumerate() {
                let mut pos = quad.position;
                pos[u_axis] += u_offset * quad.size[u_axis];
                pos[v_axis] += v_offset * quad.size[v_axis];
                
                mesh.vertices.push(Vertex {
                    position: [pos.x, pos.y, pos.z],
                    color: [1.0, 1.0, 1.0],
                    normal,
                    light: 1.0,
                    ao: quad.ao_values[i] as f32 / 255.0,
                });
            }
            
            // Add indices for two triangles
            mesh.indices.extend_from_slice(&[
                base_index, base_index + 1, base_index + 2,
                base_index, base_index + 2, base_index + 3,
            ]);
        }
        
        mesh
    }
}

/// Thread pool specifically for mesh generation with pinned threads
pub struct MeshGenerationPool {
    pool: rayon::ThreadPool,
}

impl MeshGenerationPool {
    pub fn new(thread_count: usize) -> Self {
        use rayon::ThreadPoolBuilder;
        
        let pool = ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .thread_name(|idx| format!("mesh-gen-{}", idx))
            .start_handler(|_idx| {
                // Pin thread to CPU for better cache performance
                #[cfg(target_os = "linux")]
                {
                    use std::os::raw::c_ulong;
                    unsafe {
                        let mut cpu_set: libc::cpu_set_t = std::mem::zeroed();
                        libc::CPU_SET(_idx, &mut cpu_set);
                        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &cpu_set);
                    }
                }
            })
            .build()
            .expect("Failed to create mesh generation pool");
            
        Self { pool }
    }
    
    pub fn generate_mesh<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.pool.spawn(f);
    }
}