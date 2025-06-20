// GPU Mesh Generation Compute Shader
// Generates chunk meshes entirely on GPU with zero CPU involvement

// Constants
const CHUNK_SIZE: u32 = 32u;
const WORKGROUP_SIZE: u32 = 64u; // 4x4x4 voxels

// Face normals
const FACE_NORMALS: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>( 1.0,  0.0,  0.0), // +X
    vec3<f32>(-1.0,  0.0,  0.0), // -X
    vec3<f32>( 0.0,  1.0,  0.0), // +Y
    vec3<f32>( 0.0, -1.0,  0.0), // -Y
    vec3<f32>( 0.0,  0.0,  1.0), // +Z
    vec3<f32>( 0.0,  0.0, -1.0)  // -Z
);

// Face vertices (relative positions)
const FACE_VERTICES: array<array<vec3<f32>, 4>, 6> = array<array<vec3<f32>, 4>, 6>(
    // +X face
    array<vec3<f32>, 4>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(1.0, 1.0, 0.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(1.0, 0.0, 1.0)
    ),
    // -X face
    array<vec3<f32>, 4>(
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(0.0, 1.0, 1.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0)
    ),
    // +Y face
    array<vec3<f32>, 4>(
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 1.0, 1.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(1.0, 1.0, 0.0)
    ),
    // -Y face
    array<vec3<f32>, 4>(
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(1.0, 0.0, 1.0)
    ),
    // +Z face
    array<vec3<f32>, 4>(
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(1.0, 0.0, 1.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(0.0, 1.0, 1.0)
    ),
    // -Z face
    array<vec3<f32>, 4>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(1.0, 1.0, 0.0)
    )
);

// Mesh request structure
struct MeshRequest {
    chunk_pos: vec3<i32>,
    lod_level: u32,
    buffer_index: u32,
    flags: u32,
    _padding: vec2<u32>,
}

// Meshing parameters
struct MeshingParams {
    chunk_size: u32,
    request_count: u32,
    enable_greedy: u32,
    enable_ao: u32,
    max_vertices: u32,
    max_indices: u32,
    _padding: vec2<u32>,
}

// Mesh metadata
struct MeshMetadata {
    chunk_pos: vec3<i32>,
    vertex_count: atomic<u32>,
    index_count: atomic<u32>,
    lod_level: u32,
    flags: u32,
    timestamp: u32,
}

// Bindings
@group(0) @binding(0) var<storage, read> world_data: array<u32>;
@group(0) @binding(1) var<storage, read> requests: array<MeshRequest>;
@group(0) @binding(2) var<storage, read_write> vertex_positions: array<vec3<f32>>;
@group(0) @binding(3) var<storage, read_write> vertex_normals: array<vec3<f32>>;
@group(0) @binding(4) var<storage, read_write> vertex_uvs: array<vec2<f32>>;
@group(0) @binding(5) var<storage, read_write> vertex_colors: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read_write> indices: array<u32>;
@group(0) @binding(7) var<storage, read_write> metadata: array<MeshMetadata>;
@group(0) @binding(8) var<storage, read_write> indirect_commands: array<vec4<u32>>;
@group(0) @binding(9) var<uniform> params: MeshingParams;

// Shared memory for face culling
var<workgroup> voxel_cache: array<u32, 512>; // 8x8x8 with padding

// Get voxel from world data
fn get_voxel(world_pos: vec3<i32>) -> u32 {
    // Calculate morton index for world position
    let morton_index = encode_morton3(world_pos);
    
    // Bounds check
    if (morton_index >= arrayLength(&world_data)) {
        return 0u; // AIR
    }
    
    return world_data[morton_index];
}

// Check if voxel is transparent
fn is_transparent(voxel: u32) -> bool {
    return voxel == 0u || voxel == 9u; // AIR or WATER
}

