// Pressure Jacobi iteration shader - Solve Poisson equation for pressure

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
@group(0) @binding(1) var<storage, read> divergence: array<f32>;
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

// Get neighbor pressure with boundary conditions
fn get_neighbor_pressure(x: i32, y: i32, z: i32, center_pressure: f32) -> f32 {
    // Check bounds
    if (x < 0 || x >= i32(constants.world_size_x) ||
        y < 0 || y >= i32(constants.world_size_y) ||
        z < 0 || z >= i32(constants.world_size_z)) {
        
        // Solid boundary - use center pressure (Neumann BC)
        if (is_solid_boundary(x, y, z)) {
            return center_pressure;
        }
        
        // Open boundary - zero pressure
        return 0.0;
    }
    
    let idx = get_index(u32(x), u32(y), u32(z));
    let voxel = fluid[idx];
    
    // Only use pressure from fluid cells
    if (get_fluid_type(voxel.packed_data) > 0u && get_fluid_level(voxel.packed_data) > 0.01) {
        return voxel.pressure;
    }
    
    // Air cells have zero pressure
    return 0.0;
}

// Count fluid neighbors
fn count_fluid_neighbors(x: i32, y: i32, z: i32) -> f32 {
    var count = 0.0;
    
    // Check each neighbor
    let neighbors = array<vec3<i32>, 6>(
        vec3<i32>(x + 1, y, z),
        vec3<i32>(x - 1, y, z),
        vec3<i32>(x, y + 1, z),
        vec3<i32>(x, y - 1, z),
        vec3<i32>(x, y, z + 1),
        vec3<i32>(x, y, z - 1)
    );
    
    for (var i = 0u; i < 6u; i = i + 1u) {
        let n = neighbors[i];
        
        // Check bounds
        if (n.x >= 0 && n.x < i32(constants.world_size_x) &&
            n.y >= 0 && n.y < i32(constants.world_size_y) &&
            n.z >= 0 && n.z < i32(constants.world_size_z)) {
            
            let idx = get_index(u32(n.x), u32(n.y), u32(n.z));
            let voxel = fluid[idx];
            
            if (get_fluid_type(voxel.packed_data) > 0u && get_fluid_level(voxel.packed_data) > 0.01) {
                count = count + 1.0;
            }
        } else if (!is_solid_boundary(n.x, n.y, n.z)) {
            // Open boundary counts as fluid neighbor
            count = count + 1.0;
        }
    }
    
    return count;
}

@compute @workgroup_size(8, 8, 8)
fn jacobi_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
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
    
    // Only solve pressure for fluid cells
    if (fluid_type == 0u || fluid_level < 0.01) {
        fluid[idx].pressure = 0.0;
        return;
    }
    
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let z = i32(global_id.z);
    
    // Get neighbor pressures
    let p_xp = get_neighbor_pressure(x + 1, y, z, voxel.pressure);
    let p_xn = get_neighbor_pressure(x - 1, y, z, voxel.pressure);
    let p_yp = get_neighbor_pressure(x, y + 1, z, voxel.pressure);
    let p_yn = get_neighbor_pressure(x, y - 1, z, voxel.pressure);
    let p_zp = get_neighbor_pressure(x, y, z + 1, voxel.pressure);
    let p_zn = get_neighbor_pressure(x, y, z - 1, voxel.pressure);
    
    // Get divergence
    let div = divergence[idx];
    
    // Count fluid neighbors for correct scaling
    let neighbor_count = count_fluid_neighbors(x, y, z);
    
    if (neighbor_count > 0.0) {
        // Jacobi iteration: p_new = (sum(p_neighbors) - h²*div) / neighbor_count
        let h2 = constants.cell_size * constants.cell_size;
        let sum_neighbors = p_xp + p_xn + p_yp + p_yn + p_zp + p_zn;
        
        // Update pressure
        fluid[idx].pressure = (sum_neighbors - h2 * div) / neighbor_count;
    } else {
        // Isolated cell - set pressure to zero
        fluid[idx].pressure = 0.0;
    }
}