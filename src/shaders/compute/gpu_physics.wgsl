// GPU Physics Simulation Shader
// 
// Replaces CPU physics simulation with GPU compute shader
// Part of Option 1: 95% GPU / 5% CPU performance split

// Physics body data structure (matches PhysicsBodyData)
struct PhysicsBody {
    position: vec3<f32>,
    velocity: vec3<f32>,
    aabb_min: vec3<f32>,
    aabb_max: vec3<f32>,
    mass: f32,
    friction: f32,
    restitution: f32,
    flags: u32,
}

// Physics simulation parameters
struct PhysicsParams {
    delta_time: f32,
    gravity: f32,
    entity_count: u32,
    _padding: u32,
}

// VoxelData is auto-generated from gpu/types/world.rs

// Entity physics data buffer
@group(0) @binding(0) var<storage, read_write> entities: array<PhysicsBody>;

// World voxel data buffer  
@group(0) @binding(1) var<storage, read> world_voxels: array<VoxelData>;

// Physics parameters
@group(0) @binding(2) var<uniform> params: PhysicsParams;

// Physics simulation constants
// Physics constants scaled for 1dcm³ voxels (10cm = 0.1m)
// These values are 10x larger than meter-based constants
const GRAVITY: f32 = -98.1;        // -9.81 m/s² × 10 voxels/m
const TERMINAL_VELOCITY: f32 = -500.0;  // -50 m/s × 10 voxels/m
// CHUNK_SIZE is auto-generated from constants.rs
const AIR_BLOCK_ID: u32 = 0u;

// Physics flags
const FLAG_ACTIVE: u32 = 1u;
const FLAG_GROUNDED: u32 = 2u;
const FLAG_IN_WATER: u32 = 4u;
const FLAG_ON_LADDER: u32 = 8u;

// Main physics update compute shader
@compute @workgroup_size(64, 1, 1)
fn physics_update(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let entity_index = global_id.x;
    
    // Check bounds
    if (entity_index >= params.entity_count) {
        return;
    }
    
    var entity = entities[entity_index];
    
    // Skip inactive entities
    if ((entity.flags & FLAG_ACTIVE) == 0u) {
        return;
    }
    
    // Apply gravity
    entity.velocity.y += GRAVITY * params.delta_time;
    
    // Clamp to terminal velocity
    entity.velocity.y = max(entity.velocity.y, TERMINAL_VELOCITY);
    
    // Apply friction and damping
    let friction_factor = pow(entity.friction, params.delta_time);
    entity.velocity.x *= friction_factor;
    entity.velocity.z *= friction_factor;
    
    // Calculate new position
    let old_position = entity.position;
    let new_position = old_position + entity.velocity * params.delta_time;
    
    // Collision detection and response
    let collision_result = check_collision(old_position, new_position, entity.aabb_min, entity.aabb_max);
    
    entity.position = collision_result.position;
    entity.velocity = collision_result.velocity;
    entity.flags = collision_result.flags;
    
    // Write back to buffer
    entities[entity_index] = entity;
}

// Collision result structure
struct CollisionResult {
    position: vec3<f32>,
    velocity: vec3<f32>,
    flags: u32,
}

