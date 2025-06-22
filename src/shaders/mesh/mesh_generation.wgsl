// GPU Mesh Generation Compute Shader
// Generates chunk meshes entirely on GPU with zero CPU involvement

// Constants
const CHUNK_SIZE: u32 = 32u;
const WORKGROUP_SIZE: u32 = 64u; // 4x4x4 voxels

// Face constants for clarity
// Faces are encoded as: 0=+X, 1=-X, 2=+Y, 3=-Y, 4=+Z, 5=-Z

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

// Vertex structure matching renderer expectations
struct Vertex {
    position: vec3<f32>,
    color: vec3<f32>,
    normal: vec3<f32>,
    light: f32,
    ao: f32,
}

// Bindings
@group(0) @binding(0) var<storage, read> world_data: array<u32>;
@group(0) @binding(1) var<storage, read> requests: array<MeshRequest>;
@group(0) @binding(2) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(3) var<storage, read_write> indices: array<u32>;
@group(0) @binding(4) var<storage, read_write> metadata: array<MeshMetadata>;
@group(0) @binding(5) var<storage, read_write> indirect_commands: array<vec4<u32>>;
@group(0) @binding(6) var<uniform> params: MeshingParams;

// Shared memory for face culling
var<workgroup> voxel_cache: array<u32, 512>; // 8x8x8 with padding

// Get voxel from world data
fn get_voxel(world_pos: vec3<i32>) -> u32 {
    // For now, return a simple pattern to test if mesh generation works at all
    // TODO: Fix morton encoding for negative coordinates
    
    // Create a simple terrain pattern for testing
    if (world_pos.y < 64) {
        // Below y=64, return stone
        return 1u; // STONE
    } else if (world_pos.y == 64) {
        // At y=64, return grass
        return 3u; // GRASS
    } else {
        // Above y=64, return air
        return 0u; // AIR
    }
}

// Check if voxel is transparent
fn is_transparent(voxel: u32) -> bool {
    return voxel == 0u || voxel == 9u; // AIR or WATER
}

// Compute face vertex position algorithmically
// Face encoding: 0=+X, 1=-X, 2=+Y, 3=-Y, 4=+Z, 5=-Z
// Vertex encoding follows quad winding: 0=BL, 1=BR, 2=TR, 3=TL
fn compute_face_vertex(face: u32, vertex_idx: u32) -> vec3<f32> {
    // Standard quad vertices in 2D (CCW winding when viewed from outside)
    // 0: (0,0), 1: (1,0), 2: (1,1), 3: (0,1)
    let u = f32(vertex_idx == 1u || vertex_idx == 2u);
    let v = f32(vertex_idx == 2u || vertex_idx == 3u);
    
    // Map to 3D based on face orientation
    switch face {
        case 0u: { return vec3<f32>(1.0, v, u); }      // +X: YZ plane at X=1
        case 1u: { return vec3<f32>(0.0, v, 1.0 - u); } // -X: YZ plane at X=0 (flipped U)
        case 2u: { return vec3<f32>(u, 1.0, v); }      // +Y: XZ plane at Y=1  
        case 3u: { return vec3<f32>(u, 0.0, 1.0 - v); } // -Y: XZ plane at Y=0 (flipped V)
        case 4u: { return vec3<f32>(u, v, 1.0); }      // +Z: XY plane at Z=1
        case 5u: { return vec3<f32>(1.0 - u, v, 0.0); } // -Z: XY plane at Z=0 (flipped U)
        default: { return vec3<f32>(0.0, 0.0, 0.0); }
    }
}

// Get just the normal for a face
fn compute_face_normal(face: u32) -> vec3<f32> {
    let axis = face / 2u;
    let positive = (face & 1u) == 0u;
    
    var normal = vec3<f32>(0.0, 0.0, 0.0);
    if (axis == 0u) {
        normal.x = select(-1.0, 1.0, positive);
    } else if (axis == 1u) {
        normal.y = select(-1.0, 1.0, positive);
    } else {
        normal.z = select(-1.0, 1.0, positive);
    }
    
    return normal;
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
    
    // Get face color
    let color = get_voxel_color(voxel_type);
    let normal = compute_face_normal(face);
    
    // Add vertices
    for (var i = 0u; i < 4u; i = i + 1u) {
        let vertex_pos = local_pos + compute_face_vertex(face, i);
        let vertex_offset = base_vertex_offset + vertex_idx + i;
        
        // Create vertex with all attributes
        var vertex: Vertex;
        vertex.position = vertex_pos;
        vertex.color = color;
        vertex.normal = normal;
        vertex.light = 1.0;  // Full light for now
        vertex.ao = 1.0;     // No ambient occlusion for now
        
        vertices[vertex_offset] = vertex;
    }
    
    // Add indices (two triangles)
    // Indices need to be absolute, including the base_vertex_offset
    let absolute_vertex_base = base_vertex_offset + vertex_idx;
    let base_idx = base_index_offset + index_idx;
    indices[base_idx + 0u] = absolute_vertex_base + 0u;
    indices[base_idx + 1u] = absolute_vertex_base + 1u;
    indices[base_idx + 2u] = absolute_vertex_base + 2u;
    indices[base_idx + 3u] = absolute_vertex_base + 0u;
    indices[base_idx + 4u] = absolute_vertex_base + 2u;
    indices[base_idx + 5u] = absolute_vertex_base + 3u;
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

// TODO: Add morton encoding functions when needed
// #include "morton.wgsl"

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
    
    // For debugging: Just create a simple cube for each chunk
    // This ensures we have visible geometry
    if (local_id.x == 0u) {
        // Add a cube at chunk center
        let center = vec3<f32>(16.0, 16.0, 16.0);
        
        // Add all 6 faces of a cube
        for (var face = 0u; face < 6u; face = face + 1u) {
            add_face(request_idx, center, face, 1u); // Use stone (1u)
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