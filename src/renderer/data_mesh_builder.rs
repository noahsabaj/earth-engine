/// Data-Oriented Mesh Builder
/// 
/// Sprint 35: Zero-allocation mesh building using buffer pools.
/// No Vec::new() in hot paths, all buffers are pre-allocated and reused.

use crate::{ChunkPos, BlockId, VoxelPos};
use bytemuck::{Pod, Zeroable};
use parking_lot::Mutex;
use std::sync::Arc;

/// Maximum vertices per mesh (64K for 16-bit indices)
pub const MAX_VERTICES: usize = 65536;
/// Maximum indices per mesh
pub const MAX_INDICES: usize = MAX_VERTICES * 3 / 2;
/// Pool size for mesh buffers
pub const MESH_POOL_SIZE: usize = 128;

/// Mesh data without allocations
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct MeshMetadata {
    pub chunk_pos: [i32; 3],
    pub vertex_count: u32,
    pub index_count: u32,
    pub flags: u32,
    pub generation_time_us: u32,
}

/// Pre-allocated mesh buffer
pub struct MeshBuffer {
    /// Pre-allocated vertex array
    pub vertices: Vec<Vertex>,
    /// Pre-allocated index array
    pub indices: Vec<u32>,
    /// Current vertex count
    pub vertex_count: usize,
    /// Current index count
    pub index_count: usize,
    /// Associated metadata
    pub metadata: MeshMetadata,
}

impl MeshBuffer {
    fn new() -> Self {
        let mut vertices = Vec::with_capacity(MAX_VERTICES);
        let mut indices = Vec::with_capacity(MAX_INDICES);
        
        // Important: resize to capacity to avoid allocations
        vertices.resize(MAX_VERTICES, Vertex {
            position: [0.0; 3],
            tex_coords: [0.0; 2],
            normal: [0.0; 3],
            ao: 0,
            light: 0,
        });
        indices.resize(MAX_INDICES, 0);
        
        Self {
            vertices,
            indices,
            vertex_count: 0,
            index_count: 0,
            metadata: MeshMetadata {
                chunk_pos: [0; 3],
                vertex_count: 0,
                index_count: 0,
                flags: 0,
                generation_time_us: 0,
            },
        }
    }
    
    /// Reset buffer for reuse (doesn't deallocate)
    fn reset(&mut self) {
        self.vertex_count = 0;
        self.index_count = 0;
        self.metadata = MeshMetadata {
            chunk_pos: [0; 3],
            vertex_count: 0,
            index_count: 0,
            flags: 0,
            generation_time_us: 0,
        };
    }
}

/// Buffer pool for zero-allocation mesh building
pub struct MeshBufferPool {
    /// Available buffers
    available: Mutex<Vec<MeshBuffer>>,
    /// Total buffers created
    total_created: std::sync::atomic::AtomicUsize,
}

impl MeshBufferPool {
    pub fn new() -> Self {
        let mut available = Vec::with_capacity(MESH_POOL_SIZE);
        
        // Pre-allocate initial buffers
        for _ in 0..MESH_POOL_SIZE / 4 {
            available.push(MeshBuffer::new());
        }
        
        Self {
            available: Mutex::new(available),
            total_created: std::sync::atomic::AtomicUsize::new(MESH_POOL_SIZE / 4),
        }
    }
    
    /// Acquire a buffer from the pool
    pub fn acquire(&self) -> MeshBuffer {
        let mut available = self.available.lock();
        
        if let Some(mut buffer) = available.pop() {
            buffer.reset();
            buffer
        } else {
            // Create new buffer if needed (up to limit)
            let created = self.total_created.load(std::sync::atomic::Ordering::Relaxed);
            if created < MESH_POOL_SIZE {
                self.total_created.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                MeshBuffer::new()
            } else {
                // If at limit, wait for one to be available
                // In production, this would block or return an error
                panic!("Mesh buffer pool exhausted");
            }
        }
    }
    
    /// Release a buffer back to the pool
    pub fn release(&self, buffer: MeshBuffer) {
        let mut available = self.available.lock();
        if available.len() < MESH_POOL_SIZE {
            available.push(buffer);
        }
        // If pool is full, buffer is dropped
    }
}

/// Global mesh buffer pool
lazy_static::lazy_static! {
    pub static ref MESH_BUFFER_POOL: Arc<MeshBufferPool> = Arc::new(MeshBufferPool::new());
}

/// Mesh building operations - pure functions
pub mod operations {
    use super::*;
    use std::time::Instant;
    
    /// Block face for mesh generation
    #[derive(Copy, Clone, Debug)]
    pub enum BlockFace {
        Top,
        Bottom,
        North,
        South,
        East,
        West,
    }
    
