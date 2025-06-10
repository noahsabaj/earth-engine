// Fluid forces shader - Apply gravity and external forces

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
@group(0) @binding(1) var<storage, read_write> temp: array<FluidVoxel>; // Not used in forces
@group(0) @binding(2) var<uniform> constants: FluidConstants;
@group(0) @binding(3) var<uniform> boundaries: BoundaryConditions;

// Get fluid type from packed data
fn get_fluid_type(packed: u32) -> u32 {
    return packed & 0xFFu;
}

// Get fluid level from packed data
fn get_fluid_level(packed: u32) -> f32 {
    return f32((packed >> 8u) & 0xFFu) / 255.0;
}

// Get temperature offset
fn get_temp_offset(packed: u32) -> i32 {
    return i32((packed >> 16u) & 0xFFu) - 128;
}

// Check if neighbor is solid
fn is_neighbor_solid(packed: u32, direction: u32) -> bool {
    let bit = 24u + direction;
    return (packed & (1u << bit)) != 0u;
}

// Get voxel index
fn get_index(x: u32, y: u32, z: u32) -> u32 {
    return x + y * constants.world_size_x + z * constants.world_size_x * constants.world_size_y;
}

// Apply boundary conditions to velocity
fn apply_boundary_velocity(vel: vec3<f32>, x: u32, y: u32, z: u32) -> vec3<f32> {
    var new_vel = vel;
    
    // X boundaries
    if (x == 0u && boundaries.boundary_x_neg == 1u) {
        new_vel.x = max(new_vel.x, 0.0); // No flow through solid wall
    }
    if (x == constants.world_size_x - 1u && boundaries.boundary_x_pos == 1u) {
        new_vel.x = min(new_vel.x, 0.0);
    }
    
    // Y boundaries
    if (y == 0u && boundaries.boundary_y_neg == 1u) {
        new_vel.y = max(new_vel.y, 0.0);
    }
    if (y == constants.world_size_y - 1u && boundaries.boundary_y_pos == 1u) {
        new_vel.y = min(new_vel.y, 0.0);
    }
    
    // Z boundaries
    if (z == 0u && boundaries.boundary_z_neg == 1u) {
        new_vel.z = max(new_vel.z, 0.0);
    }
    if (z == constants.world_size_z - 1u && boundaries.boundary_z_pos == 1u) {
        new_vel.z = min(new_vel.z, 0.0);
    }
    
    return new_vel;
}

// Get fluid density based on type
fn get_fluid_density(fluid_type: u32) -> f32 {
    switch (fluid_type) {
        case 0u: { return 1.2; }      // Air
        case 1u: { return 1000.0; }   // Water
        case 2u: { return 3100.0; }   // Lava
        case 3u: { return 800.0; }    // Oil
        case 4u: { return 0.6; }      // Steam
        case 5u: { return 0.3; }      // Smoke
        default: { return 1000.0; }
    }
}

// Calculate buoyancy force
fn calculate_buoyancy(fluid_type: u32, y: u32) -> f32 {
    let density = get_fluid_density(fluid_type);
    let air_density = 1.2;
    
    // Buoyancy for gases (steam, smoke)
    if (fluid_type == 4u || fluid_type == 5u) {
        return -constants.gravity * (density - air_density) / density;
    }
    
    return 0.0;
}

@compute @workgroup_size(8, 8, 8)
fn forces_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Check bounds
    if (global_id.x >= constants.world_size_x ||
        global_id.y >= constants.world_size_y ||
        global_id.z >= constants.world_size_z) {
        return;
    }
    
    let idx = get_index(global_id.x, global_id.y, global_id.z);
    var voxel = fluid[idx];
    
    let fluid_type = get_fluid_type(voxel.packed_data);
    
    // Skip air cells
    if (fluid_type == 0u) {
        return;
    }
    
    let fluid_level = get_fluid_level(voxel.packed_data);
    
    // Only apply forces if there's fluid
    if (fluid_level > 0.01) {
        // Apply gravity (negative Y)
        voxel.velocity_y += constants.gravity * constants.dt;
        
        // Apply buoyancy for light fluids
        let buoyancy = calculate_buoyancy(fluid_type, global_id.y);
        voxel.velocity_y += buoyancy * constants.dt;
        
        // Temperature-based forces (hot fluids rise)
        let temp_offset = get_temp_offset(voxel.packed_data);
        if (temp_offset > 0) {
            // Hot fluid rises
            let temp_force = f32(temp_offset) * 0.01; // Adjust strength as needed
            voxel.velocity_y += temp_force * constants.dt;
        }
        
        // Apply viscosity damping
        voxel.velocity_x *= constants.viscosity_damping;
        voxel.velocity_y *= constants.viscosity_damping;
        voxel.velocity_z *= constants.viscosity_damping;
        
        // Apply boundary conditions
        let new_velocity = apply_boundary_velocity(
            vec3<f32>(voxel.velocity_x, voxel.velocity_y, voxel.velocity_z),
            global_id.x, global_id.y, global_id.z
        );
        
        voxel.velocity_x = new_velocity.x;
        voxel.velocity_y = new_velocity.y;
        voxel.velocity_z = new_velocity.z;
        
        // Clamp velocity to maximum
        let vel_mag = length(vec3<f32>(voxel.velocity_x, voxel.velocity_y, voxel.velocity_z));
        if (vel_mag > constants.max_velocity) {
            let scale = constants.max_velocity / vel_mag;
            voxel.velocity_x *= scale;
            voxel.velocity_y *= scale;
            voxel.velocity_z *= scale;
        }
    }
    
    fluid[idx] = voxel;
}