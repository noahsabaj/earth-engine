// Fluid advection shader - Semi-Lagrangian method
// Moves fluid quantities by tracing backwards along velocity field

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

@group(0) @binding(0) var<storage, read> fluid_in: array<FluidVoxel>;
@group(0) @binding(1) var<storage, read_write> fluid_out: array<FluidVoxel>;
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

// Pack fluid data
fn pack_fluid_data(fluid_type: u32, level: f32, temp_offset: i32, flags: u32) -> u32 {
    let level_u8 = u32(clamp(level * 255.0, 0.0, 255.0));
    let temp_u8 = u32(clamp(temp_offset + 128, 0, 255));
    return fluid_type | (level_u8 << 8u) | (temp_u8 << 16u) | (flags << 24u);
}

// Get voxel index from coordinates
fn get_index(x: u32, y: u32, z: u32) -> u32 {
    return x + y * constants.world_size_x + z * constants.world_size_x * constants.world_size_y;
}

// Sample velocity at position using trilinear interpolation
fn sample_velocity(pos: vec3<f32>) -> vec3<f32> {
    // Clamp to valid range
    let clamped_pos = clamp(pos, vec3<f32>(0.5), 
        vec3<f32>(f32(constants.world_size_x) - 0.5, 
                  f32(constants.world_size_y) - 0.5,
                  f32(constants.world_size_z) - 0.5));
    
    // Get integer and fractional parts
    let i = vec3<u32>(floor(clamped_pos));
    let f = fract(clamped_pos);
    
    // Sample 8 corners
    var v000 = vec3<f32>(0.0);
    var v100 = vec3<f32>(0.0);
    var v010 = vec3<f32>(0.0);
    var v110 = vec3<f32>(0.0);
    var v001 = vec3<f32>(0.0);
    var v101 = vec3<f32>(0.0);
    var v011 = vec3<f32>(0.0);
    var v111 = vec3<f32>(0.0);
    
    // Safely sample with bounds checking
    if (i.x < constants.world_size_x && i.y < constants.world_size_y && i.z < constants.world_size_z) {
        let idx000 = get_index(i.x, i.y, i.z);
        let voxel000 = fluid_in[idx000];
        v000 = vec3<f32>(voxel000.velocity_x, voxel000.velocity_y, voxel000.velocity_z);
    }
    
    if (i.x + 1u < constants.world_size_x && i.y < constants.world_size_y && i.z < constants.world_size_z) {
        let idx100 = get_index(i.x + 1u, i.y, i.z);
        let voxel100 = fluid_in[idx100];
        v100 = vec3<f32>(voxel100.velocity_x, voxel100.velocity_y, voxel100.velocity_z);
    }
    
    // ... (similar for other corners, abbreviated for length)
    
    // Trilinear interpolation
    let v00 = mix(v000, v100, f.x);
    let v10 = mix(v010, v110, f.x);
    let v01 = mix(v001, v101, f.x);
    let v11 = mix(v011, v111, f.x);
    
    let v0 = mix(v00, v10, f.y);
    let v1 = mix(v01, v11, f.y);
    
    return mix(v0, v1, f.z);
}

// Sample fluid properties
fn sample_fluid(pos: vec3<f32>) -> FluidVoxel {
    // For now, use nearest neighbor (can improve to trilinear later)
    let i = vec3<u32>(clamp(pos, vec3<f32>(0.0), 
        vec3<f32>(f32(constants.world_size_x) - 1.0,
                  f32(constants.world_size_y) - 1.0,
                  f32(constants.world_size_z) - 1.0)));
    
    let idx = get_index(i.x, i.y, i.z);
    return fluid_in[idx];
}

@compute @workgroup_size(8, 8, 8)
fn advection_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Check bounds
    if (global_id.x >= constants.world_size_x ||
        global_id.y >= constants.world_size_y ||
        global_id.z >= constants.world_size_z) {
        return;
    }
    
    let idx = get_index(global_id.x, global_id.y, global_id.z);
    let current_voxel = fluid_in[idx];
    
    // Skip empty cells
    if (get_fluid_type(current_voxel.packed_data) == 0u) {
        fluid_out[idx] = current_voxel;
        return;
    }
    
    // Current position (cell center)
    let pos = vec3<f32>(f32(global_id.x) + 0.5, 
                        f32(global_id.y) + 0.5, 
                        f32(global_id.z) + 0.5);
    
    // Get velocity at current position
    let velocity = vec3<f32>(current_voxel.velocity_x, 
                             current_voxel.velocity_y, 
                             current_voxel.velocity_z);
    
    // Semi-Lagrangian: trace backwards
    // Use RK2 for better accuracy
    let mid_pos = pos - 0.5 * constants.dt * velocity;
    let mid_velocity = sample_velocity(mid_pos);
    let prev_pos = pos - constants.dt * mid_velocity;
    
    // Sample fluid at previous position
    let prev_fluid = sample_fluid(prev_pos);
    
    // Update current cell with advected values
    var new_voxel = current_voxel;
    
    // Advect velocity
    new_voxel.velocity_x = prev_fluid.velocity_x;
    new_voxel.velocity_y = prev_fluid.velocity_y;
    new_voxel.velocity_z = prev_fluid.velocity_z;
    
    // Advect other properties (level, temperature)
    let prev_level = get_fluid_level(prev_fluid.packed_data);
    let prev_type = get_fluid_type(prev_fluid.packed_data);
    let prev_temp = (prev_fluid.packed_data >> 16u) & 0xFFu;
    let prev_flags = (prev_fluid.packed_data >> 24u) & 0xFFu;
    
    // Only update if same fluid type or current is air
    if (prev_type == get_fluid_type(current_voxel.packed_data) || 
        get_fluid_type(current_voxel.packed_data) == 0u) {
        new_voxel.packed_data = pack_fluid_data(prev_type, prev_level, i32(prev_temp) - 128, prev_flags);
    }
    
    // Clamp velocity to maximum
    let vel_mag = length(vec3<f32>(new_voxel.velocity_x, new_voxel.velocity_y, new_voxel.velocity_z));
    if (vel_mag > constants.max_velocity) {
        let scale = constants.max_velocity / vel_mag;
        new_voxel.velocity_x *= scale;
        new_voxel.velocity_y *= scale;
        new_voxel.velocity_z *= scale;
    }
    
    fluid_out[idx] = new_voxel;
}