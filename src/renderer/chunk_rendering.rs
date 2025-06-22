#![allow(unused_variables, dead_code)]
use crate::renderer::data_mesh_builder::{operations, MeshBuffer};
use crate::renderer::gpu_driven::gpu_driven_renderer::RenderObject;
/// Data-Oriented Chunk Rendering Functions
///
/// Pure functions for transforming chunk data into render-ready structures.
/// No methods, no self, just data transformation following DOP principles.
use crate::{BlockId, ChunkPos};
use cgmath::{InnerSpace, Vector3};

/// Configuration for chunk rendering
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ChunkRenderConfig {
    pub chunk_size: u32,
    pub world_scale: f32,
    pub enable_face_culling: bool,
    pub enable_ambient_occlusion: bool,
}

impl Default for ChunkRenderConfig {
    fn default() -> Self {
        Self {
            chunk_size: 32,
            world_scale: 1.0,
            enable_face_culling: true,
            enable_ambient_occlusion: true,
        }
    }
}

/// Build mesh data from chunk voxels
/// Transforms chunk voxel data → mesh vertices/indices
pub fn build_chunk_mesh_data<F, N>(
    buffer: &mut MeshBuffer,
    chunk_pos: ChunkPos,
    chunk_size: u32,
    get_block: F,
    get_neighbor_block: N,
) -> Result<(), &'static str>
where
    F: Fn(u32, u32, u32) -> BlockId,
    N: Fn(i32, i32, i32) -> BlockId,
{
    let start = std::time::Instant::now();
    buffer.metadata.chunk_pos = [chunk_pos.x, chunk_pos.y, chunk_pos.z];

    // Iterate through all blocks in the chunk
    for y in 0..chunk_size {
        for z in 0..chunk_size {
            for x in 0..chunk_size {
                let block = get_block(x, y, z);
                if block == BlockId::AIR {
                    continue;
                }

                let world_x = x as f32;
                let world_y = y as f32;
                let world_z = z as f32;

                // Check each face for visibility
                // Top face (Y+)
                if should_render_face(
                    x as i32,
                    y as i32 + 1,
                    z as i32,
                    chunk_size,
                    &get_block,
                    &get_neighbor_block,
                ) {
                    operations::add_quad(
                        buffer,
                        operations::BlockFace::Top,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        calculate_face_light(
                            x as i32,
                            y as i32 + 1,
                            z as i32,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                        calculate_face_ao(
                            x as i32,
                            y as i32 + 1,
                            z as i32,
                            operations::BlockFace::Top,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                    )?;
                }

                // Bottom face (Y-)
                if should_render_face(
                    x as i32,
                    y as i32 - 1,
                    z as i32,
                    chunk_size,
                    &get_block,
                    &get_neighbor_block,
                ) {
                    operations::add_quad(
                        buffer,
                        operations::BlockFace::Bottom,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        calculate_face_light(
                            x as i32,
                            y as i32 - 1,
                            z as i32,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                        calculate_face_ao(
                            x as i32,
                            y as i32 - 1,
                            z as i32,
                            operations::BlockFace::Bottom,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                    )?;
                }

                // North face (Z-)
                if should_render_face(
                    x as i32,
                    y as i32,
                    z as i32 - 1,
                    chunk_size,
                    &get_block,
                    &get_neighbor_block,
                ) {
                    operations::add_quad(
                        buffer,
                        operations::BlockFace::North,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        calculate_face_light(
                            x as i32,
                            y as i32,
                            z as i32 - 1,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                        calculate_face_ao(
                            x as i32,
                            y as i32,
                            z as i32 - 1,
                            operations::BlockFace::North,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                    )?;
                }

                // South face (Z+)
                if should_render_face(
                    x as i32,
                    y as i32,
                    z as i32 + 1,
                    chunk_size,
                    &get_block,
                    &get_neighbor_block,
                ) {
                    operations::add_quad(
                        buffer,
                        operations::BlockFace::South,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        calculate_face_light(
                            x as i32,
                            y as i32,
                            z as i32 + 1,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                        calculate_face_ao(
                            x as i32,
                            y as i32,
                            z as i32 + 1,
                            operations::BlockFace::South,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                    )?;
                }

                // East face (X+)
                if should_render_face(
                    x as i32 + 1,
                    y as i32,
                    z as i32,
                    chunk_size,
                    &get_block,
                    &get_neighbor_block,
                ) {
                    operations::add_quad(
                        buffer,
                        operations::BlockFace::East,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        calculate_face_light(
                            x as i32 + 1,
                            y as i32,
                            z as i32,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                        calculate_face_ao(
                            x as i32 + 1,
                            y as i32,
                            z as i32,
                            operations::BlockFace::East,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                    )?;
                }

                // West face (X-)
                if should_render_face(
                    x as i32 - 1,
                    y as i32,
                    z as i32,
                    chunk_size,
                    &get_block,
                    &get_neighbor_block,
                ) {
                    operations::add_quad(
                        buffer,
                        operations::BlockFace::West,
                        world_x,
                        world_y,
                        world_z,
                        block,
                        calculate_face_light(
                            x as i32 - 1,
                            y as i32,
                            z as i32,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                        calculate_face_ao(
                            x as i32 - 1,
                            y as i32,
                            z as i32,
                            operations::BlockFace::West,
                            chunk_size,
                            &get_neighbor_block,
                        ),
                    )?;
                }
            }
        }
    }

    // Update metadata
    buffer.metadata.vertex_count = buffer.vertex_count as u32;
    buffer.metadata.index_count = buffer.index_count as u32;
    buffer.metadata.generation_time_us = start.elapsed().as_micros() as u32;

    Ok(())
}

/// Check if a face should be rendered based on neighbor blocks
fn should_render_face<F, N>(
    x: i32,
    y: i32,
    z: i32,
    chunk_size: u32,
    get_block: &F,
    get_neighbor_block: &N,
) -> bool
where
    F: Fn(u32, u32, u32) -> BlockId,
    N: Fn(i32, i32, i32) -> BlockId,
{
    // Check if position is within chunk bounds
    if x >= 0
        && x < chunk_size as i32
        && y >= 0
        && y < chunk_size as i32
        && z >= 0
        && z < chunk_size as i32
    {
        // Within chunk - use local block getter
        get_block(x as u32, y as u32, z as u32) == BlockId::AIR
    } else {
        // Outside chunk - use neighbor block getter
        get_neighbor_block(x, y, z) == BlockId::AIR
    }
}

/// Calculate lighting value for a face
fn calculate_face_light<N>(x: i32, y: i32, z: i32, chunk_size: u32, get_neighbor_block: &N) -> u8
where
    N: Fn(i32, i32, i32) -> BlockId,
{
    // Simple lighting based on Y position and sky access
    // In a real implementation, this would use the lighting system
    if y >= chunk_size as i32 {
        15 // Full skylight
    } else {
        // Check if there's a clear path to sky
        let mut sky_access = true;
        for check_y in (y + 1)..64 {
            if get_neighbor_block(x, check_y, z) != BlockId::AIR {
                sky_access = false;
                break;
            }
        }
        if sky_access {
            15
        } else {
            8
        }
    }
}

/// Calculate ambient occlusion values for a face
fn calculate_face_ao<N>(
    x: i32,
    y: i32,
    z: i32,
    face: operations::BlockFace,
    chunk_size: u32,
    get_neighbor_block: &N,
) -> [u8; 4]
where
    N: Fn(i32, i32, i32) -> BlockId,
{
    // Simple AO calculation - in production this would be more sophisticated
    // For now, return no occlusion
    [255, 255, 255, 255]
}

/// Transform chunk data and mesh into a RenderObject
pub fn chunk_to_render_object(
    chunk_pos: ChunkPos,
    mesh_buffer: &MeshBuffer,
    config: &ChunkRenderConfig,
) -> RenderObject {
    let world_pos = chunk_world_position(chunk_pos, config.chunk_size);

    RenderObject {
        position: world_pos,
        scale: config.world_scale,
        color: [1.0, 1.0, 1.0, 1.0], // Default white color
        bounding_radius: chunk_bounding_radius(config.chunk_size, config.world_scale),
        mesh_id: 0,        // Would be assigned by mesh manager
        material_id: 0,    // Would be assigned by material manager
        index_count: None, // Would be set by mesh generation
    }
}

/// Calculate world position from chunk coordinates
pub fn chunk_world_position(chunk_pos: ChunkPos, chunk_size: u32) -> Vector3<f32> {
    Vector3::new(
        (chunk_pos.x * chunk_size as i32) as f32,
        (chunk_pos.y * chunk_size as i32) as f32,
        (chunk_pos.z * chunk_size as i32) as f32,
    )
}

/// Calculate bounding radius for culling
pub fn chunk_bounding_radius(chunk_size: u32, world_scale: f32) -> f32 {
    // Radius of sphere that encompasses entire chunk
    // Diagonal of cube = sqrt(3) * side_length
    let diagonal = (3.0_f32).sqrt() * chunk_size as f32 * world_scale;
    diagonal / 2.0
}

/// Calculate squared distance between chunk and view position
pub fn chunk_distance_squared(chunk_pos: ChunkPos, view_pos: Vector3<f32>, chunk_size: u32) -> f32 {
    let chunk_center = chunk_world_position(chunk_pos, chunk_size)
        + Vector3::new(
            chunk_size as f32 / 2.0,
            chunk_size as f32 / 2.0,
            chunk_size as f32 / 2.0,
        );
    let diff = chunk_center - view_pos;
    diff.x * diff.x + diff.y * diff.y + diff.z * diff.z
}

/// Determine LOD level based on distance
pub fn calculate_chunk_lod(distance_squared: f32, lod_distances: &[f32]) -> u32 {
    for (level, &dist_sq) in lod_distances.iter().enumerate() {
        if distance_squared < dist_sq * dist_sq {
            return level as u32;
        }
    }
    lod_distances.len() as u32
}

/// Check if chunk is within view frustum (simplified)
pub fn is_chunk_in_frustum(
    chunk_pos: ChunkPos,
    chunk_size: u32,
    view_pos: Vector3<f32>,
    view_dir: Vector3<f32>,
    fov_cos: f32,
) -> bool {
    let chunk_center = chunk_world_position(chunk_pos, chunk_size)
        + Vector3::new(
            chunk_size as f32 / 2.0,
            chunk_size as f32 / 2.0,
            chunk_size as f32 / 2.0,
        );
    let to_chunk = (chunk_center - view_pos).normalize();

    // Simple frustum check using dot product
    to_chunk.dot(view_dir) > fov_cos
}

/// Calculate priority for chunk loading/rendering
pub fn chunk_render_priority(
    chunk_pos: ChunkPos,
    view_pos: Vector3<f32>,
    view_dir: Vector3<f32>,
    chunk_size: u32,
) -> f32 {
    let chunk_center = chunk_world_position(chunk_pos, chunk_size)
        + Vector3::new(
            chunk_size as f32 / 2.0,
            chunk_size as f32 / 2.0,
            chunk_size as f32 / 2.0,
        );
    let to_chunk = chunk_center - view_pos;
    let distance = to_chunk.magnitude();
    let direction_score = to_chunk.normalize().dot(view_dir);

    // Higher priority for chunks that are closer and in view direction
    let priority = (1.0 / (distance + 1.0)) * (direction_score + 1.0);
    priority
}

/// Batch transform for multiple chunks
pub fn batch_chunks_to_render_objects(
    chunks: &[(ChunkPos, &MeshBuffer)],
    config: &ChunkRenderConfig,
    output: &mut Vec<RenderObject>,
) {
    output.clear();
    output.reserve(chunks.len());

    for (chunk_pos, mesh_buffer) in chunks {
        if mesh_buffer.vertex_count > 0 {
            output.push(chunk_to_render_object(*chunk_pos, mesh_buffer, config));
        }
    }
}

/// Filter chunks for rendering based on view parameters
pub fn filter_visible_chunks(
    chunks: &[ChunkPos],
    view_pos: Vector3<f32>,
    view_dir: Vector3<f32>,
    max_distance: f32,
    fov_cos: f32,
    chunk_size: u32,
    output: &mut Vec<ChunkPos>,
) {
    output.clear();
    let max_dist_sq = max_distance * max_distance;

    for &chunk_pos in chunks {
        let dist_sq = chunk_distance_squared(chunk_pos, view_pos, chunk_size);
        if dist_sq <= max_dist_sq
            && is_chunk_in_frustum(chunk_pos, chunk_size, view_pos, view_dir, fov_cos)
        {
            output.push(chunk_pos);
        }
    }

    // Sort by render priority
    output.sort_by(|a, b| {
        let priority_a = chunk_render_priority(*a, view_pos, view_dir, chunk_size);
        let priority_b = chunk_render_priority(*b, view_pos, view_dir, chunk_size);
        match priority_b.partial_cmp(&priority_a) {
            Some(ordering) => ordering,
            None => {
                log::warn!("Invalid priority values during chunk sorting");
                std::cmp::Ordering::Equal
            }
        }
    });
}
