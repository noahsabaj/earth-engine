/// Chunk Mesh Adapter - Connects data_mesh_builder to the rendering pipeline
///
/// Sprint 35: Integration layer following DOP principles
/// No allocations in hot paths, uses buffer pools
use crate::{
    renderer::{
        data_mesh_builder::{operations, MeshBuffer, MESH_BUFFER_POOL},
        mesh::{chunk_mesh_ops, ChunkMesh},
        vertex::Vertex,
    },
    typed_blocks,
    world::{storage::ChunkSoA, BlockRegistry},
    BlockId, ChunkPos,
};
use parking_lot::RwLock;
use std::sync::Arc;

/// Neighbor chunk data for face culling
pub struct NeighborData<'a> {
    pub north: Option<&'a ChunkSoA>,
    pub south: Option<&'a ChunkSoA>,
    pub east: Option<&'a ChunkSoA>,
    pub west: Option<&'a ChunkSoA>,
    pub up: Option<&'a ChunkSoA>,
    pub down: Option<&'a ChunkSoA>,
}

/// Convert MeshBuffer to ChunkMesh (for compatibility with existing code)
pub fn mesh_buffer_to_chunk_mesh(buffer: &MeshBuffer) -> ChunkMesh {
    let mut mesh = chunk_mesh_ops::create_empty();

    // Copy only the used portion of the buffers
    mesh.vertices.reserve(buffer.vertex_count);
    mesh.indices.reserve(buffer.index_count);

    // Convert vertices
    for i in 0..buffer.vertex_count {
        let src = &buffer.vertices[i];
        mesh.vertices.push(Vertex {
            position: src.position,
            color: src.color,
            normal: src.normal,
            light: src.light,
            ao: src.ao,
        });
    }

    // Copy indices
    mesh.indices
        .extend_from_slice(&buffer.indices[0..buffer.index_count]);

    mesh
}

/// Build mesh for a chunk using the data-oriented mesh builder
pub fn build_chunk_mesh_dop(
    chunk: &ChunkSoA,
    neighbors: NeighborData,
    registry: &BlockRegistry,
) -> MeshBuffer {
    let mut buffer = MESH_BUFFER_POOL.acquire();

    let chunk_size = chunk.size();

    // Build mesh using the operations module
    operations::build_chunk_mesh(&mut buffer, chunk.position(), chunk_size, |x, y, z| {
        // Get block from chunk
        chunk.get_block(x, y, z)
    });

    // Enhanced version with neighbor culling
    build_chunk_mesh_with_neighbors(&mut buffer, chunk, neighbors, registry);

    buffer
}

/// Build mesh with proper neighbor face culling
fn build_chunk_mesh_with_neighbors(
    buffer: &mut MeshBuffer,
    chunk: &ChunkSoA,
    neighbors: NeighborData,
    registry: &BlockRegistry,
) {
    let start = std::time::Instant::now();
    buffer.metadata.chunk_pos = [chunk.position().x, chunk.position().y, chunk.position().z];

    let chunk_size = chunk.size();

    // Iterate through all blocks
    for y in 0..chunk_size {
        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let block = chunk.get_block(x, y, z);
                if block == BlockId::AIR {
                    continue;
                }

                // Get block properties from registry if needed
                let is_opaque = true; // For now, assume all non-air blocks are opaque

                let world_x = x as f32;
                let world_y = y as f32;
                let world_z = z as f32;

                // Get lighting info
                let light_level = chunk.get_light(x, y, z);
                let light = light_level.sky.max(light_level.block);

                // Simple AO calculation (can be enhanced)
                let ao = [255; 4]; // No AO for now

                // Check each face visibility

                // Top face
                if should_render_face(chunk, neighbors.up, x, y, z, 0, 1, 0, chunk_size) {
                    let _ = operations::add_quad(
                        buffer,
                        operations::BlockFace::Top,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        light,
                        ao,
                    );
                }

                // Bottom face
                if should_render_face(chunk, neighbors.down, x, y, z, 0, -1, 0, chunk_size) {
                    let _ = operations::add_quad(
                        buffer,
                        operations::BlockFace::Bottom,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        light,
                        ao,
                    );
                }

                // North face (-Z)
                if should_render_face(chunk, neighbors.north, x, y, z, 0, 0, -1, chunk_size) {
                    let _ = operations::add_quad(
                        buffer,
                        operations::BlockFace::North,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        light,
                        ao,
                    );
                }

                // South face (+Z)
                if should_render_face(chunk, neighbors.south, x, y, z, 0, 0, 1, chunk_size) {
                    let _ = operations::add_quad(
                        buffer,
                        operations::BlockFace::South,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        light,
                        ao,
                    );
                }

                // East face (+X)
                if should_render_face(chunk, neighbors.east, x, y, z, 1, 0, 0, chunk_size) {
                    let _ = operations::add_quad(
                        buffer,
                        operations::BlockFace::East,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        light,
                        ao,
                    );
                }

                // West face (-X)
                if should_render_face(chunk, neighbors.west, x, y, z, -1, 0, 0, chunk_size) {
                    let _ = operations::add_quad(
                        buffer,
                        operations::BlockFace::West,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        light,
                        ao,
                    );
                }
            }
        }
    }

    // Update metadata
    buffer.metadata.vertex_count = buffer.vertex_count as u32;
    buffer.metadata.index_count = buffer.index_count as u32;
    buffer.metadata.generation_time_us = start.elapsed().as_micros() as u32;
}

