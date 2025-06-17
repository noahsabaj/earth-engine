// Pressure projection shader - Make velocity field divergence-free

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
@group(0) @binding(1) var<storage, read> divergence: array<f32>; // Not used in projection
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

// Get neighbor pressure safely
fn get_neighbor_pressure(x: i32, y: i32, z: i32) -> f32 {
    // Check bounds
    if (x < 0 || x >= i32(constants.world_size_x) ||
        y < 0 || y >= i32(constants.world_size_y) ||
        z < 0 || z >= i32(constants.world_size_z)) {
        return 0.0;
    }
    
    let idx = get_index(u32(x), u32(y), u32(z));
    return fluid[idx].pressure;
}

// Check if neighbor is fluid
fn is_neighbor_fluid(x: i32, y: i32, z: i32) -> bool {
    if (x < 0 || x >= i32(constants.world_size_x) ||
        y < 0 || y >= i32(constants.world_size_y) ||
        z < 0 || z >= i32(constants.world_size_z)) {
        return !is_solid_boundary(x, y, z); // Open boundaries act like fluid
    }
    
    let idx = get_index(u32(x), u32(y), u32(z));
    let voxel = fluid[idx];
    
    return get_fluid_type(voxel.packed_data) > 0u && get_fluid_level(voxel.packed_data) > 0.01;
}

@compute @workgroup_size(8, 8, 4)
fn projection_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Check bounds
    if (global_id.x >= constants.world_size_x ||
        global_id.y >= constants.world_size_y ||
        global_id.z >= constants.world_size_z) {
        return;
    }
    
    let idx = get_index(global_id.x, global_id.y, global_id.z);
    var voxel = fluid[idx];
    
    let fluid_type = get_fluid_type(voxel.packed_data);
    let fluid_level = get_fluid_level(voxel.packed_data);
    
    // Only project velocity for fluid cells
    if (fluid_type == 0u || fluid_level < 0.01) {
        return;
    }
    
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let z = i32(global_id.z);
    
    // Get current pressure
    let p_center = voxel.pressure;
    
    // Calculate pressure gradient
    var grad_p = vec3<f32>(0.0);
    
    // X gradient
    if (is_neighbor_fluid(x + 1, y, z)) {
        let p_right = get_neighbor_pressure(x + 1, y, z);
        grad_p.x = (p_right - p_center) / constants.cell_size;
    } else if (is_solid_boundary(x + 1, y, z)) {
        grad_p.x = 0.0; // No flow through solid
    }
    
    if (is_neighbor_fluid(x - 1, y, z)) {
        let p_left = get_neighbor_pressure(x - 1, y, z);
        grad_p.x = (p_center - p_left) / constants.cell_size;
    } else if (is_solid_boundary(x - 1, y, z)) {
        grad_p.x = 0.0; // No flow through solid
    }
    
    // Y gradient
    if (is_neighbor_fluid(x, y + 1, z)) {
        let p_top = get_neighbor_pressure(x, y + 1, z);
        grad_p.y = (p_top - p_center) / constants.cell_size;
    } else if (is_solid_boundary(x, y + 1, z)) {
        grad_p.y = 0.0;
    }
    
    if (is_neighbor_fluid(x, y - 1, z)) {
        let p_bottom = get_neighbor_pressure(x, y - 1, z);
        grad_p.y = (p_center - p_bottom) / constants.cell_size;
    } else if (is_solid_boundary(x, y - 1, z)) {
        grad_p.y = 0.0;
    }
    
    // Z gradient
    if (is_neighbor_fluid(x, y, z + 1)) {
        let p_front = get_neighbor_pressure(x, y, z + 1);
        grad_p.z = (p_front - p_center) / constants.cell_size;
    } else if (is_solid_boundary(x, y, z + 1)) {
        grad_p.z = 0.0;
    }
    
    if (is_neighbor_fluid(x, y, z - 1)) {
        let p_back = get_neighbor_pressure(x, y, z - 1);
        grad_p.z = (p_center - p_back) / constants.cell_size;
    } else if (is_solid_boundary(x, y, z - 1)) {
        grad_p.z = 0.0;
    }
    
    // Get fluid density
    let density = select(1000.0, 1.2, fluid_type == 0u); // Water vs air density
    
    // Project velocity: v = v - dt * grad(p) / density
    voxel.velocity_x -= constants.dt * grad_p.x / density;
    voxel.velocity_y -= constants.dt * grad_p.y / density;
    voxel.velocity_z -= constants.dt * grad_p.z / density;
    
    // Apply boundary conditions
    if (x == 0u && boundaries.boundary_x_neg == 1u) {
        voxel.velocity_x = max(voxel.velocity_x, 0.0);
    }
    if (x == constants.world_size_x - 1u && boundaries.boundary_x_pos == 1u) {
        voxel.velocity_x = min(voxel.velocity_x, 0.0);
    }
    
    if (y == 0u && boundaries.boundary_y_neg == 1u) {
        voxel.velocity_y = max(voxel.velocity_y, 0.0);
    }
    if (y == constants.world_size_y - 1u && boundaries.boundary_y_pos == 1u) {
        voxel.velocity_y = min(voxel.velocity_y, 0.0);
    }
    
    if (z == 0u && boundaries.boundary_z_neg == 1u) {
        voxel.velocity_z = max(voxel.velocity_z, 0.0);
    }
    if (z == constants.world_size_z - 1u && boundaries.boundary_z_pos == 1u) {
        voxel.velocity_z = min(voxel.velocity_z, 0.0);
    }
    
    // Write back
    fluid[idx] = voxel;
}