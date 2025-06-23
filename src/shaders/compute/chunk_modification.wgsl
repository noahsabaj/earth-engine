// GPU Chunk Modification Shader
// Handles thread-safe voxel modifications using atomic operations

struct ModificationCommand {
    position: vec3<i32>,
    block_id: u32,
    mod_type: u32,      // 0=set, 1=break, 2=explode
    radius: f32,
    _padding: vec2<u32>,
}

// World constants - auto-generated from constants.rs
// CHUNK_SIZE, MAX_WORLD_SIZE, BLOCK_AIR, BLOCK_STONE are auto-generated
const WORLD_HEIGHT: u32 = 256u;

// Voxel packing constants
const BLOCK_ID_MASK: u32 = 0xFFFFu;
const LIGHT_MASK: u32 = 0xFu;
const LIGHT_SHIFT: u32 = 16u;
const SKYLIGHT_SHIFT: u32 = 20u;
const METADATA_SHIFT: u32 = 24u;

// Bindings
@group(0) @binding(0) var<storage, read_write> world_voxels: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read> commands: array<ModificationCommand>;
@group(0) @binding(2) var<uniform> command_count: u32;

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

fn pack_voxel(block_id: u32, light: u32, skylight: u32, metadata: u32) -> u32 {
    return block_id | (light << LIGHT_SHIFT) | (skylight << SKYLIGHT_SHIFT) | (metadata << METADATA_SHIFT);
}

fn unpack_block_id(voxel: u32) -> u32 {
    return voxel & BLOCK_ID_MASK;
}

fn unpack_light(voxel: u32) -> u32 {
    return (voxel >> LIGHT_SHIFT) & LIGHT_MASK;
}

fn unpack_skylight(voxel: u32) -> u32 {
    return (voxel >> SKYLIGHT_SHIFT) & LIGHT_MASK;
}

// Single block modification kernel
@compute @workgroup_size(64, 1, 1)
fn modify_blocks(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cmd_idx = global_id.x;
    if (cmd_idx >= command_count) {
        return;
    }
    
    let cmd = commands[cmd_idx];
    let voxel_idx = world_to_index(cmd.position);
    
    if (voxel_idx == 0xFFFFFFFFu) {
        return; // Out of bounds
    }
    
    if (cmd.mod_type == 0u) {
        // Set block - preserve lighting
        let old_voxel = atomicLoad(&world_voxels[voxel_idx]);
        let light = unpack_light(old_voxel);
        let skylight = unpack_skylight(old_voxel);
        let new_voxel = pack_voxel(cmd.block_id, light, skylight, 0u);
        atomicStore(&world_voxels[voxel_idx], new_voxel);
    } else if (cmd.mod_type == 1u) {
        // Break block - set to air
        let new_voxel = pack_voxel(BLOCK_AIR, 0u, 15u, 0u); // Full skylight for air
        atomicStore(&world_voxels[voxel_idx], new_voxel);
    }
}

// Explosion effect kernel
@compute @workgroup_size(8, 8, 4)
fn explode_blocks(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    // Each explosion is processed by multiple workgroups
    let explosion_idx = workgroup_id.x / 8u;
    if (explosion_idx >= command_count) {
        return;
    }
    
    let cmd = commands[explosion_idx];
    let center = vec3<f32>(cmd.position);
    let radius = cmd.radius;
    let radius_squared = radius * radius;
    
    // Calculate position to check
    let local_offset = vec3<i32>(global_id % 64u) - vec3<i32>(32);
    let check_pos = cmd.position + local_offset;
    
    // Calculate distance from explosion center
    let offset = vec3<f32>(check_pos) - center;
    let dist_squared = dot(offset, offset);
    
    if (dist_squared <= radius_squared) {
        let voxel_idx = world_to_index(check_pos);
        if (voxel_idx != 0xFFFFFFFFu) {
            // Calculate damage based on distance
            let damage_factor = 1.0 - sqrt(dist_squared) / radius;
            
            // Random chance to destroy block based on damage
            let hash = u32(check_pos.x * 73856093) ^ u32(check_pos.y * 19349663) ^ u32(check_pos.z * 83492791);
            let random = f32(hash & 0xFFFFu) / 65535.0;
            
            if (random < damage_factor * damage_factor) {
                // Destroy block
                let old_voxel = atomicLoad(&world_voxels[voxel_idx]);
                let old_block = unpack_block_id(old_voxel);
                
                // Don't destroy bedrock
                if (old_block != 32u) {
                    let new_voxel = pack_voxel(BLOCK_AIR, 0u, 0u, 0u);
                    atomicStore(&world_voxels[voxel_idx], new_voxel);
                }
            }
        }
    }
}

// Bulk fill operation (for editor tools)
@compute @workgroup_size(8, 8, 4)
fn fill_region(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    // TODO: Implement region filling for editor tools
    // This would read region bounds and block type from a uniform buffer
}

// Lighting update kernel (marks chunks for relighting)
@compute @workgroup_size(32, 1, 1)
fn mark_for_relighting(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // TODO: Mark affected chunks for lighting updates
    // This would update chunk metadata flags
}