/// Check if a face should be rendered
fn should_render_face(
    chunk: &ChunkSoA,
    neighbor: Option<&ChunkSoA>,
    x: u32,
    y: u32,
    z: u32,
    dx: i32,
    dy: i32,
    dz: i32,
    chunk_size: u32,
) -> bool {
    let nx = x as i32 + dx;
    let ny = y as i32 + dy;
    let nz = z as i32 + dz;

    // Check if we need to look at neighbor chunk
    if nx < 0
        || ny < 0
        || nz < 0
        || nx >= chunk_size as i32
        || ny >= chunk_size as i32
        || nz >= chunk_size as i32
    {
        // Check neighbor chunk
        if let Some(neighbor) = neighbor {
            // Calculate position in neighbor chunk
            let neighbor_x = if nx < 0 {
                chunk_size - 1
            } else if nx >= chunk_size as i32 {
                0
            } else {
                nx as u32
            };
            let neighbor_y = if ny < 0 {
                chunk_size - 1
            } else if ny >= chunk_size as i32 {
                0
            } else {
                ny as u32
            };
            let neighbor_z = if nz < 0 {
                chunk_size - 1
            } else if nz >= chunk_size as i32 {
                0
            } else {
                nz as u32
            };

            let neighbor_block = neighbor.get_block(neighbor_x, neighbor_y, neighbor_z);
            neighbor_block == BlockId::AIR
        } else {
            // No neighbor, render the face
            true
        }
    } else {
        // Check within same chunk
        let adjacent_block = chunk.get_block(nx as u32, ny as u32, nz as u32);
        adjacent_block == BlockId::AIR
    }
}

/// Batch mesh building for multiple chunks
pub struct ChunkMeshBatch {
    chunks: Vec<(ChunkPos, Arc<RwLock<ChunkSoA>>)>,
    meshes: Vec<MeshBuffer>,
}

impl ChunkMeshBatch {
    pub fn new(capacity: usize) -> Self {
        Self {
            chunks: Vec::with_capacity(capacity),
            meshes: Vec::with_capacity(capacity),
        }
    }

    pub fn add_chunk(&mut self, pos: ChunkPos, chunk: Arc<RwLock<ChunkSoA>>) {
        self.chunks.push((pos, chunk));
    }

    /// Build all meshes in parallel
    pub fn build_all(&mut self, registry: &BlockRegistry) {
        use rayon::prelude::*;

        // Build meshes in parallel
        let meshes: Vec<_> = self
            .chunks
            .par_iter()
            .map(|(pos, chunk)| {
                let chunk_guard = chunk.read();
                let neighbors = NeighborData {
                    north: None, // TODO: Get actual neighbors
                    south: None,
                    east: None,
                    west: None,
                    up: None,
                    down: None,
                };

                build_chunk_mesh_dop(&*chunk_guard, neighbors, registry)
            })
            .collect();

        self.meshes = meshes;
    }

    /// Get the built meshes
    pub fn take_meshes(self) -> Vec<MeshBuffer> {
        self.meshes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_buffer_conversion() {
        let mut buffer = MESH_BUFFER_POOL.acquire();

        // Add a simple quad
        let _ = operations::add_quad(
            &mut buffer,
            operations::BlockFace::Top,
            0.0,
            0.0,
            0.0,
            typed_blocks::GRASS,
            15,
            [255; 4],
        );

        let mesh = mesh_buffer_to_chunk_mesh(&buffer);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);

        MESH_BUFFER_POOL.release(buffer);
    }
}
