// GPU Ambient Occlusion Calculation Shader
// Calculates ambient occlusion values for voxels based on neighboring blocks

// World constants - auto-generated from constants.rs
// CHUNK_SIZE, MAX_WORLD_SIZE are auto-generated
const WORLD_HEIGHT: u32 = 256u;

// Voxel packing constants
const BLOCK_ID_MASK: u32 = 0xFFFFu;
const LIGHT_MASK: u32 = 0xFu;
const LIGHT_SHIFT: u32 = 16u;
const SKYLIGHT_SHIFT: u32 = 20u;
const METADATA_SHIFT: u32 = 24u;
const METADATA_MASK: u32 = 0xFFu;

// AO is stored in lower 4 bits of metadata
const AO_MASK: u32 = 0xFu;
const AO_SHIFT: u32 = 0u;

// Block types
const BLOCK_AIR: u32 = 0u;
const BLOCK_WATER: u32 = 5u;

// AO calculation constants
const AO_STRENGTH: f32 = 0.2; // How much each occluder darkens
const AO_MAX: u32 = 15u;      // Maximum AO value (4 bits)

// Bindings
@group(0) @binding(0) var<storage, read_write> world_voxels: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read> chunk_positions: array<vec4<i32>>;

// Helper functions
fn world_to_index(pos: vec3<i32>) -> u32 {
    // Bounds check
    if (pos.x < 0 || pos.y < 0 || pos.z < 0 ||
        pos.x >= i32(WORLD_SIZE * CHUNK_SIZE) ||
        pos.y >= i32(WORLD_HEIGHT) ||
        pos.z >= i32(WORLD_SIZE * CHUNK_SIZE)) {
        return 0xFFFFFFFFu; // Invalid index
    }
    
    let chunk_x = u32(pos.x) >> 5u; // div by 32
    let chunk_y = u32(pos.y) >> 5u;
    let chunk_z = u32(pos.z) >> 5u;
    
    let local_x = u32(pos.x) & 31u; // mod 32
    let local_y = u32(pos.y) & 31u;
    let local_z = u32(pos.z) & 31u;
    
    let chunk_index = chunk_x + chunk_y * WORLD_SIZE + chunk_z * WORLD_SIZE * WORLD_SIZE;
    let chunk_offset = chunk_index * CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    let local_index = local_x + local_y * CHUNK_SIZE + local_z * CHUNK_SIZE * CHUNK_SIZE;
    
    return chunk_offset + local_index;
}

fn unpack_block_id(voxel: u32) -> u32 {
    return voxel & BLOCK_ID_MASK;
}

fn unpack_metadata(voxel: u32) -> u32 {
    return (voxel >> METADATA_SHIFT) & METADATA_MASK;
}

fn pack_ao_in_metadata(metadata: u32, ao: u32) -> u32 {
    // Clear AO bits and set new value
    return (metadata & ~AO_MASK) | (ao & AO_MASK);
}

fn is_solid_block(block_id: u32) -> bool {
    return block_id != BLOCK_AIR && block_id != BLOCK_WATER;
}

// Check if a neighboring position blocks light
fn is_occluder(pos: vec3<i32>) -> f32 {
    let idx = world_to_index(pos);
    if (idx == 0xFFFFFFFFu) {
        return 1.0; // Out of bounds blocks light
    }
    
    let voxel = atomicLoad(&world_voxels[idx]);
    let block_id = unpack_block_id(voxel);
    
    return select(0.0, 1.0, is_solid_block(block_id));
}

// Calculate AO for a vertex based on 3 neighboring blocks
fn vertex_ao(side1: f32, side2: f32, corner: f32) -> f32 {
    if (side1 + side2 >= 2.0) {
        return 0.0; // Fully occluded
    }
    return 3.0 - (side1 + side2 + corner);
}

