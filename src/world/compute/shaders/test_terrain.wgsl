// Test GPU Terrain Generation - Self-contained minimal shader
// This shader contains everything needed for basic terrain generation

struct TerrainParams {
    seed: u32,
    sea_level: f32,
    terrain_scale: f32,
    mountain_threshold: f32,
    cave_threshold: f32,
    // Note: This test shader doesn't use distributions
    // The real terrain_generation.wgsl has the full BlockDistribution support
}

struct ChunkMetadata {
    flags: u32,
    timestamp: u32,
    checksum: u32,
    reserved: u32,
}

// Block IDs
const BLOCK_AIR: u32 = 0u;
const BLOCK_GRASS: u32 = 1u;
const BLOCK_DIRT: u32 = 2u;
const BLOCK_STONE: u32 = 3u;
const BLOCK_SAND: u32 = 5u;
const BLOCK_WATER: u32 = 6u;

const CHUNK_SIZE: u32 = 32u;

// Bindings
@group(0) @binding(0) var<storage, read_write> world_voxels: array<u32>;
@group(0) @binding(1) var<storage, read_write> chunk_metadata: array<ChunkMetadata>;
@group(0) @binding(2) var<uniform> params: TerrainParams;
@group(0) @binding(3) var<storage, read> chunk_positions: array<vec4<i32>>;

// Simple noise function (simplified Perlin noise)
fn simple_noise(x: f32, z: f32) -> f32 {
    let xi = i32(floor(x));
    let zi = i32(floor(z));
    
    // Simple hash-based noise
    let hash = u32((xi * 374761393 + zi * 668265263 + i32(params.seed)) % 1073741824);
    return f32(hash) / 1073741824.0 - 0.5;
}

// Simple terrain height function
fn get_terrain_height(x: f32, z: f32) -> f32 {
    let height1 = simple_noise(x * 0.01, z * 0.01) * 32.0;
    let height2 = simple_noise(x * 0.02, z * 0.02) * 16.0;
    let height3 = simple_noise(x * 0.04, z * 0.04) * 8.0;
    
    return 64.0 + height1 + height2 + height3;
}

// Pack voxel data
fn pack_voxel(block_id: u32, light: u32, skylight: u32, metadata: u32) -> u32 {
    return block_id | (light << 16u) | (skylight << 20u) | (metadata << 24u);
}

@compute @workgroup_size(8, 4, 4)
fn generate_chunk(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let chunk_idx = workgroup_id.x;
    if (chunk_idx >= arrayLength(&chunk_positions)) {
        return;
    }
    
    let chunk_pos = chunk_positions[chunk_idx];
    let chunk_world_x = chunk_pos.x * i32(CHUNK_SIZE);
    let chunk_world_y = chunk_pos.y * i32(CHUNK_SIZE);
    let chunk_world_z = chunk_pos.z * i32(CHUNK_SIZE);
    
    let local_x = local_id.x;
    let local_y = local_id.y; 
    let local_z = local_id.z;
    
    // Each thread processes a 4x4x4 block
    for (var dx = 0u; dx < 4u; dx++) {
        for (var dy = 0u; dy < 4u; dy++) {
            for (var dz = 0u; dz < 4u; dz++) {
                let x = local_x * 4u + dx;
                let y = local_y * 4u + dy;
                let z = local_z * 4u + dz;
                
                if (x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE) {
                    continue;
                }
                
                let world_x = f32(chunk_world_x + i32(x));
                let world_y = f32(chunk_world_y + i32(y));
                let world_z = f32(chunk_world_z + i32(z));
                
                // Generate terrain
                var block_id = BLOCK_AIR;
                var skylight = 15u;
                
                let height = get_terrain_height(world_x, world_z);
                
                if (world_y < height) {
                    skylight = 0u;
                    if (world_y < height - 4.0) {
                        block_id = BLOCK_STONE;
                    } else {
                        block_id = BLOCK_DIRT;
                    }
                } else if (world_y == floor(height)) {
                    if (height < params.sea_level + 2.0) {
                        block_id = BLOCK_SAND;
                    } else {
                        block_id = BLOCK_GRASS;
                    }
                } else if (world_y < params.sea_level) {
                    block_id = BLOCK_WATER;
                    skylight = max(0u, 15u - u32((params.sea_level - world_y) * 0.5));
                }
                
                let buffer_index = chunk_idx * CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE + 
                                  x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE;
                
                if (buffer_index < arrayLength(&world_voxels)) {
                    world_voxels[buffer_index] = pack_voxel(block_id, 0u, skylight, 0u);
                }
            }
        }
    }
    
    // Update metadata
    if (all(local_id == vec3<u32>(0u, 0u, 0u))) {
        if (chunk_idx < arrayLength(&chunk_metadata)) {
            chunk_metadata[chunk_idx].flags = 1u;
            chunk_metadata[chunk_idx].timestamp = 0u;
        }
    }
}