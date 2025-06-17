// Jump flooding algorithm for SDF distance propagation

struct SdfValue {
    distance: f32,
    material: u32,
}

@group(0) @binding(0) var<storage, read> voxels: array<u32>; // Not used in this pass
@group(0) @binding(1) var<storage, read_write> sdf: array<SdfValue>;

var<push_constant> step_size: u32;

// Get SDF grid dimensions (hardcoded for now, could be uniforms)
fn get_grid_size() -> vec3<u32> {
    return vec3<u32>(128u, 128u, 128u); // Example size
}

// Convert 3D position to 1D index
fn pos_to_idx(pos: vec3<u32>, size: vec3<u32>) -> u32 {
    return pos.x + pos.y * size.x + pos.z * size.x * size.y;
}

// Check if position is in bounds
fn in_bounds(pos: vec3<i32>, size: vec3<u32>) -> bool {
    return all(pos >= vec3<i32>(0)) && all(pos < vec3<i32>(size));
}

@compute @workgroup_size(8, 8, 4)
fn jump_flood(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let grid_size = get_grid_size();
    
    if (any(global_id >= grid_size)) {
        return;
    }
    
    let center_idx = pos_to_idx(global_id, grid_size);
    var current_sdf = sdf[center_idx];
    
    // Jump flooding offsets
    let offsets = array<vec3<i32>, 8>(
        vec3<i32>(-1, -1, -1),
        vec3<i32>( 1, -1, -1),
        vec3<i32>(-1,  1, -1),
        vec3<i32>( 1,  1, -1),
        vec3<i32>(-1, -1,  1),
        vec3<i32>( 1, -1,  1),
        vec3<i32>(-1,  1,  1),
        vec3<i32>( 1,  1,  1)
    );
    
    // Check neighbors at current step size
    for (var i = 0u; i < 8u; i++) {
        let offset = offsets[i] * i32(step_size);
        let neighbor_pos = vec3<i32>(global_id) + offset;
        
        if (in_bounds(neighbor_pos, grid_size)) {
            let neighbor_idx = pos_to_idx(vec3<u32>(neighbor_pos), grid_size);
            let neighbor_sdf = sdf[neighbor_idx];
            
            // Calculate actual distance to neighbor's seed point
            if (neighbor_sdf.material > 0u) {
                let distance_to_neighbor = length(vec3<f32>(offset));
                let total_distance = neighbor_sdf.distance + distance_to_neighbor;
                
                // Update if closer
                if (abs(total_distance) < abs(current_sdf.distance)) {
                    current_sdf.distance = total_distance;
                    current_sdf.material = neighbor_sdf.material;
                }
            }
        }
    }
    
    // Write back result
    sdf[center_idx] = current_sdf;
}