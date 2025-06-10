// SDF gradient calculation using finite differences

struct SdfValue {
    distance: f32,
    material: u32,
    gradient_mag: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> voxels: array<u32>; // Not used
@group(0) @binding(1) var<storage, read_write> sdf: array<SdfValue>;

// Get SDF grid dimensions
fn get_grid_size() -> vec3<u32> {
    return vec3<u32>(128u, 128u, 128u); // Example size
}

// Convert 3D position to 1D index
fn pos_to_idx(pos: vec3<u32>, size: vec3<u32>) -> u32 {
    return pos.x + pos.y * size.x + pos.z * size.x * size.y;
}

// Sample SDF distance at position
fn sample_distance(pos: vec3<i32>, size: vec3<u32>) -> f32 {
    // Clamp to bounds
    let clamped_pos = vec3<u32>(clamp(pos, vec3<i32>(0), vec3<i32>(size - vec3<u32>(1u))));
    let idx = pos_to_idx(clamped_pos, size);
    return sdf[idx].distance;
}

// Pack gradient magnitude to u16
fn pack_gradient_mag(mag: f32) -> u32 {
    // Normalize and pack to 16 bits
    let normalized = clamp(mag / 10.0, 0.0, 1.0); // Assume max gradient of 10
    return u32(normalized * 65535.0);
}

@compute @workgroup_size(8, 8, 8)
fn calculate_gradient(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let grid_size = get_grid_size();
    
    if (any(global_id >= grid_size)) {
        return;
    }
    
    let pos = vec3<i32>(global_id);
    
    // Calculate gradient using central differences
    let dx = sample_distance(pos + vec3<i32>(1, 0, 0), grid_size) - 
             sample_distance(pos - vec3<i32>(1, 0, 0), grid_size);
    let dy = sample_distance(pos + vec3<i32>(0, 1, 0), grid_size) - 
             sample_distance(pos - vec3<i32>(0, 1, 0), grid_size);
    let dz = sample_distance(pos + vec3<i32>(0, 0, 1), grid_size) - 
             sample_distance(pos - vec3<i32>(0, 0, 1), grid_size);
    
    let gradient = vec3<f32>(dx, dy, dz) * 0.5; // Central difference scale
    let gradient_mag = length(gradient);
    
    // Store gradient magnitude
    let idx = pos_to_idx(global_id, grid_size);
    sdf[idx].gradient_mag = pack_gradient_mag(gradient_mag);
}