// Voxel to SDF conversion shader

struct SdfValue {
    distance: f32,
    material: u32,
}

struct VoxelData {
    voxel_type: u32,
}

struct GenerationParams {
    chunk_offset: vec3<i32>,
    chunk_size: vec3<u32>,
    sdf_size: vec3<u32>,
    resolution: f32,
    _padding: u32,
}

struct SdfConstants {
    resolution_factor: f32,
    max_distance: f32,
    surface_threshold: f32,
    smoothing_factor: f32,
    voxel_size: f32,
}

@group(0) @binding(0) var<storage, read> voxels: array<VoxelData>;
@group(0) @binding(1) var<storage, read_write> sdf: array<SdfValue>;
@group(0) @binding(2) var<uniform> constants: SdfConstants;

var<push_constant> params: GenerationParams;

// Convert SDF grid position to voxel position
fn sdf_to_voxel_pos(sdf_pos: vec3<u32>) -> vec3<f32> {
    let sdf_margin = 4u;
    let voxel_pos = (vec3<f32>(sdf_pos) - f32(sdf_margin)) * constants.resolution_factor;
    return voxel_pos + vec3<f32>(params.chunk_offset);
}

// Sample voxel at position
fn sample_voxel(pos: vec3<f32>) -> u32 {
    let voxel_pos = vec3<u32>(floor(pos));
    
    // Check bounds
    if (any(voxel_pos >= params.chunk_size)) {
        return 0u; // Empty outside chunk
    }
    
    let idx = voxel_pos.x + voxel_pos.y * params.chunk_size.x + voxel_pos.z * params.chunk_size.x * params.chunk_size.y;
    if (idx < arrayLength(&voxels)) {
        return voxels[idx].voxel_type;
    }
    
    return 0u;
}

// Calculate distance to nearest voxel surface
fn calculate_voxel_distance(world_pos: vec3<f32>) -> f32 {
    let voxel_center = floor(world_pos) + vec3<f32>(0.5);
    let local_pos = world_pos - voxel_center;
    
    // Distance to voxel faces
    let face_dist = vec3<f32>(0.5) - abs(local_pos);
    let min_face_dist = min(min(face_dist.x, face_dist.y), face_dist.z);
    
    return min_face_dist;
}

@compute @workgroup_size(8, 8, 8)
fn voxel_to_sdf(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (any(global_id >= params.sdf_size)) {
        return;
    }
    
    // Get world position for this SDF cell
    let world_pos = sdf_to_voxel_pos(global_id);
    
    // Sample voxel at this position
    let voxel_type = sample_voxel(world_pos);
    
    // Initialize distance
    var min_distance = constants.max_distance;
    var nearest_material = voxel_type;
    
    if (voxel_type > 0u) {
        // Inside solid voxel
        min_distance = -calculate_voxel_distance(world_pos);
    } else {
        // Search nearby voxels for nearest surface
        let search_radius = 3;
        
        for (var dz = -search_radius; dz <= search_radius; dz++) {
            for (var dy = -search_radius; dy <= search_radius; dy++) {
                for (var dx = -search_radius; dx <= search_radius; dx++) {
                    let sample_pos = world_pos + vec3<f32>(f32(dx), f32(dy), f32(dz));
                    let sample_type = sample_voxel(sample_pos);
                    
                    if (sample_type > 0u) {
                        // Found solid voxel
                        let voxel_center = floor(sample_pos) + vec3<f32>(0.5);
                        let distance = distance(world_pos, voxel_center) - 0.5;
                        
                        if (distance < min_distance) {
                            min_distance = distance;
                            nearest_material = sample_type;
                        }
                    }
                }
            }
        }
    }
    
    // Store result
    let sdf_idx = global_id.x + global_id.y * params.sdf_size.x + global_id.z * params.sdf_size.x * params.sdf_size.y;
    sdf[sdf_idx].distance = min_distance;
    sdf[sdf_idx].material = nearest_material;
}