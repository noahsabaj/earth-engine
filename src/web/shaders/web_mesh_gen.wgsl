// Web-optimized mesh generation shader
// Generates vertices and indices directly on GPU from voxel data

struct VoxelData {
    packed: u32,
}

struct ChunkMetadata {
    chunk_pos: vec3<u32>,
    flags: u32,
}

struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    ao: f32,
    light_level: f32,
}

struct IndirectCommand {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

@group(0) @binding(0) var<storage, read> voxel_buffer: array<VoxelData>;
@group(0) @binding(1) var<storage, read> metadata_buffer: array<ChunkMetadata>;

@group(1) @binding(0) var<storage, read_write> vertex_buffer: array<Vertex>;
@group(1) @binding(1) var<storage, read_write> index_buffer: array<u32>;
@group(1) @binding(2) var<storage, read_write> indirect_buffer: array<IndirectCommand>;

const CHUNK_SIZE: u32 = 32u;
const VOXELS_PER_CHUNK: u32 = 32768u; // 32^3

// Unpack voxel data
fn unpack_voxel(packed: u32) -> vec4<u32> {
    let block_id = packed & 0xFFFFu;
    let light = (packed >> 16u) & 0xFu;
    let sky_light = (packed >> 20u) & 0xFu;
    let metadata = (packed >> 24u) & 0xFFu;
    return vec4<u32>(block_id, light, sky_light, metadata);
}

// Get voxel at position
fn get_voxel(chunk_idx: u32, local_pos: vec3<u32>) -> VoxelData {
    let idx = local_pos.x + local_pos.y * CHUNK_SIZE + local_pos.z * CHUNK_SIZE * CHUNK_SIZE;
    let global_idx = chunk_idx * VOXELS_PER_CHUNK + idx;
    return voxel_buffer[global_idx];
}

// Check if block is solid
fn is_solid(block_id: u32) -> bool {
    return block_id != 0u; // 0 = air
}

// Face vertices for cube
const FACE_VERTICES: array<vec3<f32>, 24> = array<vec3<f32>, 24>(
    // Front face
    vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 1.0),
    vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.0, 1.0, 1.0),
    // Back face
    vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 1.0, 0.0),
    // Top face
    vec3<f32>(0.0, 1.0, 1.0), vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(0.0, 1.0, 0.0),
    // Bottom face
    vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(1.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, 1.0),
    // Right face
    vec3<f32>(1.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(1.0, 1.0, 0.0), vec3<f32>(1.0, 1.0, 1.0),
    // Left face
    vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0),
    vec3<f32>(0.0, 1.0, 1.0), vec3<f32>(0.0, 1.0, 0.0)
);

// Face normals
const FACE_NORMALS: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
    vec3<f32>(0.0, 0.0, 1.0),   // Front
    vec3<f32>(0.0, 0.0, -1.0),  // Back
    vec3<f32>(0.0, 1.0, 0.0),   // Top
    vec3<f32>(0.0, -1.0, 0.0),  // Bottom
    vec3<f32>(1.0, 0.0, 0.0),   // Right
    vec3<f32>(-1.0, 0.0, 0.0)   // Left
);

// Face indices
const FACE_INDICES: array<u32, 6> = array<u32, 6>(0u, 1u, 2u, 2u, 3u, 0u);

// Calculate ambient occlusion
fn calculate_ao(chunk_idx: u32, pos: vec3<u32>, normal: vec3<f32>, corner: vec3<f32>) -> f32 {
    // Simplified AO calculation
    var occlusion = 0u;
    let sample_pos = vec3<i32>(pos) + vec3<i32>(normal + corner);
    
    if (sample_pos.x >= 0 && sample_pos.x < i32(CHUNK_SIZE) &&
        sample_pos.y >= 0 && sample_pos.y < i32(CHUNK_SIZE) &&
        sample_pos.z >= 0 && sample_pos.z < i32(CHUNK_SIZE)) {
        let voxel = get_voxel(chunk_idx, vec3<u32>(sample_pos));
        let data = unpack_voxel(voxel.packed);
        if (is_solid(data.x)) {
            occlusion = 1u;
        }
    }
    
    return 1.0 - f32(occlusion) * 0.25;
}

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let chunk_idx = global_id.x + global_id.y * 8u + global_id.z * 64u;
    
    // Get chunk metadata
    let metadata = metadata_buffer[chunk_idx];
    
    // Skip empty chunks
    if ((metadata.flags & 1u) == 0u) {
        return;
    }
    
    var vertex_offset = chunk_idx * 65536u; // Max vertices per chunk
    var index_offset = chunk_idx * 98304u;  // Max indices per chunk
    var vertex_count = 0u;
    var index_count = 0u;
    
    // Generate mesh for each voxel
    for (var z = 0u; z < CHUNK_SIZE; z = z + 1u) {
        for (var y = 0u; y < CHUNK_SIZE; y = y + 1u) {
            for (var x = 0u; x < CHUNK_SIZE; x = x + 1u) {
                let pos = vec3<u32>(x, y, z);
                let voxel = get_voxel(chunk_idx, pos);
                let data = unpack_voxel(voxel.packed);
                
                if (!is_solid(data.x)) {
                    continue;
                }
                
                // Check each face
                for (var face = 0u; face < 6u; face = face + 1u) {
                    let normal = FACE_NORMALS[face];
                    let neighbor_pos = vec3<i32>(pos) + vec3<i32>(normal);
                    
                    // Check if face is visible
                    var visible = false;
                    if (neighbor_pos.x < 0 || neighbor_pos.x >= i32(CHUNK_SIZE) ||
                        neighbor_pos.y < 0 || neighbor_pos.y >= i32(CHUNK_SIZE) ||
                        neighbor_pos.z < 0 || neighbor_pos.z >= i32(CHUNK_SIZE)) {
                        visible = true;
                    } else {
                        let neighbor = get_voxel(chunk_idx, vec3<u32>(neighbor_pos));
                        let neighbor_data = unpack_voxel(neighbor.packed);
                        visible = !is_solid(neighbor_data.x);
                    }
                    
                    if (!visible) {
                        continue;
                    }
                    
                    // Add face vertices
                    let base_vertex = vertex_offset + vertex_count;
                    for (var v = 0u; v < 4u; v = v + 1u) {
                        let vertex_pos = FACE_VERTICES[face * 4u + v] + vec3<f32>(pos);
                        
                        var vertex: Vertex;
                        vertex.position = vertex_pos;
                        vertex.normal = normal;
                        vertex.uv = vec2<f32>(f32(v & 1u), f32(v >> 1u));
                        vertex.ao = calculate_ao(chunk_idx, pos, normal, FACE_VERTICES[face * 4u + v]);
                        vertex.light_level = f32(data.y) / 15.0;
                        
                        vertex_buffer[base_vertex + v] = vertex;
                    }
                    
                    // Add face indices
                    let base_index = index_offset + index_count;
                    for (var i = 0u; i < 6u; i = i + 1u) {
                        index_buffer[base_index + i] = base_vertex + FACE_INDICES[i];
                    }
                    
                    vertex_count = vertex_count + 4u;
                    index_count = index_count + 6u;
                }
            }
        }
    }
    
    // Write indirect command
    var cmd: IndirectCommand;
    cmd.vertex_count = index_count;
    cmd.instance_count = 1u;
    cmd.first_vertex = 0u;
    cmd.first_instance = chunk_idx;
    indirect_buffer[chunk_idx] = cmd;
}