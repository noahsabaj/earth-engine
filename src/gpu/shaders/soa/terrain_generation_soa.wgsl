// SOA-optimized terrain generation compute shader
// Uses Structure of Arrays for maximum GPU performance

#include "../generated/types_soa.wgsl"

// Include noise functions
#include "../../../renderer/shaders/perlin_noise.wgsl"

@group(0) @binding(0) var<storage, read_write> world_data: array<u32>;
@group(0) @binding(1) var<storage, read> metadata: array<ChunkMetadata>;
@group(0) @binding(2) var<storage, read> params: TerrainParamsSOA;

// Constants
const CHUNK_SIZE: u32 = 32u;
const CHUNK_SIZE_F: f32 = 32.0;


// Get voxel at specific position using SOA data
fn get_voxel_soa(world_pos: vec3<i32>, params: ptr<storage, TerrainParamsSOA>) -> u32 {
    let pos_f = vec3<f32>(f32(world_pos.x), f32(world_pos.y), f32(world_pos.z));
    
    // Use 2D noise for terrain height (not 3D)
    let height_noise = fbm2d(
        pos_f.x * (*params).terrain_scale, 
        pos_f.z * (*params).terrain_scale,
        6,     // octaves
        2.0,   // lacunarity
        0.5    // persistence
    );
    
    // Convert noise (-1 to 1) to height with base at sea level
    let base_height = i32((*params).sea_level + height_noise * 32.0);
    
    // Cave generation using 3D noise
    let cave_noise = perlin3d(pos_f.x * 0.05, pos_f.y * 0.05, pos_f.z * 0.05);
    let is_cave = cave_noise > (*params).cave_threshold && world_pos.y < base_height - 5;
    
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
fn generate_terrain(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    // For single chunk generation, chunk index is 0
    let chunk_idx = 0u;
    if (chunk_idx >= arrayLength(&metadata)) {
        return;
    }
    
    let chunk_meta = metadata[chunk_idx];
    
    // Calculate local position within chunk
    let local_pos = local_id + workgroup_id * vec3<u32>(8u, 8u, 8u);
    
    // Check bounds
    if (local_pos.x >= CHUNK_SIZE || local_pos.y >= CHUNK_SIZE || local_pos.z >= CHUNK_SIZE) {
        return;
    }
    
    // Extract chunk position from metadata
    let chunk_x = i32((chunk_meta.flags >> 16) & 0xFFFF);
    let chunk_z = i32(chunk_meta.flags & 0xFFFF);
    let chunk_y = i32(chunk_meta.reserved); // Y stored in reserved field
    
    // Sign extend if negative
    let chunk_x_signed = select(chunk_x, chunk_x - 65536, chunk_x > 32767);
    let chunk_z_signed = select(chunk_z, chunk_z - 65536, chunk_z > 32767);
    
    let chunk_offset = vec3<i32>(chunk_x_signed, chunk_y, chunk_z_signed);
    let world_pos = vec3<i32>(local_pos) + chunk_offset * i32(CHUNK_SIZE);
    
    // Generate voxel using SOA data
    let voxel = get_voxel_soa(world_pos, &params);
    
    // Calculate linear index
    let index = local_pos.x + local_pos.y * CHUNK_SIZE + local_pos.z * CHUNK_SIZE * CHUNK_SIZE;
    
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