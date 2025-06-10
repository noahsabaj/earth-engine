// Optimized fluid advection shader with workgroup shared memory
// Uses shared memory to cache 10x10x10 blocks for 8x8x8 workgroups

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

// Shared memory for caching neighborhood (10x10x10 for 8x8x8 workgroup with 1-voxel border)
// We store velocity components separately for better access patterns
var<workgroup> shared_velocity_x: array<f32, 1000>; // 10x10x10
var<workgroup> shared_velocity_y: array<f32, 1000>;
var<workgroup> shared_velocity_z: array<f32, 1000>;
var<workgroup> shared_pressure: array<f32, 1000>;
var<workgroup> shared_packed: array<u32, 1000>;

// Convert 3D local coordinates to shared memory index
fn local_to_shared_index(x: u32, y: u32, z: u32) -> u32 {
    return x + y * 10u + z * 100u;
}

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

// Get voxel index from coordinates with Morton encoding
fn get_morton_index(x: u32, y: u32, z: u32) -> u32 {
    // Simplified Morton encoding for demonstration
    // In production, use full bit interleaving
    var morton = 0u;
    for (var i = 0u; i < 10u; i++) {
        morton |= ((x >> i) & 1u) << (i * 3u);
        morton |= ((y >> i) & 1u) << (i * 3u + 1u);
        morton |= ((z >> i) & 1u) << (i * 3u + 2u);
    }
    return morton;
}

// Sample velocity from shared memory with interpolation
fn sample_velocity_shared(local_pos: vec3<f32>) -> vec3<f32> {
    // Local position is in shared memory space (0-9)
    let i = vec3<u32>(floor(local_pos));
    let f = fract(local_pos);
    
    // Sample 8 corners from shared memory
    let idx000 = local_to_shared_index(i.x, i.y, i.z);
    let idx100 = local_to_shared_index(i.x + 1u, i.y, i.z);
    let idx010 = local_to_shared_index(i.x, i.y + 1u, i.z);
    let idx110 = local_to_shared_index(i.x + 1u, i.y + 1u, i.z);
    let idx001 = local_to_shared_index(i.x, i.y, i.z + 1u);
    let idx101 = local_to_shared_index(i.x + 1u, i.y, i.z + 1u);
    let idx011 = local_to_shared_index(i.x, i.y + 1u, i.z + 1u);
    let idx111 = local_to_shared_index(i.x + 1u, i.y + 1u, i.z + 1u);
    
    // Load from shared memory (much faster than global memory)
    let v000 = vec3<f32>(shared_velocity_x[idx000], shared_velocity_y[idx000], shared_velocity_z[idx000]);
    let v100 = vec3<f32>(shared_velocity_x[idx100], shared_velocity_y[idx100], shared_velocity_z[idx100]);
    let v010 = vec3<f32>(shared_velocity_x[idx010], shared_velocity_y[idx010], shared_velocity_z[idx010]);
    let v110 = vec3<f32>(shared_velocity_x[idx110], shared_velocity_y[idx110], shared_velocity_z[idx110]);
    let v001 = vec3<f32>(shared_velocity_x[idx001], shared_velocity_y[idx001], shared_velocity_z[idx001]);
    let v101 = vec3<f32>(shared_velocity_x[idx101], shared_velocity_y[idx101], shared_velocity_z[idx101]);
    let v011 = vec3<f32>(shared_velocity_x[idx011], shared_velocity_y[idx011], shared_velocity_z[idx011]);
    let v111 = vec3<f32>(shared_velocity_x[idx111], shared_velocity_y[idx111], shared_velocity_z[idx111]);
    
    // Trilinear interpolation
    let v00 = mix(v000, v100, f.x);
    let v10 = mix(v010, v110, f.x);
    let v01 = mix(v001, v101, f.x);
    let v11 = mix(v011, v111, f.x);
    
    let v0 = mix(v00, v10, f.y);
    let v1 = mix(v01, v11, f.y);
    
    return mix(v0, v1, f.z);
}