// Main AO calculation kernel
@compute @workgroup_size(8, 8, 4)
fn calculate_ao(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    // Get chunk to process
    let chunk_idx = workgroup_id.x;
    if (chunk_idx >= arrayLength(&chunk_positions)) {
        return;
    }
    
    let chunk_pos = chunk_positions[chunk_idx];
    let chunk_world_x = chunk_pos.x * i32(CHUNK_SIZE);
    let chunk_world_y = chunk_pos.y * i32(CHUNK_SIZE);
    let chunk_world_z = chunk_pos.z * i32(CHUNK_SIZE);
    
    // Calculate position for this thread
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
                
                let world_pos = vec3<i32>(
                    chunk_world_x + i32(x),
                    chunk_world_y + i32(y),
                    chunk_world_z + i32(z)
                );
                
                let voxel_idx = world_to_index(world_pos);
                if (voxel_idx == 0xFFFFFFFFu) {
                    continue;
                }
                
                let voxel = atomicLoad(&world_voxels[voxel_idx]);
                let block_id = unpack_block_id(voxel);
                
                // Skip air blocks and water
                if (!is_solid_block(block_id)) {
                    continue;
                }
                
                // Calculate AO by checking neighbors
                // We use a simplified approach: count solid neighbors in 6 directions
                var occlusion = 0.0;
                
                // Check 6 face neighbors
                occlusion += is_occluder(world_pos + vec3<i32>(1, 0, 0));
                occlusion += is_occluder(world_pos + vec3<i32>(-1, 0, 0));
                occlusion += is_occluder(world_pos + vec3<i32>(0, 1, 0));
                occlusion += is_occluder(world_pos + vec3<i32>(0, -1, 0));
                occlusion += is_occluder(world_pos + vec3<i32>(0, 0, 1));
                occlusion += is_occluder(world_pos + vec3<i32>(0, 0, -1));
                
                // Check 12 edge neighbors (reduced weight)
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(1, 1, 0));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(1, -1, 0));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(-1, 1, 0));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(-1, -1, 0));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(1, 0, 1));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(1, 0, -1));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(-1, 0, 1));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(-1, 0, -1));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(0, 1, 1));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(0, 1, -1));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(0, -1, 1));
                occlusion += 0.5 * is_occluder(world_pos + vec3<i32>(0, -1, -1));
                
                // Normalize and convert to 4-bit value
                let ao_factor = clamp(occlusion / 12.0, 0.0, 1.0);
                let ao_value = u32(ao_factor * f32(AO_MAX));
                
                // Update metadata with AO value
                let old_metadata = unpack_metadata(voxel);
                let new_metadata = pack_ao_in_metadata(old_metadata, ao_value);
                
                // Atomically update only if metadata changed
                if (new_metadata != old_metadata) {
                    // Reconstruct voxel with new metadata
                    let new_voxel = (voxel & ~(METADATA_MASK << METADATA_SHIFT)) | 
                                   (new_metadata << METADATA_SHIFT);
                    atomicStore(&world_voxels[voxel_idx], new_voxel);
                }
            }
        }
    }
}

// Smooth AO kernel - averages AO values with neighbors for smoother gradients
@compute @workgroup_size(8, 8, 4)
fn smooth_ao(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    // Get chunk to process
    let chunk_idx = workgroup_id.x;
    if (chunk_idx >= arrayLength(&chunk_positions)) {
        return;
    }
    
    let chunk_pos = chunk_positions[chunk_idx];
    let chunk_world_x = chunk_pos.x * i32(CHUNK_SIZE);
    let chunk_world_y = chunk_pos.y * i32(CHUNK_SIZE);
    let chunk_world_z = chunk_pos.z * i32(CHUNK_SIZE);
    
    // Calculate position for this thread
    let x = local_id.x * 4u;
    let y = local_id.y * 4u;
    let z = local_id.z * 4u;
    
    if (x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE) {
        return;
    }
    
    let world_pos = vec3<i32>(
        chunk_world_x + i32(x),
        chunk_world_y + i32(y),
        chunk_world_z + i32(z)
    );
    
    let center_idx = world_to_index(world_pos);
    if (center_idx == 0xFFFFFFFFu) {
        return;
    }
    
    let center_voxel = atomicLoad(&world_voxels[center_idx]);
    let center_block = unpack_block_id(center_voxel);
    
    if (!is_solid_block(center_block)) {
        return;
    }
    
    // Average AO with neighbors
    var ao_sum = 0.0;
    var count = 0.0;
    
    // Sample 3x3x3 neighborhood
    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dz = -1; dz <= 1; dz++) {
                let neighbor_pos = world_pos + vec3<i32>(dx, dy, dz);
                let neighbor_idx = world_to_index(neighbor_pos);
                
                if (neighbor_idx != 0xFFFFFFFFu) {
                    let neighbor_voxel = atomicLoad(&world_voxels[neighbor_idx]);
                    let neighbor_block = unpack_block_id(neighbor_voxel);
                    
                    if (is_solid_block(neighbor_block)) {
                        let neighbor_metadata = unpack_metadata(neighbor_voxel);
                        let neighbor_ao = neighbor_metadata & AO_MASK;
                        ao_sum += f32(neighbor_ao);
                        count += 1.0;
                    }
                }
            }
        }
    }
    
    if (count > 0.0) {
        let smoothed_ao = u32(round(ao_sum / count));
        let old_metadata = unpack_metadata(center_voxel);
        let new_metadata = pack_ao_in_metadata(old_metadata, smoothed_ao);
        
        if (new_metadata != old_metadata) {
            let new_voxel = (center_voxel & ~(METADATA_MASK << METADATA_SHIFT)) | 
                           (new_metadata << METADATA_SHIFT);
            atomicStore(&world_voxels[center_idx], new_voxel);
        }
    }
}