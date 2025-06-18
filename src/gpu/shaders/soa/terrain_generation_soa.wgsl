// SOA-optimized terrain generation compute shader
// Uses Structure of Arrays for maximum GPU performance

#include "../generated/types_soa.wgsl"

// Include noise functions
#include "../../../renderer/shaders/perlin_noise.wgsl"

@group(0) @binding(0) var<storage, read_write> world_data: array<u32>;
@group(0) @binding(1) var<storage, read> metadata: ChunkMetadata;
@group(0) @binding(2) var<storage, read> params: TerrainParamsSOA;

// Constants
const CHUNK_SIZE: u32 = 32u;
const CHUNK_SIZE_F: f32 = 32.0;


// Get voxel at specific position using SOA data
fn get_voxel_soa(world_pos: vec3<i32>, params: ptr<storage, TerrainParamsSOA>) -> u32 {
    let pos_f = vec3<f32>(f32(world_pos.x), f32(world_pos.y), f32(world_pos.z));
    
    // Base terrain height
    let height_noise = perlin3d(pos_f.x * (*params).terrain_scale, pos_f.y * (*params).terrain_scale, pos_f.z * (*params).terrain_scale);
    let base_height = i32(height_noise * 64.0);
    
    // Cave generation
    let cave_noise = perlin3d(pos_f.x * 0.05, pos_f.y * 0.05, pos_f.z * 0.05);
    let is_cave = cave_noise > (*params).cave_threshold;
    
    // Basic terrain rules
    if (world_pos.y > base_height || is_cave) {
        return 0u; // Air
    }
    
    // Check custom block distributions using SOA
    let custom_block = check_height_soa(&(*params).distributions, world_pos.y);
    if (custom_block != 0u) {
        let distribution_noise = perlin3d(pos_f.x * 0.1, pos_f.y * 0.1, pos_f.z * 0.1);
        let dist_index = find_distribution_index_soa(&(*params).distributions, custom_block);
        
        if (dist_index < (*params).distributions.count) {
            let probability = (*params).distributions.probabilities[dist_index];
            let threshold = (*params).distributions.noise_thresholds[dist_index];
            
            if (distribution_noise < probability && distribution_noise > threshold) {
                return custom_block;
            }
        }
    }
    
    // Default blocks based on height
    if (world_pos.y > i32((*params).sea_level) - 5) {
        return 3u; // Grass
    } else if (world_pos.y > i32((*params).sea_level) - 10) {
        return 2u; // Dirt
    } else {
        return 1u; // Stone
    }
}

// Find distribution index for a given block ID (SOA optimized)
fn find_distribution_index_soa(distributions: ptr<storage, BlockDistributionSOA>, block_id: u32) -> u32 {
    let count = (*distributions).count;
    
    // Linear search optimized for GPU (could be vectorized further)
    for (var i = 0u; i < count; i++) {
        if ((*distributions).block_ids[i] == block_id) {
            return i;
        }
    }
    
    return count; // Not found
}

// Main compute kernel
@compute @workgroup_size(8, 8, 8)
fn generate_terrain(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Check bounds
    if (global_id.x >= CHUNK_SIZE || global_id.y >= CHUNK_SIZE || global_id.z >= CHUNK_SIZE) {
        return;
    }
    
    // Calculate world position
    let chunk_offset = vec3<i32>(
        metadata.flags >> 16,  // Chunk X stored in upper 16 bits
        0,                     // Y is always 0 for now
        metadata.flags & 0xFFFF // Chunk Z stored in lower 16 bits
    );
    
    let world_pos = vec3<i32>(global_id) + chunk_offset * i32(CHUNK_SIZE);
    
    // Generate voxel using SOA data
    let voxel = get_voxel_soa(world_pos, &params);
    
    // Calculate linear index
    let index = global_id.x + global_id.y * CHUNK_SIZE + global_id.z * CHUNK_SIZE * CHUNK_SIZE;
    
    // Store result
    world_data[index] = voxel;
}

// Vectorized compute kernel for even better performance
@compute @workgroup_size(8, 8, 8)
fn generate_terrain_vectorized(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Process 4 voxels at once when possible
    let base_x = global_id.x * 4u;
    
    if (base_x + 3u >= CHUNK_SIZE || global_id.y >= CHUNK_SIZE || global_id.z >= CHUNK_SIZE) {
        // Fall back to scalar version for edge cases
        return;
    }
    
    // Calculate world positions for 4 voxels
    let chunk_offset = vec3<i32>(
        metadata.flags >> 16,
        0,
        metadata.flags & 0xFFFF
    );
    
    // Generate 4 voxels at once
    for (var i = 0u; i < 4u; i++) {
        let local_pos = vec3<u32>(base_x + i, global_id.y, global_id.z);
        let world_pos = vec3<i32>(local_pos) + chunk_offset * i32(CHUNK_SIZE);
        
        let voxel = get_voxel_soa(world_pos, &params);
        let index = (base_x + i) + global_id.y * CHUNK_SIZE + global_id.z * CHUNK_SIZE * CHUNK_SIZE;
        
        world_data[index] = voxel;
    }
}