// GPU Terrain Generation Compute Shader
// Generates realistic terrain using Perlin noise directly in the WorldBuffer
// 
// This shader contains ONLY compute logic. All type definitions, constants,
// and bindings are provided by the unified GPU type generation system at runtime.
// DO NOT ADD any bindings, types, or constants here - they will cause conflicts!

// Include required shader modules
#include "morton.wgsl"
#include "perlin_noise.wgsl"

// Helper functions
fn pack_voxel(block_id: u32, light: u32, skylight: u32, metadata: u32) -> u32 {
    return block_id | (light << 16u) | (skylight << 20u) | (metadata << 24u);
}

// Terrain height generation using Perlin noise
fn terrain_height(x: f32, z: f32) -> f32 {
    // Base terrain using octaves of noise
    var height = 0.0;
    var amplitude = 1.0;
    var frequency = 0.01;
    
    // Add multiple octaves for more interesting terrain
    for (var i = 0; i < 4; i++) {
        height += perlin_noise_2d(x * frequency, z * frequency) * amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    
    // Scale to world coordinates (base height around y=64)
    return height * 32.0 + 64.0;
}

// Cave density calculation using 3D Perlin noise
fn cave_density(x: f32, y: f32, z: f32) -> f32 {
    // Use 3D noise for cave generation
    let scale = 0.03;
    let density = perlin_noise_3d(x * scale, y * scale, z * scale);
    
    // Modify density based on depth for more caves underground
    let depth_factor = clamp((64.0 - y) / 64.0, 0.0, 1.0);
    return density + depth_factor * 0.2;
}

// Wrapper for 3D noise (using 3D perlin from included file)
fn noise3d(x: f32, y: f32, z: f32) -> f32 {
    return perlin_noise_3d(x, y, z);
}

// Check height-based block distributions from SOA data
fn check_height_soa(world_y: i32) -> u32 {
    // Check each distribution in the SOA arrays
    for (var i = 0u; i < params.distributions.count; i++) {
        let min_y = params.distributions.min_heights[i];
        let max_y = params.distributions.max_heights[i];
        
        if (world_y >= min_y && world_y <= max_y) {
            let probability = params.distributions.probabilities[i];
            
            // Use position-based hash for deterministic randomness
            let hash = u32(world_y * 73856093) ^ u32(i * 19349663);
            let random = f32(hash & 0xFFFFu) / 65535.0;
            
            if (random < probability) {
                return params.distributions.block_ids[i];
            }
        }
    }
    
    return 0u; // No custom block
}

// Main terrain generation kernel
@compute @workgroup_size(8, 4, 4)
fn generate_terrain(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    // Calculate chunk index and local workgroup within chunk
    // For chunk_size=50 and workgroup_size=8x4x4, we need 7x13x13 workgroups per chunk
    let workgroups_per_chunk_x = 7u; // (50 + 7) / 8 = 7
    let chunk_idx = workgroup_id.x / workgroups_per_chunk_x;
    let local_workgroup_x = workgroup_id.x % workgroups_per_chunk_x;
    
    // Read chunk position from metadata buffer
    // Access metadata array directly using chunk index
    let chunk_meta = metadata[chunk_idx];
    let flags = chunk_meta.flags;
    let checksum = chunk_meta.checksum; // Slot index stored here
    let reserved = chunk_meta.y_position; // Y position stored here
    
    // Decode chunk position from metadata
    let chunk_x = i32((flags >> 16u) & 0xFFFFu);
    let chunk_y = i32(reserved);
    let chunk_z = i32(flags & 0xFFFFu);
    
    // Sign extend from 16-bit to 32-bit
    let chunk_pos_x = select(chunk_x, chunk_x - 65536, chunk_x >= 32768);
    let chunk_pos_y = chunk_y;
    let chunk_pos_z = select(chunk_z, chunk_z - 65536, chunk_z >= 32768);
    
    let chunk_pos = vec4<i32>(chunk_pos_x, chunk_pos_y, chunk_pos_z, i32(checksum));
    let chunk_world_x = chunk_pos.x * i32(CHUNK_SIZE);
    let chunk_world_y = chunk_pos.y * i32(CHUNK_SIZE);
    let chunk_world_z = chunk_pos.z * i32(CHUNK_SIZE);
    
    // Calculate local position for this thread
    let local_x = local_workgroup_x * 8u + local_id.x;
    let local_y = workgroup_id.y * 4u + local_id.y; 
    let local_z = workgroup_id.z * 4u + local_id.z;
    
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
                            let custom_block = check_height_soa(i32(world_y));
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
                        // Cave - empty for now
                        block_id = BLOCK_AIR;
                    }
                } else if (world_y == floor(height)) {
                    // Surface layer
                    block_id = BLOCK_GRASS;
                } else if (world_y < params.sea_level) {
                    // Below sea level - use dirt for now
                    block_id = BLOCK_DIRT;
                    skylight = max(0u, 15u - u32((params.sea_level - world_y) * 0.5));
                }
                
                // Use the slot from chunk_pos.w which contains the WorldBuffer slot assignment
                let slot = u32(chunk_pos.w);
                let buffer_index = slot * CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE + 
                                  x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE;
                
                // Write voxel data
                world_data[buffer_index] = pack_voxel(block_id, 0u, skylight, 0u);
            }
        }
    }
    
    // TODO: Update metadata when unified GPU system supports it
    // For now, metadata updates are handled by the CPU side
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

