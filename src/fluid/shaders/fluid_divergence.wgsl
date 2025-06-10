// Fluid divergence shader - Calculate velocity divergence for pressure solve

struct FluidVoxel {
    packed_data: u32,
    velocity_x: f32,
    velocity_y: f32,
    velocity_z: f32,
    pressure: f32,
}

struct FluidConstants {
    world_size_x: u32,
    world_size_y: u32,
    world_size_z: u32,
    dt: f32,
    gravity: f32,
    pressure_iterations: u32,
    cell_size: f32,
    max_velocity: f32,
    viscosity_damping: f32,
    surface_tension: f32,
    _padding: f32,
}

struct BoundaryConditions {
    boundary_x_neg: u32,
    boundary_x_pos: u32,
    boundary_y_neg: u32,
    boundary_y_pos: u32,
    boundary_z_neg: u32,
    boundary_z_pos: u32,
    _padding: vec2<u32>,
}

@group(0) @binding(0) var<storage, read_write> fluid: array<FluidVoxel>;
@group(0) @binding(1) var<storage, read_write> divergence: array<f32>; // Reusing temp buffer
@group(0) @binding(2) var<uniform> constants: FluidConstants;
@group(0) @binding(3) var<uniform> boundaries: BoundaryConditions;

// Get fluid type from packed data
fn get_fluid_type(packed: u32) -> u32 {
    return packed & 0xFFu;
}

// Get fluid level
fn get_fluid_level(packed: u32) -> f32 {
    return f32((packed >> 8u) & 0xFFu) / 255.0;
}

// Get voxel index
fn get_index(x: u32, y: u32, z: u32) -> u32 {
    return x + y * constants.world_size_x + z * constants.world_size_x * constants.world_size_y;
}

// Check if position is solid boundary
fn is_solid_boundary(x: i32, y: i32, z: i32) -> bool {
    if (x < 0 && boundaries.boundary_x_neg == 1u) { return true; }
    if (x >= i32(constants.world_size_x) && boundaries.boundary_x_pos == 1u) { return true; }
    if (y < 0 && boundaries.boundary_y_neg == 1u) { return true; }
    if (y >= i32(constants.world_size_y) && boundaries.boundary_y_pos == 1u) { return true; }
    if (z < 0 && boundaries.boundary_z_neg == 1u) { return true; }
    if (z >= i32(constants.world_size_z) && boundaries.boundary_z_pos == 1u) { return true; }
    return false;
}

// Get velocity component at face between cells
fn get_face_velocity(x: i32, y: i32, z: i32, component: u32) -> f32 {
    // Check bounds
    if (x < 0 || x >= i32(constants.world_size_x) ||
        y < 0 || y >= i32(constants.world_size_y) ||
        z < 0 || z >= i32(constants.world_size_z)) {
        
        // Check if solid boundary
        if (is_solid_boundary(x, y, z)) {
            return 0.0; // No flow through solid
        }
        
        // Open boundary - use nearest valid cell
        let cx = clamp(x, 0, i32(constants.world_size_x) - 1);
        let cy = clamp(y, 0, i32(constants.world_size_y) - 1);
        let cz = clamp(z, 0, i32(constants.world_size_z) - 1);
        
        let idx = get_index(u32(cx), u32(cy), u32(cz));
        let voxel = fluid[idx];
        
        switch (component) {
            case 0u: { return voxel.velocity_x; }
            case 1u: { return voxel.velocity_y; }
            case 2u: { return voxel.velocity_z; }
            default: { return 0.0; }
        }
    }
    
    let idx = get_index(u32(x), u32(y), u32(z));
    let voxel = fluid[idx];
    
    // Only return velocity if cell contains fluid
    if (get_fluid_type(voxel.packed_data) > 0u && get_fluid_level(voxel.packed_data) > 0.01) {
        switch (component) {
            case 0u: { return voxel.velocity_x; }
            case 1u: { return voxel.velocity_y; }
            case 2u: { return voxel.velocity_z; }
            default: { return 0.0; }
        }
    }
    
    return 0.0;
}

@compute @workgroup_size(8, 8, 8)
fn divergence_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Check bounds
    if (global_id.x >= constants.world_size_x ||
        global_id.y >= constants.world_size_y ||
        global_id.z >= constants.world_size_z) {
        return;
    }
    
    let idx = get_index(global_id.x, global_id.y, global_id.z);
    let voxel = fluid[idx];
    
    let fluid_type = get_fluid_type(voxel.packed_data);
    let fluid_level = get_fluid_level(voxel.packed_data);
    
    // Only calculate divergence for fluid cells
    if (fluid_type == 0u || fluid_level < 0.01) {
        divergence[idx] = 0.0;
        return;
    }
    
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let z = i32(global_id.z);
    
    // Get velocities at cell faces
    // Note: Using staggered grid convention
    let vx_right = get_face_velocity(x + 1, y, z, 0u);
    let vx_left = get_face_velocity(x, y, z, 0u);
    
    let vy_top = get_face_velocity(x, y + 1, z, 1u);
    let vy_bottom = get_face_velocity(x, y, z, 1u);
    
    let vz_front = get_face_velocity(x, y, z + 1, 2u);
    let vz_back = get_face_velocity(x, y, z, 2u);
    
    // Calculate divergence
    let div = ((vx_right - vx_left) + 
               (vy_top - vy_bottom) + 
               (vz_front - vz_back)) / constants.cell_size;
    
    // Store negative divergence (for pressure solve)
    divergence[idx] = -div;
    
    // Also reset pressure for solve
    fluid[idx].pressure = 0.0;
}