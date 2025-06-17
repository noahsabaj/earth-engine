// Fluid viscosity shader - Diffuse velocity based on fluid viscosity

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

@group(0) @binding(0) var<storage, read> fluid_in: array<FluidVoxel>;
@group(0) @binding(1) var<storage, read_write> fluid_out: array<FluidVoxel>;
@group(0) @binding(2) var<uniform> constants: FluidConstants;

// Get fluid type from packed data
fn get_fluid_type(packed: u32) -> u32 {
    return packed & 0xFFu;
}

// Get fluid viscosity based on type
fn get_fluid_viscosity(fluid_type: u32) -> f32 {
    switch (fluid_type) {
        case 0u: { return 0.00001; }  // Air
        case 1u: { return 0.001; }    // Water
        case 2u: { return 100.0; }    // Lava (very viscous)
        case 3u: { return 0.1; }      // Oil
        case 4u: { return 0.00001; }  // Steam
        case 5u: { return 0.00001; }  // Smoke
        default: { return 0.001; }
    }
}

// Get voxel index
fn get_index(x: u32, y: u32, z: u32) -> u32 {
    return x + y * constants.world_size_x + z * constants.world_size_x * constants.world_size_y;
}

// Safe neighbor access
fn get_neighbor_velocity(x: i32, y: i32, z: i32) -> vec3<f32> {
    // Clamp to bounds
    let cx = clamp(x, 0, i32(constants.world_size_x) - 1);
    let cy = clamp(y, 0, i32(constants.world_size_y) - 1);
    let cz = clamp(z, 0, i32(constants.world_size_z) - 1);
    
    let idx = get_index(u32(cx), u32(cy), u32(cz));
    let voxel = fluid_in[idx];
    
    // Only return velocity if same fluid type or compatible
    let fluid_type = get_fluid_type(voxel.packed_data);
    if (fluid_type > 0u) {
        return vec3<f32>(voxel.velocity_x, voxel.velocity_y, voxel.velocity_z);
    }
    
    return vec3<f32>(0.0);
}

@compute @workgroup_size(8, 8, 4)
fn viscosity_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Check bounds
    if (global_id.x >= constants.world_size_x ||
        global_id.y >= constants.world_size_y ||
        global_id.z >= constants.world_size_z) {
        return;
    }
    
    let idx = get_index(global_id.x, global_id.y, global_id.z);
    let current_voxel = fluid_in[idx];
    
    let fluid_type = get_fluid_type(current_voxel.packed_data);
    
    // Skip air cells
    if (fluid_type == 0u) {
        fluid_out[idx] = current_voxel;
        return;
    }
    
    let viscosity = get_fluid_viscosity(fluid_type);
    
    // Low viscosity fluids don't need diffusion
    if (viscosity < 0.01) {
        fluid_out[idx] = current_voxel;
        return;
    }
    
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let z = i32(global_id.z);
    
    // Get current velocity
    let current_vel = vec3<f32>(current_voxel.velocity_x, current_voxel.velocity_y, current_voxel.velocity_z);
    
    // Get neighbor velocities
    let vel_xp = get_neighbor_velocity(x + 1, y, z);
    let vel_xn = get_neighbor_velocity(x - 1, y, z);
    let vel_yp = get_neighbor_velocity(x, y + 1, z);
    let vel_yn = get_neighbor_velocity(x, y - 1, z);
    let vel_zp = get_neighbor_velocity(x, y, z + 1);
    let vel_zn = get_neighbor_velocity(x, y, z - 1);
    
    // Calculate laplacian of velocity
    let laplacian = (vel_xp + vel_xn + vel_yp + vel_yn + vel_zp + vel_zn - 6.0 * current_vel) 
                    / (constants.cell_size * constants.cell_size);
    
    // Apply viscosity diffusion
    let diffusion_rate = viscosity * constants.dt / (constants.cell_size * constants.cell_size);
    let diffusion_factor = min(diffusion_rate, 0.5); // Stability limit
    
    let new_velocity = current_vel + diffusion_factor * laplacian;
    
    // Update voxel
    var new_voxel = current_voxel;
    new_voxel.velocity_x = new_velocity.x;
    new_voxel.velocity_y = new_velocity.y;
    new_voxel.velocity_z = new_velocity.z;
    
    fluid_out[idx] = new_voxel;
}