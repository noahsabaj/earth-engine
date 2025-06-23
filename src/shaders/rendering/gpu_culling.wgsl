// GPU Culling Compute Shader
// Performs frustum culling and generates indirect draw commands

struct CameraData {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding0: f32,
    frustum_planes: array<vec4<f32>, 6>,
};

struct DrawMetadata {
    bounding_sphere: vec4<f32>,  // xyz = center, w = radius
    lod_info: vec4<f32>,         // x = lod0_distance, y = lod1_distance, z = index_count, w = base_vertex
    material_id: u32,
    mesh_id: u32,
    instance_offset: u32,
    flags: u32,                  // bit 0 = visible, bit 2 = always visible
};

struct IndirectCommand {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
};

struct DrawCount {
    count: atomic<u32>,
};

struct CullingStats {
    total_tested: atomic<u32>,
    frustum_culled: atomic<u32>,
    distance_culled: atomic<u32>,
    drawn: atomic<u32>,
};

@group(0) @binding(0) var<uniform> camera: CameraData;
@group(0) @binding(1) var<storage, read> draw_metadata: array<DrawMetadata>;
@group(0) @binding(2) var<storage, read_write> indirect_commands: array<IndirectCommand>;
@group(0) @binding(3) var<storage, read_write> draw_count: DrawCount;
@group(0) @binding(4) var<storage, read_write> stats: CullingStats;

// Constants
const FLAG_VISIBLE: u32 = 1u;
const FLAG_SKIP_FRUSTUM: u32 = 2u;
const FLAG_ALWAYS_VISIBLE: u32 = 4u;
const FLAG_SHADOW_CASTER: u32 = 8u;

// Mesh constants from constants.rs buffer_layouts
const CUBE_INDEX_COUNT: u32 = 36u;

// Check if a sphere is inside the frustum
fn sphere_inside_frustum(center: vec3<f32>, radius: f32) -> bool {
    // Test against all 6 frustum planes
    for (var i = 0u; i < 6u; i = i + 1u) {
        let plane = camera.frustum_planes[i];
        let distance = dot(plane.xyz, center) + plane.w;
        
        // If sphere is completely outside this plane, cull it
        if (distance < -radius) {
            return false;
        }
    }
    return true;
}

@compute @workgroup_size(64)
fn cull_objects(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_objects = arrayLength(&draw_metadata);
    
    if (idx >= total_objects) {
        return;
    }
    
    atomicAdd(&stats.total_tested, 1u);
    
    let metadata = draw_metadata[idx];
    let center = metadata.bounding_sphere.xyz;
    let radius = metadata.bounding_sphere.w;
    
    // Check visibility flags
    let is_visible = (metadata.flags & FLAG_VISIBLE) != 0u;
    let skip_frustum = (metadata.flags & FLAG_SKIP_FRUSTUM) != 0u;
    let always_visible = (metadata.flags & FLAG_ALWAYS_VISIBLE) != 0u;
    
    if (!is_visible && !always_visible) {
        return; // Object is not visible
    }
    
    // Perform frustum culling unless skipped
    var passed_culling = true;
    if (!skip_frustum && !always_visible) {
        passed_culling = sphere_inside_frustum(center, radius);
        if (!passed_culling) {
            atomicAdd(&stats.frustum_culled, 1u);
            return;
        }
    }
    
    // Distance culling (optional)
    let distance_to_camera = length(camera.position - center);
    let max_distance = 500.0; // Max view distance
    
    if (distance_to_camera - radius > max_distance && !always_visible) {
        atomicAdd(&stats.distance_culled, 1u);
        return;
    }
    
    // Object passed all culling tests - add to draw list
    let draw_index = atomicAdd(&draw_count.count, 1u);
    
    // Write indirect draw command
    // Use actual mesh index count instead of hardcoded value
    var actual_index_count: u32;
    if u32(metadata.lod_info.z) > 0u {
        actual_index_count = u32(metadata.lod_info.z);
    } else {
        actual_index_count = CUBE_INDEX_COUNT;
    }
    
    indirect_commands[draw_index] = IndirectCommand(
        actual_index_count,          // index_count - use actual mesh index count
        1u,                          // instance_count - one instance per draw
        0u,                          // first_index
        0,                           // base_vertex
        metadata.instance_offset     // first_instance - use the instance offset
    );
    
    atomicAdd(&stats.drawn, 1u);
}

// Reset counters before culling
@compute @workgroup_size(1)
fn reset_counters() {
    atomicStore(&draw_count.count, 0u);
    atomicStore(&stats.total_tested, 0u);
    atomicStore(&stats.frustum_culled, 0u);
    atomicStore(&stats.distance_culled, 0u);
    atomicStore(&stats.drawn, 0u);
}