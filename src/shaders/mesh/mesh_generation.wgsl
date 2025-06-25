// GPU Mesh Generation Compute Shader
// Generates chunk meshes entirely on GPU with zero CPU involvement

// Constants
// CHUNK_SIZE is auto-generated from constants.rs
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
@group(0) @binding(5) var<storage, read_write> indirect_commands: array<u32>;
@group(0) @binding(6) var<uniform> params: MeshingParams;

// Shared memory for face culling
var<workgroup> voxel_cache: array<u32, 512>; // 8x8x8 with padding

// Get voxel from world data
fn get_voxel(world_pos: vec3<i32>) -> u32 {
    // For now, return a simple pattern to test if mesh generation works at all
    // TODO: Fix morton encoding for negative coordinates
    
    // Create a simple terrain pattern for testing
    // Adjusted for spawn at y=70 (camera_constants::DEFAULT_HEIGHT)
    // Block IDs from hearth-engine: AIR=0, GRASS=1, DIRT=2, STONE=3
    if (world_pos.y < 68) {
        // Below y=68, return stone
        return 3u; // STONE (BlockId(3))
    } else if (world_pos.y >= 68 && world_pos.y <= 69) {
        // At y=68-69, return grass (surface layer)
        return 1u; // GRASS (BlockId(1))
    } else {
        // Above y=69, return air
        return 0u; // AIR (BlockId(0))
    }
}

// Check if voxel is transparent
fn is_transparent(voxel: u32) -> bool {
    return voxel == 0u || voxel == 6u; // AIR (0) or WATER (6)
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
    // Indices need to include base_vertex_offset for proper indexing in shared buffer
    let base_idx = base_index_offset + index_idx;
    let absolute_vertex_base = base_vertex_offset + vertex_idx;
    indices[base_idx + 0u] = absolute_vertex_base + 0u;
    indices[base_idx + 1u] = absolute_vertex_base + 1u;
    indices[base_idx + 2u] = absolute_vertex_base + 2u;
    indices[base_idx + 3u] = absolute_vertex_base + 0u;
    indices[base_idx + 4u] = absolute_vertex_base + 2u;
    indices[base_idx + 5u] = absolute_vertex_base + 3u;
}

// Get color for voxel type
fn get_voxel_color(voxel_type: u32) -> vec3<f32> {
    // Block IDs from hearth-engine: AIR=0, GRASS=1, DIRT=2, STONE=3, WOOD=4, SAND=5, WATER=6, LEAVES=7
    switch (voxel_type) {
        case 1u: { return vec3<f32>(0.3, 0.7, 0.3); }  // GRASS (BlockId(1))
        case 2u: { return vec3<f32>(0.55, 0.4, 0.3); } // DIRT (BlockId(2))
        case 3u: { return vec3<f32>(0.5, 0.5, 0.5); }  // STONE (BlockId(3))
        case 4u: { return vec3<f32>(0.6, 0.5, 0.4); }  // WOOD (BlockId(4))
        case 5u: { return vec3<f32>(0.9, 0.8, 0.6); }  // SAND (BlockId(5))
        case 6u: { return vec3<f32>(0.2, 0.4, 0.8); }  // WATER (BlockId(6))
        case 7u: { return vec3<f32>(0.2, 0.6, 0.2); }  // LEAVES (BlockId(7))
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
    
    let request = requests[request_idx];
    let chunk_origin = vec3<i32>(request.chunk_pos) * i32(params.chunk_size);
    
    // Each thread processes one voxel
    let voxel_offset = vec3<i32>(local_id);
    
    // Only process voxels within chunk bounds
    if (voxel_offset.x < i32(params.chunk_size) &&
        voxel_offset.y < i32(params.chunk_size) &&
        voxel_offset.z < i32(params.chunk_size)) {
        
        let world_pos = chunk_origin + voxel_offset;
        let voxel = get_voxel(world_pos);
        
        // Skip air voxels
        if (!is_transparent(voxel)) {
            let local_pos = vec3<f32>(voxel_offset);
            
            // Check all 6 faces
            for (var face = 0u; face < 6u; face = face + 1u) {
                let normal = compute_face_normal(face);
                let neighbor_pos = world_pos + vec3<i32>(normal);
                let neighbor = get_voxel(neighbor_pos);
                
                // Only add face if neighbor is transparent
                if (is_transparent(neighbor)) {
                    add_face(request_idx, local_pos, face, voxel);
                }
            }
        }
    }
    
    // Synchronize before writing final counts
    workgroupBarrier();
    
    // Thread 0 writes indirect command
    if (local_id.x == 0u && local_id.y == 0u && local_id.z == 0u) {
        let vertex_count = atomicLoad(&metadata[request_idx].vertex_count);
        let index_count = atomicLoad(&metadata[request_idx].index_count);
        
        // For debugging: If no geometry was generated, create a simple cube
        if (index_count == 0u) {
            // Add a debug cube at chunk origin
            add_face(request_idx, vec3<f32>(25.0, 64.0, 25.0), 0u, 1u); // +X face
            add_face(request_idx, vec3<f32>(25.0, 64.0, 25.0), 1u, 1u); // -X face
            add_face(request_idx, vec3<f32>(25.0, 64.0, 25.0), 2u, 1u); // +Y face
            add_face(request_idx, vec3<f32>(25.0, 64.0, 25.0), 3u, 1u); // -Y face
            add_face(request_idx, vec3<f32>(25.0, 64.0, 25.0), 4u, 1u); // +Z face
            add_face(request_idx, vec3<f32>(25.0, 64.0, 25.0), 5u, 1u); // -Z face
        }
        
        // Re-read counts after potential debug cube addition
        let final_vertex_count = atomicLoad(&metadata[request_idx].vertex_count);
        let final_index_count = atomicLoad(&metadata[request_idx].index_count);
        
        // Write indirect draw indexed command
        // Format for DrawIndexedIndirect requires 5 u32 values:
        // [0] index_count
        // [1] instance_count  
        // [2] first_index
        // [3] base_vertex (signed i32 as u32)
        // [4] first_instance
        
        // For now, use a conservative but reasonable index count
        // TODO: Implement proper accumulation of indices across all chunks
        if (request_idx == 0u) {
            // Use 10000 indices as a conservative estimate that should show terrain
            // without causing overdraw issues
            indirect_commands[0] = 10000u;                  // index_count (conservative)
            indirect_commands[1] = 1u;                      // instance_count 
            indirect_commands[2] = 0u;                      // first_index
            indirect_commands[3] = 0u;                      // base_vertex  
            indirect_commands[4] = 0u;                      // first_instance
        }
    }
}