// Check collision with world voxels
fn check_collision(old_pos: vec3<f32>, new_pos: vec3<f32>, aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> CollisionResult {
    var result: CollisionResult;
    result.position = new_pos;
    result.velocity = (new_pos - old_pos) / params.delta_time;
    result.flags = FLAG_ACTIVE;
    
    // Check collision with voxel world
    let entity_min = new_pos + aabb_min;
    let entity_max = new_pos + aabb_max;
    
    // Sample voxels in AABB region
    let min_voxel = vec3<i32>(floor(entity_min));
    let max_voxel = vec3<i32>(ceil(entity_max));
    
    var collision_detected = false;
    var ground_contact = false;
    
    // Check each voxel in the entity's AABB
    for (var x = min_voxel.x; x <= max_voxel.x; x++) {
        for (var y = min_voxel.y; y <= max_voxel.y; y++) {
            for (var z = min_voxel.z; z <= max_voxel.z; z++) {
                let voxel_pos = vec3<i32>(x, y, z);
                let block_id = get_block_at(voxel_pos);
                
                // Check if this voxel is solid
                if (block_id != AIR_BLOCK_ID) {
                    collision_detected = true;
                    
                    // Simple collision response - separate on each axis
                    let voxel_min = vec3<f32>(voxel_pos);
                    let voxel_max = voxel_min + vec3<f32>(1.0, 1.0, 1.0);
                    
                    // Check overlap on each axis
                    let overlap_x = min(entity_max.x, voxel_max.x) - max(entity_min.x, voxel_min.x);
                    let overlap_y = min(entity_max.y, voxel_max.y) - max(entity_min.y, voxel_min.y);
                    let overlap_z = min(entity_max.z, voxel_max.z) - max(entity_min.z, voxel_min.z);
                    
                    // Resolve collision on smallest overlap axis
                    if (overlap_x > 0.0 && overlap_y > 0.0 && overlap_z > 0.0) {
                        if (overlap_y <= overlap_x && overlap_y <= overlap_z) {
                            // Y-axis collision (ground/ceiling)
                            if (result.velocity.y < 0.0) {
                                // Hitting ground
                                result.position.y = voxel_max.y - aabb_min.y;
                                result.velocity.y = 0.0;
                                ground_contact = true;
                            } else {
                                // Hitting ceiling
                                result.position.y = voxel_min.y - aabb_max.y;
                                result.velocity.y = 0.0;
                            }
                        } else if (overlap_x <= overlap_z) {
                            // X-axis collision
                            if (result.position.x > f32(x)) {
                                result.position.x = voxel_max.x - aabb_min.x;
                            } else {
                                result.position.x = voxel_min.x - aabb_max.x;
                            }
                            result.velocity.x = 0.0;
                        } else {
                            // Z-axis collision
                            if (result.position.z > f32(z)) {
                                result.position.z = voxel_max.z - aabb_min.z;
                            } else {
                                result.position.z = voxel_min.z - aabb_max.z;
                            }
                            result.velocity.z = 0.0;
                        }
                    }
                }
            }
        }
    }
    
    // Set grounded flag
    if (ground_contact) {
        result.flags |= FLAG_GROUNDED;
    }
    
    return result;
}

// Get block ID at world position
fn get_block_at(world_pos: vec3<i32>) -> u32 {
    // Convert world position to chunk coordinates
    let chunk_pos = vec3<i32>(
        world_pos.x / i32(CHUNK_SIZE),
        world_pos.y / i32(CHUNK_SIZE),
        world_pos.z / i32(CHUNK_SIZE)
    );
    
    // Convert to local position within chunk
    let local_pos = vec3<u32>(
        u32(world_pos.x % i32(CHUNK_SIZE)),
        u32(world_pos.y % i32(CHUNK_SIZE)),
        u32(world_pos.z % i32(CHUNK_SIZE))
    );
    
    // Calculate voxel index in world buffer
    // For now, simplified - in production would use proper chunk indexing
    let world_size = 64u; // Assuming reasonable world size
    if (chunk_pos.x < 0 || chunk_pos.y < 0 || chunk_pos.z < 0 ||
        chunk_pos.x >= i32(world_size) || chunk_pos.y >= i32(world_size) || chunk_pos.z >= i32(world_size)) {
        return AIR_BLOCK_ID; // Out of bounds
    }
    
    let chunk_index = u32(chunk_pos.x) + u32(chunk_pos.y) * world_size + u32(chunk_pos.z) * world_size * world_size;
    let voxel_index = local_pos.x + local_pos.y * CHUNK_SIZE + local_pos.z * CHUNK_SIZE * CHUNK_SIZE;
    let global_voxel_index = chunk_index * (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) + voxel_index;
    
    // Bounds check
    if (global_voxel_index >= arrayLength(&world_voxels)) {
        return AIR_BLOCK_ID;
    }
    
    // Extract block ID from voxel data (lower 16 bits)
    let voxel_data = world_voxels[global_voxel_index].data;
    return voxel_data & 0xFFFFu;
}