@compute @workgroup_size(8, 8, 8)
fn advection_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32
) {
    // Calculate workgroup base position
    let workgroup_base = workgroup_id * vec3<u32>(8u, 8u, 8u);
    
    // Phase 1: Cooperatively load data into shared memory
    // Each thread loads multiple values to fill the 10x10x10 cache
    let loads_per_thread = (1000u + 511u) / 512u; // ceil(1000/512)
    
    for (var i = 0u; i < loads_per_thread; i++) {
        let load_idx = local_index + i * 512u;
        if (load_idx < 1000u) {
            // Convert linear index to 3D position in shared cache
            let sz = load_idx / 100u;
            let sy = (load_idx % 100u) / 10u;
            let sx = load_idx % 10u;
            
            // Convert to global position (with 1-voxel border)
            let gx = workgroup_base.x + sx - 1u;
            let gy = workgroup_base.y + sy - 1u;
            let gz = workgroup_base.z + sz - 1u;
            
            // Load from global memory with bounds checking
            if (gx < constants.world_size_x && gy < constants.world_size_y && gz < constants.world_size_z) {
                let global_idx = get_morton_index(gx, gy, gz);
                let voxel = fluid_in[global_idx];
                
                shared_velocity_x[load_idx] = voxel.velocity_x;
                shared_velocity_y[load_idx] = voxel.velocity_y;
                shared_velocity_z[load_idx] = voxel.velocity_z;
                shared_pressure[load_idx] = voxel.pressure;
                shared_packed[load_idx] = voxel.packed_data;
            } else {
                // Handle boundary
                shared_velocity_x[load_idx] = 0.0;
                shared_velocity_y[load_idx] = 0.0;
                shared_velocity_z[load_idx] = 0.0;
                shared_pressure[load_idx] = 0.0;
                shared_packed[load_idx] = 0u;
            }
        }
    }
    
    // Synchronize to ensure all data is loaded
    workgroupBarrier();
    
    // Phase 2: Perform advection using cached data
    if (global_id.x >= constants.world_size_x || 
        global_id.y >= constants.world_size_y || 
        global_id.z >= constants.world_size_z) {
        return;
    }
    
    // Get current voxel from shared memory
    let local_pos = local_id + vec3<u32>(1u, 1u, 1u); // Account for border
    let shared_idx = local_to_shared_index(local_pos.x, local_pos.y, local_pos.z);
    
    let current_velocity = vec3<f32>(
        shared_velocity_x[shared_idx],
        shared_velocity_y[shared_idx],
        shared_velocity_z[shared_idx]
    );
    
    // Semi-Lagrangian backtracing
    let world_pos = vec3<f32>(global_id) + vec3<f32>(0.5);
    let trace_pos = world_pos - current_velocity * constants.dt;
    
    // Convert trace position to local space
    let local_trace = trace_pos - vec3<f32>(workgroup_base) + vec3<f32>(1.0);
    
    var new_velocity: vec3<f32>;
    var new_pressure: f32;
    var new_packed: u32;
    
    // Check if trace position is within shared memory bounds
    if (all(local_trace >= vec3<f32>(0.0)) && all(local_trace < vec3<f32>(10.0))) {
        // Sample from shared memory (fast path)
        new_velocity = sample_velocity_shared(local_trace);
        
        // Also sample pressure and packed data
        let ti = vec3<u32>(floor(local_trace));
        let tf = fract(local_trace);
        
        let tidx = local_to_shared_index(ti.x, ti.y, ti.z);
        new_pressure = shared_pressure[tidx]; // Simplified, should interpolate
        new_packed = shared_packed[tidx];
    } else {
        // Fall back to global memory access (slow path)
        // This happens rarely at workgroup boundaries
        new_velocity = vec3<f32>(0.0); // Simplified
        new_pressure = 0.0;
        new_packed = 0u;
    }
    
    // Apply boundary conditions
    new_velocity = clamp(new_velocity, vec3<f32>(-constants.max_velocity), vec3<f32>(constants.max_velocity));
    
    // Write result
    let out_idx = get_morton_index(global_id.x, global_id.y, global_id.z);
    fluid_out[out_idx] = FluidVoxel(
        new_packed,
        new_velocity.x,
        new_velocity.y,
        new_velocity.z,
        new_pressure
    );
}