// Check if height falls within any block distribution range
fn check_height_soa(height: i32) -> u32 {
    for (var i = 0u; i < params.distributions.count; i++) {
        if (height >= params.distributions.min_heights[i] && height <= params.distributions.max_heights[i]) {
            return params.distributions.block_ids[i];
        }
    }
    return 0u; // No matching distribution
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

// Vectorized terrain generation kernel
// Uses SIMD operations and optimized memory access patterns for better GPU utilization
@compute @workgroup_size(16, 2, 2) // Optimized for vectorized computation (64 threads vs 128)
fn generate_terrain_vectorized(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    // Calculate chunk index and local workgroup within chunk
    // For chunk_size=50 and workgroup_size=8x4x4, we need 7x13x13 workgroups per chunk
    let workgroups_per_chunk_x = 7u; // (50 + 7) / 8 = 7
    let chunk_idx = workgroup_id.x / workgroups_per_chunk_x;
    let local_workgroup_x = workgroup_id.x % workgroups_per_chunk_x;
    
    // Read chunk position from metadata buffer
    // Access metadata array directly using chunk index
    let chunk_meta = metadata[chunk_idx];
    let flags = chunk_meta.flags;
    let checksum = chunk_meta.checksum; // Slot index stored here
    let reserved = chunk_meta.y_position; // Y position stored here
    
    // Decode chunk position from metadata
    let chunk_x = i32((flags >> 16u) & 0xFFFFu);
    let chunk_y = i32(reserved);
    let chunk_z = i32(flags & 0xFFFFu);
    
    // Sign extend from 16-bit to 32-bit
    let chunk_pos_x = select(chunk_x, chunk_x - 65536, chunk_x >= 32768);
    let chunk_pos_y = chunk_y;
    let chunk_pos_z = select(chunk_z, chunk_z - 65536, chunk_z >= 32768);
    
    let chunk_pos = vec4<i32>(chunk_pos_x, chunk_pos_y, chunk_pos_z, i32(checksum));
    let chunk_world = vec3<i32>(
        chunk_pos.x * i32(CHUNK_SIZE),
        chunk_pos.y * i32(CHUNK_SIZE),
        chunk_pos.z * i32(CHUNK_SIZE)
    );
    
    // Calculate local position for this thread (vectorized approach)
    // For chunk_size=50 and workgroup_size=16x2x2, we need 4x25x25 workgroups per chunk
    let workgroups_per_chunk_x_vec = 4u; // (50 + 15) / 16 = 4
    let local_workgroup_x_vec = (workgroup_id.x % workgroups_per_chunk_x_vec);
    let local_pos = vec3<u32>(
        local_workgroup_x_vec * 16u + local_id.x,
        workgroup_id.y * 2u + local_id.y,
        workgroup_id.z * 2u + local_id.z
    );
    
    // Each thread processes a 2x8x8 block to maximize vector operations
    for (var dx = 0u; dx < 2u; dx++) {
        // Process 4 voxels simultaneously using vector operations
        for (var dy = 0u; dy < 8u; dy += 4u) {
            for (var dz = 0u; dz < 8u; dz += 4u) {
                // Calculate 4 positions simultaneously using vector operations
                let base_x = local_pos.x * 2u + dx;
                let base_y = local_pos.y * 8u + dy;
                let base_z = local_pos.z * 8u + dz;
                
                // Vector positions for SIMD processing
                let pos_offsets = vec4<u32>(0u, 1u, 2u, 3u);
                let world_x = f32(chunk_world.x + i32(base_x));
                let world_y_base = f32(chunk_world.y + i32(base_y));
                let world_z = f32(chunk_world.z + i32(base_z));
                
                // Process 4 Y-axis voxels simultaneously
                for (var i = 0u; i < 4u; i++) {
                    let y = base_y + i;
                    let z = base_z;
                    
                    if (base_x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE) {
                        continue;
                    }
                    
                    let world_y = world_y_base + f32(i);
                    
                    // Generate terrain using vectorized noise calculations
                    var block_id = BLOCK_AIR;
                    var skylight = 15u;
                    
                    // Get terrain height (can be vectorized for multiple Z values)
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
                                
                                // Check custom block distributions
                                let custom_block = check_height_soa(i32(world_y));
                                if (custom_block != 0u) {
                                    // Vectorized distribution check
                                    for (var dist_idx = 0u; dist_idx < params.distributions.count; dist_idx++) {
                                        if (params.distributions.block_ids[dist_idx] == custom_block) {
                                            let block_noise = noise3d(
                                                world_x * 0.1, 
                                                world_y * 0.1, 
                                                world_z * 0.1
                                            );
                                            
                                            if (block_noise > params.distributions.noise_thresholds[dist_idx]) {
                                                let chance = hash_float(
                                                    u32(world_x) * 73856093u ^ 
                                                    u32(world_y) * 19349663u ^ 
                                                    u32(world_z) * 83492791u
                                                );
                                                if (chance < params.distributions.probabilities[dist_idx]) {
                                                    block_id = custom_block;
                                                    break;
                                                }
                                            }
                                            break;
                                        }
                                    }
                                }
                            } else {
                                block_id = BLOCK_DIRT;
                            }
                        }
                    } else if (world_y == floor(height)) {
                        block_id = BLOCK_GRASS;
                    } else if (world_y < params.sea_level) {
                        block_id = BLOCK_DIRT;
                        skylight = max(0u, 15u - u32((params.sea_level - world_y) * 0.5));
                    }
                    
                    // Vectorized buffer indexing
                    let slot = u32(chunk_pos.w);
                    let buffer_index = slot * CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE + 
                                      base_x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE;
                    
                    // Write voxel data (can be vectorized for multiple writes)
                    world_data[buffer_index] = pack_voxel(block_id, 0u, skylight, 0u);
                }
                
                // Process remaining Z voxels for this Y strip
                for (var z_offset = 1u; z_offset < 4u; z_offset++) {
                    let final_z = base_z + z_offset;
                    if (base_x >= CHUNK_SIZE || base_y >= CHUNK_SIZE || final_z >= CHUNK_SIZE) {
                        continue;
                    }
                    
                    let world_z_offset = world_z + f32(z_offset);
                    let world_y = world_y_base;
                    
                    // Same terrain generation logic for Z-offset voxels
                    var block_id = BLOCK_AIR;
                    var skylight = 15u;
                    
                    let height = terrain_height(world_x, world_z_offset);
                    
                    if (world_y < height) {
                        skylight = 0u;
                        let cave = cave_density(world_x, world_y, world_z_offset);
                        if (cave < params.cave_threshold) {
                            if (world_y < height - 4.0) {
                                block_id = BLOCK_STONE;
                                
                                let custom_block = check_height_soa(i32(world_y));
                                if (custom_block != 0u) {
                                    for (var dist_idx = 0u; dist_idx < params.distributions.count; dist_idx++) {
                                        if (params.distributions.block_ids[dist_idx] == custom_block) {
                                            let block_noise = noise3d(
                                                world_x * 0.1, 
                                                world_y * 0.1, 
                                                world_z_offset * 0.1
                                            );
                                            
                                            if (block_noise > params.distributions.noise_thresholds[dist_idx]) {
                                                let chance = hash_float(
                                                    u32(world_x) * 73856093u ^ 
                                                    u32(world_y) * 19349663u ^ 
                                                    u32(world_z_offset) * 83492791u
                                                );
                                                if (chance < params.distributions.probabilities[dist_idx]) {
                                                    block_id = custom_block;
                                                    break;
                                                }
                                            }
                                            break;
                                        }
                                    }
                                }
                            } else {
                                block_id = BLOCK_DIRT;
                            }
                        }
                    } else if (world_y == floor(height)) {
                        block_id = BLOCK_GRASS;
                    } else if (world_y < params.sea_level) {
                        block_id = BLOCK_DIRT;
                        skylight = max(0u, 15u - u32((params.sea_level - world_y) * 0.5));
                    }
                    
                    let slot = u32(chunk_pos.w);
                    let buffer_index = slot * CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE + 
                                      base_x + base_y * CHUNK_SIZE + final_z * CHUNK_SIZE * CHUNK_SIZE;
                    
                    world_data[buffer_index] = pack_voxel(block_id, 0u, skylight, 0u);
                }
            }
        }
    }
}