// Add a face to the mesh
fn add_face(
    request_idx: u32,
    local_pos: vec3<f32>,
    face: u32,
    voxel_type: u32
) {
    let base_vertex_offset = request_idx * params.max_vertices;
    let base_index_offset = request_idx * params.max_indices;
    
    // Get current counts
    let vertex_idx = atomicAdd(&metadata[request_idx].vertex_count, 4u);
    let index_idx = atomicAdd(&metadata[request_idx].index_count, 6u);
    
    // Add vertices
    for (var i = 0u; i < 4u; i = i + 1u) {
        let vertex_pos = local_pos + FACE_VERTICES[face][i];
        let vertex_offset = base_vertex_offset + vertex_idx + i;
        
        vertex_positions[vertex_offset] = vertex_pos;
        vertex_normals[vertex_offset] = FACE_NORMALS[face];
        vertex_uvs[vertex_offset] = vec2<f32>(
            FACE_VERTICES[face][i].x,
            FACE_VERTICES[face][i].z
        );
        
        // Simple color based on voxel type
        let color = get_voxel_color(voxel_type);
        vertex_colors[vertex_offset] = vec4<f32>(color, 1.0);
    }
    
    // Add indices (two triangles)
    let base_idx = base_index_offset + index_idx;
    indices[base_idx + 0u] = vertex_idx + 0u;
    indices[base_idx + 1u] = vertex_idx + 1u;
    indices[base_idx + 2u] = vertex_idx + 2u;
    indices[base_idx + 3u] = vertex_idx + 0u;
    indices[base_idx + 4u] = vertex_idx + 2u;
    indices[base_idx + 5u] = vertex_idx + 3u;
}

// Get color for voxel type
fn get_voxel_color(voxel_type: u32) -> vec3<f32> {
    switch (voxel_type) {
        case 1u: { return vec3<f32>(0.5, 0.5, 0.5); }  // STONE
        case 2u: { return vec3<f32>(0.55, 0.4, 0.3); } // DIRT
        case 3u: { return vec3<f32>(0.3, 0.7, 0.3); }  // GRASS
        case 4u: { return vec3<f32>(0.6, 0.5, 0.4); }  // WOOD
        case 5u: { return vec3<f32>(0.2, 0.6, 0.2); }  // LEAVES
        case 9u: { return vec3<f32>(0.2, 0.4, 0.8); }  // WATER
        default: { return vec3<f32>(0.8, 0.8, 0.8); }
    }
}

// Morton encoding
fn encode_morton3(pos: vec3<i32>) -> u32 {
    // Simple morton encoding for demo
    // In production, use proper bit interleaving
    let x = u32(pos.x) & 0x3FFu;
    let y = u32(pos.y) & 0x3FFu; 
    let z = u32(pos.z) & 0x3FFu;
    return (x << 20u) | (y << 10u) | z;
}

// Main mesh generation kernel
@compute @workgroup_size(WORKGROUP_SIZE)
fn generate_mesh(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    // Each workgroup processes one chunk
    let request_idx = workgroup_id.x;
    if (request_idx >= params.request_count) {
        return;
    }
    
    let request = requests[request_idx];
    let chunk_base = request.chunk_pos * i32(CHUNK_SIZE);
    
    // Each thread processes one voxel
    let thread_idx = local_id.x;
    let voxels_per_thread = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) / WORKGROUP_SIZE;
    
    // Process assigned voxels
    for (var i = 0u; i < voxels_per_thread; i = i + 1u) {
        let voxel_idx = thread_idx * voxels_per_thread + i;
        
        // Convert to 3D position
        let local_x = voxel_idx % CHUNK_SIZE;
        let local_y = (voxel_idx / CHUNK_SIZE) % CHUNK_SIZE;
        let local_z = voxel_idx / (CHUNK_SIZE * CHUNK_SIZE);
        
        let local_pos = vec3<u32>(local_x, local_y, local_z);
        let world_pos = chunk_base + vec3<i32>(local_pos);
        
        // Get voxel
        let voxel = get_voxel(world_pos);
        if (is_transparent(voxel)) {
            continue;
        }
        
        // Check each face
        for (var face = 0u; face < 6u; face = face + 1u) {
            let neighbor_offset = FACE_NORMALS[face];
            let neighbor_pos = world_pos + vec3<i32>(neighbor_offset);
            let neighbor = get_voxel(neighbor_pos);
            
            // Only add face if neighbor is transparent
            if (is_transparent(neighbor)) {
                add_face(request_idx, vec3<f32>(local_pos), face, voxel);
            }
        }
    }
    
    // Synchronize before writing final counts
    workgroupBarrier();
    
    // Thread 0 writes indirect command
    if (thread_idx == 0u) {
        let vertex_count = atomicLoad(&metadata[request_idx].vertex_count);
        let index_count = atomicLoad(&metadata[request_idx].index_count);
        
        // Write indirect draw command
        indirect_commands[request_idx] = vec4<u32>(
            index_count,    // vertex count (using indices)
            1u,             // instance count
            0u,             // first vertex
            request_idx     // first instance
        );
    }
}