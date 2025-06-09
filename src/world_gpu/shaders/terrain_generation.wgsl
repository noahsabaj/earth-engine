// GPU Terrain Generation Compute Shader
// Generates chunks directly in the WorldBuffer using Perlin noise

struct TerrainParams {
    seed: u32,
    sea_level: f32,
    terrain_scale: f32,
    mountain_threshold: f32,
    cave_threshold: f32,
    ore_chances: vec4<f32>, // Coal, Iron, Gold, Diamond
}

struct ChunkMetadata {
    flags: u32,         // Bit 0: generated, Bit 1: modified, etc.
    timestamp: u32,     // Generation timestamp
    checksum: u32,      // For validation
    reserved: u32,
}

// Voxel data is packed as u32
const BLOCK_ID_MASK: u32 = 0xFFFFu;
const LIGHT_SHIFT: u32 = 16u;
const SKYLIGHT_SHIFT: u32 = 20u;
const METADATA_SHIFT: u32 = 24u;

// Block IDs
const BLOCK_AIR: u32 = 0u;
const BLOCK_STONE: u32 = 1u;
const BLOCK_DIRT: u32 = 2u;
const BLOCK_GRASS: u32 = 3u;
const BLOCK_SAND: u32 = 4u;
const BLOCK_WATER: u32 = 5u;
const BLOCK_COAL_ORE: u32 = 16u;
const BLOCK_IRON_ORE: u32 = 17u;
const BLOCK_GOLD_ORE: u32 = 18u;
const BLOCK_DIAMOND_ORE: u32 = 19u;
const BLOCK_BEDROCK: u32 = 32u;

// World constants
const CHUNK_SIZE: u32 = 32u;
const WORLD_SIZE: u32 = 512u;
const WORLD_HEIGHT: u32 = 256u;

// Bindings
@group(0) @binding(0) var<storage, read_write> world_voxels: array<u32>;
@group(0) @binding(1) var<storage, read_write> chunk_metadata: array<ChunkMetadata>;
@group(0) @binding(2) var<uniform> params: TerrainParams;
@group(0) @binding(3) var<storage, read> chunk_positions: array<vec4<i32>>;

// Helper functions
fn pack_voxel(block_id: u32, light: u32, skylight: u32, metadata: u32) -> u32 {
    return block_id | (light << LIGHT_SHIFT) | (skylight << SKYLIGHT_SHIFT) | (metadata << METADATA_SHIFT);
}

fn world_to_chunk_index(world_x: i32, world_y: i32, world_z: i32) -> u32 {
    let chunk_x = u32(world_x >> 5); // div by 32
    let chunk_y = u32(world_y >> 5);
    let chunk_z = u32(world_z >> 5);
    return chunk_x + chunk_y * WORLD_SIZE + chunk_z * WORLD_SIZE * WORLD_SIZE;
}

fn chunk_to_voxel_offset(chunk_index: u32, local_x: u32, local_y: u32, local_z: u32) -> u32 {
    let chunk_offset = chunk_index * CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    let local_index = local_x + local_y * CHUNK_SIZE + local_z * CHUNK_SIZE * CHUNK_SIZE;
    return chunk_offset + local_index;
}

// Main chunk generation kernel
@compute @workgroup_size(8, 8, 8)
fn generate_chunk(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    // Get chunk to generate
    let chunk_idx = workgroup_id.x;
    if (chunk_idx >= arrayLength(&chunk_positions)) {
        return;
    }
    
    let chunk_pos = chunk_positions[chunk_idx];
    let chunk_world_x = chunk_pos.x * i32(CHUNK_SIZE);
    let chunk_world_y = chunk_pos.y * i32(CHUNK_SIZE);
    let chunk_world_z = chunk_pos.z * i32(CHUNK_SIZE);
    
    // Calculate world position for this thread
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
                
                // Generate voxel
                var block_id = BLOCK_AIR;
                var light = 0u;
                var skylight = 15u; // Full skylight by default
                
                // Bedrock layer
                if (world_y < 3.0) {
                    block_id = BLOCK_BEDROCK;
                    skylight = 0u;
                } else {
                    // Get terrain height
                    let height = terrain_height(world_x, world_z);
                    
                    if (world_y < height) {
                        // Below surface
                        skylight = 0u;
                        
                        // Check for caves
                        let cave = cave_density(world_x, world_y, world_z);
                        if (cave < params.cave_threshold) {
                            // Solid block
                            if (world_y < height - 4.0) {
                                block_id = BLOCK_STONE;
                                
                                // Ore generation
                                let ore_noise = perlin3d(
                                    world_x * 0.1 + f32(params.seed),
                                    world_y * 0.1,
                                    world_z * 0.1
                                );
                                
                                if (ore_noise > 0.9 && world_y < 64.0) {
                                    // Determine ore type based on depth
                                    if (world_y < 16.0 && ore_noise > 0.98) {
                                        block_id = BLOCK_DIAMOND_ORE;
                                    } else if (world_y < 32.0 && ore_noise > 0.95) {
                                        block_id = BLOCK_GOLD_ORE;
                                    } else if (world_y < 64.0 && ore_noise > 0.92) {
                                        block_id = BLOCK_IRON_ORE;
                                    } else {
                                        block_id = BLOCK_COAL_ORE;
                                    }
                                }
                            } else {
                                block_id = BLOCK_DIRT;
                            }
                        } else {
                            // Cave - check if flooded
                            if (world_y < params.sea_level) {
                                block_id = BLOCK_WATER;
                                light = 0u;
                            }
                        }
                    } else if (world_y == floor(height)) {
                        // Surface layer
                        if (height < params.sea_level + 2.0) {
                            block_id = BLOCK_SAND;
                        } else {
                            block_id = BLOCK_GRASS;
                        }
                    } else if (world_y < params.sea_level) {
                        // Water
                        block_id = BLOCK_WATER;
                        skylight = max(0u, 15u - u32((params.sea_level - world_y) * 0.5));
                    }
                }
                
                // Calculate voxel index
                let chunk_index = world_to_chunk_index(
                    chunk_world_x,
                    chunk_world_y,
                    chunk_world_z
                );
                let voxel_offset = chunk_to_voxel_offset(chunk_index, x, y, z);
                
                // Write voxel data
                world_voxels[voxel_offset] = pack_voxel(block_id, light, skylight, 0u);
            }
        }
    }
    
    // One thread per workgroup updates metadata
    if (all(local_id == vec3<u32>(0u, 0u, 0u))) {
        let chunk_index = world_to_chunk_index(
            chunk_world_x,
            chunk_world_y,
            chunk_world_z
        );
        chunk_metadata[chunk_index].flags = 1u; // Mark as generated
        chunk_metadata[chunk_index].timestamp = 0u; // TODO: Add timestamp
    }
}