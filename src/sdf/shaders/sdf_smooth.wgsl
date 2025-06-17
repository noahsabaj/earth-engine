// SDF smoothing using weighted averaging

struct SdfValue {
    distance: f32,
    material: u32,
    gradient_mag: u32,
    _padding: u32,
}

struct SdfConstants {
    resolution_factor: f32,
    max_distance: f32,
    surface_threshold: f32,
    smoothing_factor: f32,
    voxel_size: f32,
}

@group(0) @binding(0) var<storage, read> voxels: array<u32>; // Not used
@group(0) @binding(1) var<storage, read_write> sdf: array<SdfValue>;
@group(0) @binding(2) var<uniform> constants: SdfConstants;

// Get SDF grid dimensions
fn get_grid_size() -> vec3<u32> {
    return vec3<u32>(128u, 128u, 128u); // Example size
}

// Convert 3D position to 1D index
fn pos_to_idx(pos: vec3<u32>, size: vec3<u32>) -> u32 {
    return pos.x + pos.y * size.x + pos.z * size.x * size.y;
}

// Sample SDF at position with bounds checking
fn sample_sdf(pos: vec3<i32>, size: vec3<u32>) -> SdfValue {
    if (any(pos < vec3<i32>(0)) || any(pos >= vec3<i32>(size))) {
        var empty: SdfValue;
        empty.distance = constants.max_distance;
        empty.material = 0u;
        empty.gradient_mag = 0u;
        return empty;
    }
    
    let idx = pos_to_idx(vec3<u32>(pos), size);
    return sdf[idx];
}

@compute @workgroup_size(8, 8, 4)
fn smooth_sdf(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let grid_size = get_grid_size();
    
    if (any(global_id >= grid_size)) {
        return;
    }
    
    let center_pos = vec3<i32>(global_id);
    let center_sdf = sample_sdf(center_pos, grid_size);
    
    // Skip if far from surface
    if (abs(center_sdf.distance) > 3.0) {
        return;
    }
    
    // 3x3x3 Gaussian kernel weights
    let weights = array<f32, 27>(
        // Layer -1
        0.015625, 0.03125, 0.015625,
        0.03125,  0.0625,  0.03125,
        0.015625, 0.03125, 0.015625,
        // Layer 0
        0.03125, 0.0625, 0.03125,
        0.0625,  0.125,  0.0625,
        0.03125, 0.0625, 0.03125,
        // Layer +1
        0.015625, 0.03125, 0.015625,
        0.03125,  0.0625,  0.03125,
        0.015625, 0.03125, 0.015625
    );
    
    var weighted_sum = 0.0;
    var weight_sum = 0.0;
    var material_votes = array<u32, 16>(); // Support up to 16 materials
    
    // Apply weighted averaging
    var kernel_idx = 0u;
    for (var dz = -1; dz <= 1; dz++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dx = -1; dx <= 1; dx++) {
                let neighbor_pos = center_pos + vec3<i32>(dx, dy, dz);
                let neighbor_sdf = sample_sdf(neighbor_pos, grid_size);
                
                let weight = weights[kernel_idx];
                weighted_sum += neighbor_sdf.distance * weight;
                weight_sum += weight;
                
                // Vote for material
                if (neighbor_sdf.material < 16u) {
                    material_votes[neighbor_sdf.material] += 1u;
                }
                
                kernel_idx += 1u;
            }
        }
    }
    
    // Apply smoothing
    let smoothed_distance = mix(
        center_sdf.distance,
        weighted_sum / weight_sum,
        constants.smoothing_factor
    );
    
    // Find most voted material
    var max_votes = 0u;
    var best_material = center_sdf.material;
    for (var i = 0u; i < 16u; i++) {
        if (material_votes[i] > max_votes) {
            max_votes = material_votes[i];
            best_material = i;
        }
    }
    
    // Write back smoothed result
    let idx = pos_to_idx(global_id, grid_size);
    sdf[idx].distance = smoothed_distance;
    sdf[idx].material = best_material;
}