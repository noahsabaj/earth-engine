// GPU Block Query Compute Shader
// Performs high-performance block queries directly on GPU without CPU transfers

struct BlockQueryRequest {
    position: vec3<i32>,
    query_type: u32,
}

struct BlockQueryResult {
    position: vec3<i32>,
    query_type: u32,
    value: u32,
    success: u32,
    _padding: vec2<u32>,
}

struct PushConstants {
    query_count: u32,
    chunk_size: u32,
}

// World buffer binding
@group(0) @binding(0)
var<storage, read> world_voxels: array<u32>;

// Query requests
@group(0) @binding(1)
var<storage, read> queries: array<BlockQueryRequest>;

// Query results
@group(0) @binding(2)
var<storage, read_write> results: array<BlockQueryResult>;

var<push_constant> constants: PushConstants;

// Constants matching CPU side
const WORLD_SIZE: u32 = 256u;
const MAX_HEIGHT: u32 = 256u;

// Extract block ID from packed voxel data
fn extract_block_id(voxel_data: u32) -> u32 {
    return voxel_data & 0xFFFFu;
}

// Extract light level from packed voxel data
fn extract_light_level(voxel_data: u32) -> u32 {
    return (voxel_data >> 16u) & 0xFu;
}

// Extract sky light level from packed voxel data
fn extract_sky_light_level(voxel_data: u32) -> u32 {
    return (voxel_data >> 20u) & 0xFu;
}

// Extract metadata from packed voxel data
fn extract_metadata(voxel_data: u32) -> u32 {
    return (voxel_data >> 24u) & 0xFu;
}

// Morton encoding functions from unified GPU system
#include "morton.wgsl"

// Calculate world buffer index for a world position
fn world_position_to_buffer_index(world_pos: vec3<i32>) -> u32 {
    // Convert world position to chunk position
    let chunk_pos = world_pos / i32(constants.chunk_size);
    
    // Check bounds
    if (chunk_pos.x < 0 || chunk_pos.x >= i32(WORLD_SIZE) ||
        chunk_pos.y < 0 || chunk_pos.y >= i32(MAX_HEIGHT / constants.chunk_size) ||
        chunk_pos.z < 0 || chunk_pos.z >= i32(WORLD_SIZE)) {
        return 0xFFFFFFFFu; // Invalid index
    }
    
    // Calculate chunk index in world
    let chunk_index = u32(chunk_pos.x) + 
                     u32(chunk_pos.z) * WORLD_SIZE + 
                     u32(chunk_pos.y) * WORLD_SIZE * WORLD_SIZE;
    
    // Calculate local position within chunk
    let local_pos = world_pos - (chunk_pos * i32(constants.chunk_size));
    
    // Morton encode local position
    let local_index = morton_encode_3d(
        u32(local_pos.x),
        u32(local_pos.y),
        u32(local_pos.z)
    );
    
    // Calculate final buffer index
    let voxels_per_chunk = constants.chunk_size * constants.chunk_size * constants.chunk_size;
    return chunk_index * voxels_per_chunk + local_index;
}

@compute @workgroup_size(256)
fn query_blocks(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let query_idx = global_id.x;
    
    // Check bounds
    if (query_idx >= constants.query_count) {
        return;
    }
    
    // Get query request
    let request = queries[query_idx];
    
    // Initialize result
    var result: BlockQueryResult;
    result.position = request.position;
    result.query_type = request.query_type;
    result.value = 0u;
    result.success = 0u;
    
    // Calculate buffer index
    let buffer_idx = world_position_to_buffer_index(request.position);
    
    // Check if valid
    if (buffer_idx != 0xFFFFFFFFu && buffer_idx < arrayLength(&world_voxels)) {
        let voxel_data = world_voxels[buffer_idx];
        
        // Extract requested data based on query type
        switch (request.query_type) {
            case 0u: { // Get block ID
                result.value = extract_block_id(voxel_data);
                result.success = 1u;
            }
            case 1u: { // Get light level
                result.value = extract_light_level(voxel_data);
                result.success = 1u;
            }
            case 2u: { // Get sky light level
                result.value = extract_sky_light_level(voxel_data);
                result.success = 1u;
            }
            case 3u: { // Get metadata
                result.value = extract_metadata(voxel_data);
                result.success = 1u;
            }
            case 4u: { // Get full voxel data
                result.value = voxel_data;
                result.success = 1u;
            }
            default: {
                // Invalid query type
                result.success = 0u;
            }
        }
    }
    
    // Write result
    results[query_idx] = result;
}