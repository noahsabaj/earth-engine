// Marching cubes cell classification

struct SdfValue {
    distance: f32,
    material: u32,
    gradient_mag: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> sdf: array<SdfValue>;
@group(0) @binding(1) var<storage, read_write> cell_types: array<u32>;
@group(0) @binding(2) var<storage, read> edge_table: array<u32>;

var<push_constant> threshold: f32;

// Get SDF grid dimensions
fn get_grid_size() -> vec3<u32> {
    return vec3<u32>(128u, 128u, 128u); // Example size
}

// Convert 3D position to 1D index
fn pos_to_idx(pos: vec3<u32>, size: vec3<u32>) -> u32 {
    return pos.x + pos.y * size.x + pos.z * size.x * size.y;
}

// Sample SDF value at corner
fn sample_corner(base_pos: vec3<u32>, corner: vec3<u32>, grid_size: vec3<u32>) -> f32 {
    let pos = base_pos + corner;
    if (any(pos >= grid_size)) {
        return 1.0; // Outside is positive
    }
    let idx = pos_to_idx(pos, grid_size);
    return sdf[idx].distance;
}

@compute @workgroup_size(8, 8, 4)
fn classify_cells(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let grid_size = get_grid_size();
    let cell_size = grid_size - vec3<u32>(1u);
    
    if (any(global_id >= cell_size)) {
        return;
    }
    
    // Sample 8 corners of the cell
    let corners = array<vec3<u32>, 8>(
        vec3<u32>(0u, 0u, 0u),
        vec3<u32>(1u, 0u, 0u),
        vec3<u32>(1u, 1u, 0u),
        vec3<u32>(0u, 1u, 0u),
        vec3<u32>(0u, 0u, 1u),
        vec3<u32>(1u, 0u, 1u),
        vec3<u32>(1u, 1u, 1u),
        vec3<u32>(0u, 1u, 1u)
    );
    
    // Build cube index
    var cube_index = 0u;
    for (var i = 0u; i < 8u; i++) {
        let corner_value = sample_corner(global_id, corners[i], grid_size);
        if (corner_value < threshold) {
            cube_index |= (1u << i);
        }
    }
    
    // Store cell type
    let cell_idx = pos_to_idx(global_id, cell_size);
    cell_types[cell_idx] = cube_index;
}