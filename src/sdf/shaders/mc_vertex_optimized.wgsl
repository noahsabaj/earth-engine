// Optimized marching cubes vertex generation with shared memory caching
// Caches 10x10x10 SDF values for 8x8x8 workgroups

struct SdfValue {
    distance: f32,
    material: u32,
}

struct SmoothVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    material_weights: vec4<f32>,
    material_ids: vec4<u32>,
}

struct GridConstants {
    grid_size: vec3<u32>,
    voxel_size: f32,
    iso_level: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> sdf: array<SdfValue>;
@group(0) @binding(1) var<storage, read> cell_types: array<u32>;
@group(0) @binding(2) var<uniform> constants: GridConstants;
@group(0) @binding(3) var<storage, read> edge_table: array<u32, 256>;
@group(0) @binding(4) var<storage, read_write> vertices: array<SmoothVertex>;
@group(0) @binding(5) var<storage, read_write> vertex_count: atomic<u32>;

// Shared memory for caching SDF values (10x10x10 for 8x8x8 workgroup with borders)
var<workgroup> shared_sdf_distance: array<f32, 1000>;
var<workgroup> shared_sdf_material: array<u32, 1000>;

// Convert 3D local coordinates to shared memory index
fn local_to_shared_index(x: u32, y: u32, z: u32) -> u32 {
    return x + y * 10u + z * 100u;
}

// Get Morton-encoded index for better cache performance
fn get_morton_index(x: u32, y: u32, z: u32) -> u32 {
    var morton = 0u;
    for (var i = 0u; i < 10u; i++) {
        morton |= ((x >> i) & 1u) << (i * 3u);
        morton |= ((y >> i) & 1u) << (i * 3u + 1u);
        morton |= ((z >> i) & 1u) << (i * 3u + 2u);
    }
    return morton;
}

// Sample SDF value from shared memory
fn sample_sdf_shared(local_pos: vec3<u32>) -> SdfValue {
    let idx = local_to_shared_index(local_pos.x, local_pos.y, local_pos.z);
    return SdfValue(shared_sdf_distance[idx], shared_sdf_material[idx]);
}

// Compute gradient from shared memory (for normals)
fn compute_gradient_shared(local_pos: vec3<u32>) -> vec3<f32> {
    // Central differences using cached values
    let dx = sample_sdf_shared(local_pos + vec3<u32>(1u, 0u, 0u)).distance - 
             sample_sdf_shared(local_pos - vec3<u32>(1u, 0u, 0u)).distance;
    let dy = sample_sdf_shared(local_pos + vec3<u32>(0u, 1u, 0u)).distance - 
             sample_sdf_shared(local_pos - vec3<u32>(0u, 1u, 0u)).distance;
    let dz = sample_sdf_shared(local_pos + vec3<u32>(0u, 0u, 1u)).distance - 
             sample_sdf_shared(local_pos - vec3<u32>(0u, 0u, 1u)).distance;
    
    return normalize(vec3<f32>(dx, dy, dz));
}

// Vertex interpolation along an edge
fn vertex_interp(iso_level: f32, p1: vec3<f32>, p2: vec3<f32>, val1: f32, val2: f32) -> vec3<f32> {
    if (abs(iso_level - val1) < 0.00001) {
        return p1;
    }
    if (abs(iso_level - val2) < 0.00001) {
        return p2;
    }
    if (abs(val1 - val2) < 0.00001) {
        return p1;
    }
    
    let mu = (iso_level - val1) / (val2 - val1);
    return p1 + mu * (p2 - p1);
}

@compute @workgroup_size(8, 8, 4)
fn generate_vertices(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32
) {
    let workgroup_base = workgroup_id * vec3<u32>(8u, 8u, 8u);
    
    // Phase 1: Cooperatively load SDF data into shared memory
    let loads_per_thread = (1000u + 511u) / 512u;
    
    for (var i = 0u; i < loads_per_thread; i++) {
        let load_idx = local_index + i * 512u;
        if (load_idx < 1000u) {
            // Convert linear index to 3D position
            let sz = load_idx / 100u;
            let sy = (load_idx % 100u) / 10u;
            let sx = load_idx % 10u;
            
            // Convert to global position (with 1-voxel border)
            let gx = workgroup_base.x + sx - 1u;
            let gy = workgroup_base.y + sy - 1u;
            let gz = workgroup_base.z + sz - 1u;
            
            // Load from global memory with bounds checking
            if (gx < constants.grid_size.x && gy < constants.grid_size.y && gz < constants.grid_size.z) {
                let global_idx = get_morton_index(gx, gy, gz);
                let sdf_val = sdf[global_idx];
                
                shared_sdf_distance[load_idx] = sdf_val.distance;
                shared_sdf_material[load_idx] = sdf_val.material;
            } else {
                // Handle boundary - set to positive distance (outside)
                shared_sdf_distance[load_idx] = 1.0;
                shared_sdf_material[load_idx] = 0u;
            }
        }
    }
    
    workgroupBarrier();
    
    // Phase 2: Process marching cubes using cached data
    if (global_id.x >= constants.grid_size.x - 1u || 
        global_id.y >= constants.grid_size.y - 1u || 
        global_id.z >= constants.grid_size.z - 1u) {
        return;
    }
    
    // Get cube configuration from shared memory
    let local_base = local_id + vec3<u32>(1u, 1u, 1u);
    var cube_index = 0u;
    var grid_val: array<f32, 8>;
    var grid_mat: array<u32, 8>;
    
    // Sample 8 corners of the cube from shared memory
    for (var i = 0u; i < 8u; i++) {
        let corner_offset = vec3<u32>(
            (i & 1u),
            (i >> 1u) & 1u,
            (i >> 2u) & 1u
        );
        let corner_pos = local_base + corner_offset;
        let corner_sdf = sample_sdf_shared(corner_pos);
        
        grid_val[i] = corner_sdf.distance;
        grid_mat[i] = corner_sdf.material;
        
        if (grid_val[i] < constants.iso_level) {
            cube_index |= (1u << i);
        }
    }
    
    // Check if cube is entirely inside or outside
    if (cube_index == 0u || cube_index == 255u) {
        return;
    }
    
    // Get edge configuration from table
    let edge_flags = edge_table[cube_index];
    if (edge_flags == 0u) {
        return;
    }
    
    // Vertex positions for the cube
    let base_pos = vec3<f32>(global_id) * constants.voxel_size;
    var edge_verts: array<vec3<f32>, 12>;
    var edge_norms: array<vec3<f32>, 12>;
    
    // Generate vertices along edges
    if ((edge_flags & 1u) != 0u) {
        edge_verts[0] = vertex_interp(constants.iso_level,
            base_pos,
            base_pos + vec3<f32>(constants.voxel_size, 0.0, 0.0),
            grid_val[0], grid_val[1]);
        edge_norms[0] = compute_gradient_shared(local_base);
    }
    
    if ((edge_flags & 2u) != 0u) {
        edge_verts[1] = vertex_interp(constants.iso_level,
            base_pos + vec3<f32>(constants.voxel_size, 0.0, 0.0),
            base_pos + vec3<f32>(constants.voxel_size, constants.voxel_size, 0.0),
            grid_val[1], grid_val[2]);
        edge_norms[1] = compute_gradient_shared(local_base + vec3<u32>(1u, 0u, 0u));
    }
    
    // ... Continue for all 12 edges ...
    // (abbreviated for brevity - would include all edge cases)
    
    // Output vertices using atomic counter
    // In real implementation, would output triangles based on tri_table
    let vertex_idx = atomicAdd(&vertex_count, 1u);
    if (vertex_idx < arrayLength(&vertices)) {
        vertices[vertex_idx] = SmoothVertex(
            edge_verts[0],
            edge_norms[0],
            vec4<f32>(1.0, 0.0, 0.0, 0.0),
            vec4<u32>(grid_mat[0], 0u, 0u, 0u)
        );
    }
}

// Separate pass for triangle generation could also use shared memory
@compute @workgroup_size(256, 1, 1)
fn generate_triangles(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    // Triangle generation based on marching cubes tables
    // Would also benefit from shared memory for vertex lookups
}