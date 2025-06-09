// Chunk compute shader for GPU-based chunk operations

struct ChunkMetadata {
    position: vec4<i32>,  // xyz + padding
    size: u32,
    block_count: u32,
    flags: u32,
    _padding: u32,
}

struct MeshOutput {
    vertex_count: atomic<u32>,
    index_count: atomic<u32>,
    // Followed by vertex and index data
}

@group(0) @binding(0) var<uniform> metadata: ChunkMetadata;
@group(0) @binding(1) var<storage, read> blocks: array<u32>;
@group(0) @binding(2) var<storage, read> light_data: array<u32>;

@group(1) @binding(0) var<storage, read_write> output: MeshOutput;
@group(1) @binding(1) var<storage, read_write> vertices: array<f32>;
@group(1) @binding(2) var<storage, read_write> indices: array<u32>;

// Block type constants
const BLOCK_AIR: u32 = 0u;
const BLOCK_STONE: u32 = 1u;
const BLOCK_DIRT: u32 = 2u;
const BLOCK_GRASS: u32 = 3u;

// Face directions
const FACE_RIGHT: u32 = 0u;
const FACE_LEFT: u32 = 1u;
const FACE_TOP: u32 = 2u;
const FACE_BOTTOM: u32 = 3u;
const FACE_FRONT: u32 = 4u;
const FACE_BACK: u32 = 5u;

// Vertex offsets for cube faces
const FACE_VERTICES: array<array<vec3<f32>, 4>, 6> = array<array<vec3<f32>, 4>, 6>(
    // Right face (+X)
    array<vec3<f32>, 4>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(1.0, 1.0, 0.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(1.0, 0.0, 1.0)
    ),
    // Left face (-X)
    array<vec3<f32>, 4>(
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(0.0, 1.0, 1.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0)
    ),
    // Top face (+Y)
    array<vec3<f32>, 4>(
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 1.0, 1.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(1.0, 1.0, 0.0)
    ),
    // Bottom face (-Y)
    array<vec3<f32>, 4>(
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(1.0, 0.0, 1.0)
    ),
    // Front face (+Z)
    array<vec3<f32>, 4>(
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(1.0, 0.0, 1.0),
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(0.0, 1.0, 1.0)
    ),
    // Back face (-Z)
    array<vec3<f32>, 4>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(1.0, 1.0, 0.0)
    )
);

// Face normals
const FACE_NORMALS: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>(1.0, 0.0, 0.0),   // Right
    vec3<f32>(-1.0, 0.0, 0.0),  // Left
    vec3<f32>(0.0, 1.0, 0.0),   // Top
    vec3<f32>(0.0, -1.0, 0.0),  // Bottom
    vec3<f32>(0.0, 0.0, 1.0),   // Front
    vec3<f32>(0.0, 0.0, -1.0)   // Back
);

// Get block at position
fn get_block(x: i32, y: i32, z: i32) -> u32 {
    if (x < 0 || y < 0 || z < 0 || 
        x >= i32(metadata.size) || 
        y >= i32(metadata.size) || 
        z >= i32(metadata.size)) {
        return BLOCK_AIR;
    }
    
    let index = u32(x + y * i32(metadata.size) + z * i32(metadata.size) * i32(metadata.size));
    return blocks[index];
}

// Check if face should be rendered
fn should_render_face(x: i32, y: i32, z: i32, face: u32) -> bool {
    let current_block = get_block(x, y, z);
    if (current_block == BLOCK_AIR) {
        return false;
    }
    
    var neighbor_x = x;
    var neighbor_y = y;
    var neighbor_z = z;
    
    switch face {
        case FACE_RIGHT: { neighbor_x += 1; }
        case FACE_LEFT: { neighbor_x -= 1; }
        case FACE_TOP: { neighbor_y += 1; }
        case FACE_BOTTOM: { neighbor_y -= 1; }
        case FACE_FRONT: { neighbor_z += 1; }
        case FACE_BACK: { neighbor_z -= 1; }
        default: {}
    }
    
    let neighbor_block = get_block(neighbor_x, neighbor_y, neighbor_z);
    return neighbor_block == BLOCK_AIR;
}

// Get block color based on type
fn get_block_color(block_type: u32) -> vec3<f32> {
    switch block_type {
        case BLOCK_STONE: { return vec3<f32>(0.5, 0.5, 0.5); }
        case BLOCK_DIRT: { return vec3<f32>(0.4, 0.3, 0.2); }
        case BLOCK_GRASS: { return vec3<f32>(0.2, 0.6, 0.2); }
        default: { return vec3<f32>(1.0, 0.0, 1.0); } // Magenta for unknown
    }
}

@compute @workgroup_size(8, 8, 8)
fn generate_mesh(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let z = i32(global_id.z);
    
    if (x >= i32(metadata.size) || y >= i32(metadata.size) || z >= i32(metadata.size)) {
        return;
    }
    
    let block_type = get_block(x, y, z);
    if (block_type == BLOCK_AIR) {
        return;
    }
    
    let block_color = get_block_color(block_type);
    
    // Check each face
    for (var face = 0u; face < 6u; face++) {
        if (!should_render_face(x, y, z, face)) {
            continue;
        }
        
        // Add vertices for this face
        let base_vertex = atomicAdd(&output.vertex_count, 4u);
        let base_index = atomicAdd(&output.index_count, 6u);
        
        let face_normal = FACE_NORMALS[face];
        let world_pos = vec3<f32>(f32(x), f32(y), f32(z));
        
        // Add 4 vertices for the face
        for (var v = 0u; v < 4u; v++) {
            let vertex_pos = world_pos + FACE_VERTICES[face][v];
            let vertex_idx = (base_vertex + v) * 9u; // 9 floats per vertex
            
            // Position (3 floats)
            vertices[vertex_idx + 0u] = vertex_pos.x;
            vertices[vertex_idx + 1u] = vertex_pos.y;
            vertices[vertex_idx + 2u] = vertex_pos.z;
            
            // Color (3 floats)
            vertices[vertex_idx + 3u] = block_color.x;
            vertices[vertex_idx + 4u] = block_color.y;
            vertices[vertex_idx + 5u] = block_color.z;
            
            // Normal (3 floats)
            vertices[vertex_idx + 6u] = face_normal.x;
            vertices[vertex_idx + 7u] = face_normal.y;
            vertices[vertex_idx + 8u] = face_normal.z;
        }
        
        // Add 6 indices for 2 triangles
        indices[base_index + 0u] = base_vertex + 0u;
        indices[base_index + 1u] = base_vertex + 1u;
        indices[base_index + 2u] = base_vertex + 2u;
        indices[base_index + 3u] = base_vertex + 0u;
        indices[base_index + 4u] = base_vertex + 2u;
        indices[base_index + 5u] = base_vertex + 3u;
    }
}

// Entry point for counting visible faces (first pass)
@compute @workgroup_size(8, 8, 8)
fn count_faces(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let z = i32(global_id.z);
    
    if (x >= i32(metadata.size) || y >= i32(metadata.size) || z >= i32(metadata.size)) {
        return;
    }
    
    let block_type = get_block(x, y, z);
    if (block_type == BLOCK_AIR) {
        return;
    }
    
    // Count visible faces
    for (var face = 0u; face < 6u; face++) {
        if (should_render_face(x, y, z, face)) {
            atomicAdd(&output.vertex_count, 4u);
            atomicAdd(&output.index_count, 6u);
        }
    }
}