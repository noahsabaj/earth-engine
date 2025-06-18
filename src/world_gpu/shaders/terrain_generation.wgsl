// GPU Terrain Generation Compute Shader
// Generates realistic terrain using Perlin noise directly in the WorldBuffer
// Updated to use SOA (Structure of Arrays) types for optimal GPU performance

// Import auto-generated GPU types (SOA)
#include "../../gpu/shaders/generated/constants.wgsl"
#include "../../gpu/shaders/generated/types_soa.wgsl"

// Block IDs and constants are now included from generated/constants.wgsl
// This ensures single source of truth and consistency with Rust code

// Bindings
@group(0) @binding(0) var<storage, read_write> world_voxels: array<u32>;
@group(0) @binding(1) var<storage, read_write> chunk_metadata: array<ChunkMetadata>;
@group(0) @binding(2) var<uniform> params: TerrainParamsSOA;
@group(0) @binding(3) var<storage, read> chunk_positions: array<vec4<i32>>;

// Helper functions
fn pack_voxel(block_id: u32, light: u32, skylight: u32, metadata: u32) -> u32 {
    return block_id | (light << 16u) | (skylight << 20u) | (metadata << 24u);
}

// Main chunk generation kernel
@compute @workgroup_size(8, 4, 4)
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
    
    // Calculate local position for this thread
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
                
                // Calculate world positions
                let world_x = f32(chunk_world_x + i32(x));
                let world_y = f32(chunk_world_y + i32(y));
                let world_z = f32(chunk_world_z + i32(z));
                
                // Generate terrain using Perlin noise functions
                var block_id = BLOCK_AIR;
                var skylight = 15u; // Full skylight by default
                
                // Get terrain height using noise
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
                            
                            // Check custom block distributions using SOA layout for better performance
                            let custom_block = check_height_soa(&params.distributions, i32(world_y));
                            if (custom_block != 0u) {
                                // Find distribution index for this block
                                for (var i = 0u; i < params.distributions.count; i++) {
                                    if (params.distributions.block_ids[i] == custom_block) {
                                        // Use noise for distribution
                                        let block_noise = noise3d(
                                            world_x * 0.1, 
                                            world_y * 0.1, 
                                            world_z * 0.1
                                        );
                                        
                                        // Check probability and noise threshold using SOA arrays
                                        if (block_noise > params.distributions.noise_thresholds[i]) {
                                            let chance = hash_float(u32(world_x) * 73856093u ^ u32(world_y) * 19349663u ^ u32(world_z) * 83492791u);
                                            if (chance < params.distributions.probabilities[i]) {
                                                block_id = custom_block;
                                                break; // First matching distribution wins
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        } else {
                            block_id = BLOCK_DIRT;
                        }
                    } else {
                        // Cave - check if flooded
                        if (world_y < params.sea_level) {
                            block_id = BLOCK_WATER;
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
                
                // CRITICAL FIX: Use actual slot index from chunk_positions.w instead of sequential chunk_idx
                // This fixes the buffer index mismatch where compute shader assumed sequential layout
                // but WorldBuffer uses slot-based allocation
                let slot = u32(chunk_pos.w);  // Slot index packed in 4th component
                let buffer_index = slot * CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE + 
                                  x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE;
                
                // Write voxel data
                if (buffer_index < arrayLength(&world_voxels)) {
                    world_voxels[buffer_index] = pack_voxel(block_id, 0u, skylight, 0u);
                }
            }
        }
    }
    
    // Update metadata using slot index
    if (all(local_id == vec3<u32>(0u, 0u, 0u))) {
        let slot = u32(chunk_pos.w);  // Use same slot index from positions
        if (slot < arrayLength(&chunk_metadata)) {
            chunk_metadata[slot].flags = 1u; // Mark as generated
            chunk_metadata[slot].timestamp = 0u;
        }
    }
}

// Simple hash-based noise function for terrain generation
fn hash(n: i32) -> f32 {
    let x = u32(n * 374761393 + i32(params.seed));
    let h = (x * x * x) % 1073741824u;
    return f32(h) / 1073741824.0;
}

// Hash function for u32 input, returns float in range [0, 1]
fn hash_float(n: u32) -> f32 {
    let x = n * 1103515245u + 12345u;
    return f32((x / 65536u) % 32768u) / 32768.0;
}

// 2D noise function for terrain height
fn noise2d(x: f32, z: f32) -> f32 {
    let xi = i32(floor(x));
    let zi = i32(floor(z));
    let xf = fract(x);
    let zf = fract(z);
    
    // Get corner values
    let a = hash(xi + zi * 57);
    let b = hash(xi + 1 + zi * 57);
    let c = hash(xi + (zi + 1) * 57);
    let d = hash(xi + 1 + (zi + 1) * 57);
    
    // Smooth interpolation
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = zf * zf * (3.0 - 2.0 * zf);
    
    return mix(mix(a, b, u), mix(c, d, u), v);
}

// 3D noise function for caves
fn noise3d(x: f32, y: f32, z: f32) -> f32 {
    let xi = i32(floor(x));
    let yi = i32(floor(y));
    let zi = i32(floor(z));
    
    let xf = fract(x);
    let yf = fract(y);
    let zf = fract(z);
    
    // 8 corner hash values
    let h000 = hash(xi + yi * 57 + zi * 113);
    let h001 = hash(xi + yi * 57 + (zi + 1) * 113);
    let h010 = hash(xi + (yi + 1) * 57 + zi * 113);
    let h011 = hash(xi + (yi + 1) * 57 + (zi + 1) * 113);
    let h100 = hash((xi + 1) + yi * 57 + zi * 113);
    let h101 = hash((xi + 1) + yi * 57 + (zi + 1) * 113);
    let h110 = hash((xi + 1) + (yi + 1) * 57 + zi * 113);
    let h111 = hash((xi + 1) + (yi + 1) * 57 + (zi + 1) * 113);
    
    // Smooth interpolation
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let w = zf * zf * (3.0 - 2.0 * zf);
    
    let x00 = mix(h000, h100, u);
    let x01 = mix(h001, h101, u);
    let x10 = mix(h010, h110, u);
    let x11 = mix(h011, h111, u);
    
    let y0 = mix(x00, x10, v);
    let y1 = mix(x01, x11, v);
    
    return mix(y0, y1, w);
}

// Calculate terrain height using octaves of noise
fn terrain_height(x: f32, z: f32) -> f32 {
    var height = 0.0;
    var amplitude = 320.0; // Mountain amplitude (32m × 10 voxels/m = 320 voxels)
    var frequency = params.terrain_scale;
    
    // Multiple octaves for more realistic terrain
    for (var i = 0; i < 4; i++) {
        height += noise2d(x * frequency, z * frequency) * amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    
    return params.sea_level + height;
}

// Calculate cave density
fn cave_density(x: f32, y: f32, z: f32) -> f32 {
    let scale = 0.03;
    return noise3d(x * scale, y * scale, z * scale);
}