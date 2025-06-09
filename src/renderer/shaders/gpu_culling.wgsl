// GPU-driven culling compute shader
// Performs frustum culling and LOD selection on the GPU

struct CameraData {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding0: f32,
    frustum_planes: array<vec4<f32>, 6>, // 6 frustum planes
}

struct DrawMetadata {
    bounding_sphere: vec4<f32>, // xyz = center, w = radius
    lod_info: vec4<f32>,        // x = min dist, y = max dist, z = LOD level
    material_id: u32,
    mesh_id: u32,
    instance_offset: u32,
    flags: u32,
}

struct IndirectCommand {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}

struct DrawCount {
    count: atomic<u32>,
}

struct CullingStats {
    total_tested: atomic<u32>,
    frustum_culled: atomic<u32>,
    distance_culled: atomic<u32>,
    drawn: atomic<u32>,
}

// Bindings
@group(0) @binding(0) var<uniform> camera: CameraData;
@group(0) @binding(1) var<storage, read> draw_metadata: array<DrawMetadata>;
@group(0) @binding(2) var<storage, read_write> indirect_commands: array<IndirectCommand>;
@group(0) @binding(3) var<storage, read_write> draw_count: DrawCount;
@group(0) @binding(4) var<storage, read_write> culling_stats: CullingStats;

// Constants
const FRUSTUM_PLANE_COUNT: u32 = 6u;
const FLAG_VISIBLE: u32 = 1u;
const FLAG_CAST_SHADOWS: u32 = 2u;
const FLAG_ALWAYS_VISIBLE: u32 = 4u;
const MAX_DRAW_DISTANCE: f32 = 1000.0;

// Check if sphere is inside frustum
fn sphere_inside_frustum(center: vec3<f32>, radius: f32) -> bool {
    for (var i: u32 = 0u; i < FRUSTUM_PLANE_COUNT; i = i + 1u) {
        let plane = camera.frustum_planes[i];
        let distance = dot(plane.xyz, center) + plane.w;
        
        if (distance < -radius) {
            return false;
        }
    }
    return true;
}

// Calculate LOD based on distance
fn calculate_lod(distance: f32, lod_info: vec4<f32>) -> u32 {
    if (distance < lod_info.x) {
        return 0u; // Highest detail
    } else if (distance < lod_info.y) {
        return 1u; // Medium detail
    } else {
        return 2u; // Low detail
    }
}

@compute @workgroup_size(64, 1, 1)
fn cull_instances(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let instance_index = global_id.x;
    let metadata_count = arrayLength(&draw_metadata);
    
    // Bounds check
    if (instance_index >= metadata_count) {
        return;
    }
    
    // Increment total tested
    atomicAdd(&culling_stats.total_tested, 1u);
    
    // Get metadata for this draw
    let metadata = draw_metadata[instance_index];
    
    // Check if always visible flag is set
    if ((metadata.flags & FLAG_ALWAYS_VISIBLE) != 0u) {
        // Always draw this instance
        let draw_index = atomicAdd(&draw_count.count, 1u);
        indirect_commands[draw_index] = IndirectCommand(
            metadata.lod_info.z,  // Use LOD as index count (simplified)
            1u,                   // One instance
            0u,                   // First index
            0,                    // Base vertex
            metadata.instance_offset
        );
        atomicAdd(&culling_stats.drawn, 1u);
        return;
    }
    
    // Extract bounding sphere
    let center = metadata.bounding_sphere.xyz;
    let radius = metadata.bounding_sphere.w;
    
    // Distance from camera
    let distance = length(camera.position - center);
    
    // Distance culling
    if (distance - radius > MAX_DRAW_DISTANCE) {
        atomicAdd(&culling_stats.distance_culled, 1u);
        return;
    }
    
    // Frustum culling
    if (!sphere_inside_frustum(center, radius)) {
        atomicAdd(&culling_stats.frustum_culled, 1u);
        return;
    }
    
    // Calculate LOD
    let lod = calculate_lod(distance, metadata.lod_info);
    
    // Get next available slot in command buffer
    let draw_index = atomicAdd(&draw_count.count, 1u);
    
    // Write indirect draw command
    // In a real implementation, we'd look up actual mesh data
    let index_count = select(1000u, select(500u, 200u, lod == 2u), lod == 1u);
    
    indirect_commands[draw_index] = IndirectCommand(
        index_count,              // Index count based on LOD
        1u,                      // One instance
        0u,                      // First index
        0,                       // Base vertex
        metadata.instance_offset // Instance offset
    );
    
    atomicAdd(&culling_stats.drawn, 1u);
}

// Reset counters before culling
@compute @workgroup_size(1, 1, 1)
fn reset_counters() {
    atomicStore(&draw_count.count, 0u);
    atomicStore(&culling_stats.total_tested, 0u);
    atomicStore(&culling_stats.frustum_culled, 0u);
    atomicStore(&culling_stats.distance_culled, 0u);
    atomicStore(&culling_stats.drawn, 0u);
}