    /// Add a quad to the mesh buffer
    pub fn add_quad(
        buffer: &mut MeshBuffer,
        face: BlockFace,
        x: f32, y: f32, z: f32,
        block_id: BlockId,
        light: u8,
        ao: [u8; 4],
    ) -> Result<(), &'static str> {
        // Check if we have space
        if buffer.vertex_count + 4 > MAX_VERTICES || buffer.index_count + 6 > MAX_INDICES {
            return Err("Mesh buffer full");
        }
        
        let base_vertex = buffer.vertex_count as u32;
        
        // Define vertices based on face (clockwise winding)
        let (positions, normal) = match face {
            BlockFace::Top => (
                [[x, y+1.0, z], [x+1.0, y+1.0, z], [x+1.0, y+1.0, z+1.0], [x, y+1.0, z+1.0]],
                [0.0, 1.0, 0.0]
            ),
            BlockFace::Bottom => (
                [[x, y, z+1.0], [x+1.0, y, z+1.0], [x+1.0, y, z], [x, y, z]],
                [0.0, -1.0, 0.0]
            ),
            BlockFace::North => (
                [[x, y, z], [x+1.0, y, z], [x+1.0, y+1.0, z], [x, y+1.0, z]],
                [0.0, 0.0, -1.0]
            ),
            BlockFace::South => (
                [[x+1.0, y, z+1.0], [x, y, z+1.0], [x, y+1.0, z+1.0], [x+1.0, y+1.0, z+1.0]],
                [0.0, 0.0, 1.0]
            ),
            BlockFace::East => (
                [[x+1.0, y, z], [x+1.0, y, z+1.0], [x+1.0, y+1.0, z+1.0], [x+1.0, y+1.0, z]],
                [1.0, 0.0, 0.0]
            ),
            BlockFace::West => (
                [[x, y, z+1.0], [x, y, z], [x, y+1.0, z], [x, y+1.0, z+1.0]],
                [-1.0, 0.0, 0.0]
            ),
        };
        
        // Texture coordinates (same for all faces in this simple version)
        let tex_coords = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        
        // Add vertices
        for i in 0..4 {
            buffer.vertices[buffer.vertex_count] = Vertex {
                position: positions[i],
                tex_coords: tex_coords[i],
                normal,
                ao: ao[i],
                light,
            };
            buffer.vertex_count += 1;
        }
        
        // Add indices (two triangles)
        let indices = [
            base_vertex, base_vertex + 1, base_vertex + 2,
            base_vertex, base_vertex + 2, base_vertex + 3,
        ];
        
        for &idx in &indices {
            buffer.indices[buffer.index_count] = idx;
            buffer.index_count += 1;
        }
        
        Ok(())
    }
    
    /// Build mesh from chunk data (example interface)
    pub fn build_chunk_mesh<F>(
        buffer: &mut MeshBuffer,
        chunk_pos: ChunkPos,
        chunk_size: u32,
        get_block: F,
    ) where
        F: Fn(u32, u32, u32) -> BlockId,
    {
        let start = Instant::now();
        buffer.metadata.chunk_pos = [chunk_pos.x, chunk_pos.y, chunk_pos.z];
        
        // Simple visible face culling
        for y in 0..chunk_size {
            for z in 0..chunk_size {
                for x in 0..chunk_size {
                    let block = get_block(x, y, z);
                    if block == BlockId::AIR {
                        continue;
                    }
                    
                    // Check each face
                    let world_x = x as f32;
                    let world_y = y as f32;
                    let world_z = z as f32;
                    
                    // Top face
                    if y == chunk_size - 1 || get_block(x, y + 1, z) == BlockId::AIR {
                        let _ = add_quad(
                            buffer,
                            BlockFace::Top,
                            world_x, world_y, world_z,
                            block,
                            15, // Full light for now
                            [255; 4], // No AO for now
                        );
                    }
                    
                    // Bottom face
                    if y == 0 || get_block(x, y - 1, z) == BlockId::AIR {
                        let _ = add_quad(
                            buffer,
                            BlockFace::Bottom,
                            world_x, world_y, world_z,
                            block,
                            15,
                            [255; 4],
                        );
                    }
                    
                    // Other faces...
                    // (Similar checks for North, South, East, West)
                }
            }
        }
        
        // Update metadata
        buffer.metadata.vertex_count = buffer.vertex_count as u32;
        buffer.metadata.index_count = buffer.index_count as u32;
        buffer.metadata.generation_time_us = start.elapsed().as_micros() as u32;
    }
}

/// Vertex definition matching GPU layout
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub ao: u8,
    pub light: u8,
}

// Usage example:
// let mut buffer = MESH_BUFFER_POOL.acquire();
// operations::build_chunk_mesh(&mut buffer, chunk_pos, 32, |x, y, z| {
//     // Get block at position
//     BlockId::STONE
// });
// // Use buffer.vertices[0..buffer.vertex_count] and buffer.indices[0..buffer.index_count]
// MESH_BUFFER_POOL.release(buffer);