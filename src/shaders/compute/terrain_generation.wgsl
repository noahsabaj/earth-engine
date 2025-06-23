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

// Extract weather type from packed value
fn get_weather_type() -> u32 {
    return params.weather_type_intensity & 0xFFu;
}

// Extract weather intensity from packed value  
fn get_weather_intensity() -> u32 {
    return (params.weather_type_intensity >> 8u) & 0xFFu;
}

// Check if current weather conditions would produce snow
fn should_place_snow(world_y: f32, temperature: i32) -> bool {
    let weather_type = get_weather_type();
    let weather_intensity = get_weather_intensity();
    
    // Snow only in snow or blizzard weather
    if (weather_type != WEATHER_SNOW && weather_type != WEATHER_BLIZZARD) {
        return false;
    }
    
    // Temperature check - must be below snow threshold
    if (temperature > SNOW_THRESHOLD) {
        return false;
    }
    
    // Altitude-based snow with weather influence
    let snow_height_threshold = f32(SNOW_HEIGHT_TYPICAL_LOW) - f32(weather_intensity); // Lower threshold with higher intensity
    return world_y >= snow_height_threshold;
}

// Check if water should be ice based on weather
fn should_freeze_water(temperature: i32) -> bool {
    return temperature <= FREEZING_POINT;
}

// Get weather-modified surface block
fn get_weather_surface_block(base_block: u32, world_y: f32, temperature: i32) -> u32 {
    let weather_type = get_weather_type();
    let weather_intensity = get_weather_intensity();
    
    // Check for snow placement
    if (should_place_snow(world_y, temperature)) {
        return BLOCK_SNOW;
    }
    
    // Modify blocks based on weather
    if (base_block == BLOCK_GRASS) {
        // Frozen grass in cold weather
        if (temperature <= FREEZING_POINT) {
            return BLOCK_FROZEN_GRASS;
        }
        // Mud in heavy rain
        if (weather_type == WEATHER_RAIN && weather_intensity >= INTENSITY_HEAVY) {
            return BLOCK_MUD;
        }
    }
    
    // Wet stone in rain
    if (base_block == BLOCK_STONE && weather_type == WEATHER_RAIN) {
        return BLOCK_WET_STONE;
    }
    
    return base_block;
}

