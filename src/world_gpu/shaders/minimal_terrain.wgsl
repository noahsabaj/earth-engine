// Minimal terrain generation shader for testing pipeline
// Just fills chunks with simple patterns to verify GPU pipeline works

struct TerrainParams {
    seed: u32,
    sea_level: f32,
    terrain_scale: f32,
    mountain_threshold: f32,
    cave_threshold: f32,
    // Note: This minimal test shader doesn't use distributions
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

// World constants
const CHUNK_SIZE: u32 = 32u;

// Bindings
@group(0) @binding(0) var<storage, read_write> world_voxels: array<u32>;
@group(0) @binding(1) var<storage, read_write> chunk_metadata: array<ChunkMetadata>;
@group(0) @binding(2) var<uniform> params: TerrainParams;
@group(0) @binding(3) var<storage, read> chunk_positions: array<vec4<i32>>;

// Helper functions
fn pack_voxel(block_id: u32, light: u32, skylight: u32, metadata: u32) -> u32 {
    return block_id | (light << 16u) | (skylight << 20u) | (metadata << 24u);
}

// Simple terrain generation for testing
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
                
                // Simple terrain pattern for testing
                var block_id = BLOCK_AIR;
                var skylight = 15u;
                
                // Generate terrain adaptive to chunk Y position to ensure camera visibility
                // Camera spawns at Y=80, so we need terrain in chunks that intersect this view
                // Each chunk is 32 blocks high, so:
                // - Chunk Y=0: world Y 0-31, generate terrain at Y=24-31 (top 8 blocks)
                // - Chunk Y=1: world Y 32-63, generate terrain at Y=56-63 (top 8 blocks) 
                // - Chunk Y=2: world Y 64-95, generate terrain at Y=88-95 (top 8 blocks) <- Camera will see this
                
                let chunk_y_coord = f32(chunk_pos.y);
                let chunk_base_y = chunk_y_coord * f32(CHUNK_SIZE);
                
                // Generate terrain in the top 8 blocks of each chunk
                let surface_top = chunk_base_y + f32(CHUNK_SIZE) - 1.0;  // Top of chunk
                let surface_bottom = chunk_base_y + f32(CHUNK_SIZE) - 8.0; // 8 blocks thick
                
                if (world_y < surface_bottom) {
                    skylight = 0u;
                    if (world_y < surface_bottom - 8.0) {
                        block_id = BLOCK_STONE;
                    } else {
                        block_id = BLOCK_DIRT;
                    }
                } else if (world_y >= surface_bottom && world_y <= surface_top) {
                    skylight = 0u;
                    block_id = BLOCK_GRASS; // Thick grass layer for visibility
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