// Wrapper for 3D noise (using 3D perlin from included file)
fn noise3d(x: f32, y: f32, z: f32) -> f32 {
    return perlin3d(x, y, z);
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
    let slot = chunk_meta.slot_index;
    let y_position = chunk_meta.y_position;
    
    // Decode chunk position from metadata
    let chunk_x = i32((flags >> 16u) & 0xFFFFu);
    let chunk_y = i32(y_position);
    let chunk_z = i32(flags & 0xFFFFu);
    
    // Sign extend from 16-bit to 32-bit
    let chunk_pos_x = select(chunk_x, chunk_x - 65536, chunk_x >= 32768);
    let chunk_pos_y = chunk_y;
    let chunk_pos_z = select(chunk_z, chunk_z - 65536, chunk_z >= 32768);
    
    let chunk_pos = vec4<i32>(chunk_pos_x, chunk_pos_y, chunk_pos_z, i32(slot));
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
                
                // Improved terrain generation with height variation and proper surface topology
                // Calculate terrain height with variation (matching CPU fallback algorithm)
                let height_variation = sin(world_x * 0.05) * 5.0 + cos(world_z * 0.05) * 5.0;
                let surface_height = f32(TERRAIN_THRESHOLD) + height_variation;
                
                if (world_y < surface_height - 3.0) {
                    // Deep underground: stone
                    skylight = 0u;
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
                } else if (world_y < surface_height) {
                    // Just below surface: stone with occasional air (caves)
                    skylight = 0u;
                    
                    // Simple cave generation: create air pockets using deterministic noise
                    let cave_noise_val = f32((u32(world_x) + u32(world_y) * 7u + u32(world_z) * 13u) % 100u) / 100.0;
                    if (cave_noise_val > 0.85 && world_y < surface_height - 5.0) {
                        // Cave air pocket
                        block_id = BLOCK_AIR;
                        skylight = 5u; // Some light in caves
                    } else {
                        // Solid stone
                        block_id = BLOCK_STONE;
                    }
                } else if (world_y == floor(surface_height)) {
                    // Surface layer: grass
                    block_id = get_weather_surface_block(BLOCK_GRASS, world_y, params.temperature);
                    skylight = 15u;
                } else if (world_y < params.sea_level) {
                    // Below sea level - water or ice based on temperature
                    if (should_freeze_water(params.temperature)) {
                        block_id = BLOCK_ICE;
                    } else {
                        block_id = BLOCK_WATER;
                    }
                    skylight = max(0u, 15u - u32((params.sea_level - world_y) * 0.5));
                } else if (world_y > surface_height && should_place_snow(world_y, params.temperature)) {
                    // Snow layer on top of terrain in cold weather
                    let snow_depth = u32((world_y - surface_height) / 2.0);
                    if (snow_depth < 3u) { // Max 3 blocks of snow accumulation
                        block_id = BLOCK_SNOW;
                        skylight = 15u;
                    }
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
    
    // Metadata updates are handled by CPU side after generation completes
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

// terrain_height function is already defined in perlin_noise.wgsl include

// cave_density function is already defined in perlin_noise.wgsl include

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
    let slot = chunk_meta.slot_index;
    let y_position = chunk_meta.y_position;
    
    // Decode chunk position from metadata
    let chunk_x = i32((flags >> 16u) & 0xFFFFu);
    let chunk_y = i32(y_position);
    let chunk_z = i32(flags & 0xFFFFu);
    
    // Sign extend from 16-bit to 32-bit
    let chunk_pos_x = select(chunk_x, chunk_x - 65536, chunk_x >= 32768);
    let chunk_pos_y = chunk_y;
    let chunk_pos_z = select(chunk_z, chunk_z - 65536, chunk_z >= 32768);
    
    let chunk_pos = vec4<i32>(chunk_pos_x, chunk_pos_y, chunk_pos_z, i32(slot));
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
                    
                    // Improved vectorized terrain generation with height variation and proper surface topology
                    var block_id = BLOCK_AIR;
                    var skylight = 15u;
                    
                    // Calculate terrain height with variation (matching CPU fallback algorithm)
                    let height_variation = sin(world_x * 0.05) * 5.0 + cos(world_z * 0.05) * 5.0;
                    let surface_height = f32(TERRAIN_THRESHOLD) + height_variation;
                    
                    if (world_y < surface_height - 3.0) {
                        // Deep underground: stone
                        skylight = 0u;
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
                    } else if (world_y < surface_height) {
                        // Just below surface: stone with occasional air (caves)
                        skylight = 0u;
                        
                        // Simple cave generation: create air pockets using deterministic noise
                        let cave_noise_val = f32((u32(world_x) + u32(world_y) * 7u + u32(world_z) * 13u) % 100u) / 100.0;
                        if (cave_noise_val > 0.85 && world_y < surface_height - 5.0) {
                            // Cave air pocket
                            block_id = BLOCK_AIR;
                            skylight = 5u; // Some light in caves
                        } else {
                            // Solid stone
                            block_id = BLOCK_STONE;
                        }
                    } else if (world_y == floor(surface_height)) {
                        // Surface layer: grass
                        block_id = get_weather_surface_block(BLOCK_GRASS, world_y, params.temperature);
                        skylight = 15u;
                    } else if (world_y < params.sea_level) {
                        if (should_freeze_water(params.temperature)) {
                            block_id = BLOCK_ICE;
                        } else {
                            block_id = BLOCK_WATER;
                        }
                        skylight = max(0u, 15u - u32((params.sea_level - world_y) * 0.5));
                    } else if (world_y > surface_height && should_place_snow(world_y, params.temperature)) {
                        let snow_depth = u32((world_y - surface_height) / 2.0);
                        if (snow_depth < 3u) {
                            block_id = BLOCK_SNOW;
                            skylight = 15u;
                        }
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
                    
                    // Improved terrain generation logic for Z-offset voxels (matching CPU fallback)
                    var block_id = BLOCK_AIR;
                    var skylight = 15u;
                    
                    // Calculate terrain height with variation (matching CPU fallback algorithm)
                    let height_variation = sin(world_x * 0.05) * 5.0 + cos(world_z_offset * 0.05) * 5.0;
                    let surface_height = 64.0 + height_variation;
                    
                    if (world_y < surface_height - 3.0) {
                        // Deep underground: stone
                        skylight = 0u;
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
                    } else if (world_y < surface_height) {
                        // Just below surface: stone with occasional air (caves)
                        skylight = 0u;
                        
                        // Simple cave generation: create air pockets using deterministic noise
                        let cave_noise_val = f32((u32(world_x) + u32(world_y) * 7u + u32(world_z_offset) * 13u) % 100u) / 100.0;
                        if (cave_noise_val > 0.85 && world_y < surface_height - 5.0) {
                            // Cave air pocket
                            block_id = BLOCK_AIR;
                            skylight = 5u; // Some light in caves
                        } else {
                            // Solid stone
                            block_id = BLOCK_STONE;
                        }
                    } else if (world_y == floor(surface_height)) {
                        // Surface layer: grass
                        block_id = get_weather_surface_block(BLOCK_GRASS, world_y, params.temperature);
                        skylight = 15u;
                    } else if (world_y < params.sea_level) {
                        if (should_freeze_water(params.temperature)) {
                            block_id = BLOCK_ICE;
                        } else {
                            block_id = BLOCK_WATER;
                        }
                        skylight = max(0u, 15u - u32((params.sea_level - world_y) * 0.5));
                    } else if (world_y > surface_height && should_place_snow(world_y, params.temperature)) {
                        let snow_depth = u32((world_y - surface_height) / 2.0);
                        if (snow_depth < 3u) {
                            block_id = BLOCK_SNOW;
                            skylight = 15u;
                        }
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