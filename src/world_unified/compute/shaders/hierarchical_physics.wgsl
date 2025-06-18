// Hierarchical Physics Queries
// GPU-accelerated physics using octree and BVH

struct VoxelData {
    packed: u32,
}

struct OctreeNode {
    children: array<u32, 8>,
    metadata: u32,
    bbox_min: vec3<f32>,
    bbox_max: vec3<f32>,
}

struct BvhNode {
    aabb_min: vec3<f32>,
    left_first: u32,
    aabb_max: vec3<f32>,
    prim_count: u32,
}

struct PhysicsQuery {
    query_type: u32,
    origin: vec3<f32>,
    max_distance: f32,
    direction: vec3<f32>,
    radius: f32,
    half_extents: vec3<f32>,
    flags: u32,
}

struct QueryResult {
    hit_distance: f32,
    hit_position: vec3<f32>,
    hit_normal: vec3<f32>,
    block_id: u32,
    chunk_index: u32,
    padding: array<u32, 3>,
}

@group(0) @binding(0) var<storage, read> world_voxels: array<VoxelData>;
@group(0) @binding(1) var<storage, read> octree_nodes: array<OctreeNode>;
@group(0) @binding(2) var<storage, read> bvh_nodes: array<BvhNode>;
@group(0) @binding(3) var<storage, read> queries: array<PhysicsQuery>;
@group(0) @binding(4) var<storage, read_write> results: array<QueryResult>;

const CHUNK_SIZE: u32 = 32u;
const EPSILON: f32 = 0.0001;

// DDA voxel traversal for ray casting
struct DDAState {
    pos: vec3<i32>,
    t_max: vec3<f32>,
    t_delta: vec3<f32>,
    step: vec3<i32>,
    normal: vec3<f32>,
}

fn init_dda(origin: vec3<f32>, direction: vec3<f32>) -> DDAState {
    let pos = vec3<i32>(floor(origin));
    let step = vec3<i32>(sign(direction));
    
    let t_delta = abs(1.0 / direction);
    
    var t_max: vec3<f32>;
    if direction.x > 0.0 {
        t_max.x = (f32(pos.x + 1) - origin.x) / direction.x;
    } else {
        t_max.x = (origin.x - f32(pos.x)) / -direction.x;
    }
    
    if direction.y > 0.0 {
        t_max.y = (f32(pos.y + 1) - origin.y) / direction.y;
    } else {
        t_max.y = (origin.y - f32(pos.y)) / -direction.y;
    }
    
    if direction.z > 0.0 {
        t_max.z = (f32(pos.z + 1) - origin.z) / direction.z;
    } else {
        t_max.z = (origin.z - f32(pos.z)) / -direction.z;
    }
    
    return DDAState(pos, t_max, t_delta, step, vec3(0.0));
}

fn step_dda(state: ptr<function, DDAState>) -> f32 {
    var t_min = min(min((*state).t_max.x, (*state).t_max.y), (*state).t_max.z);
    
    if (*state).t_max.x <= t_min {
        (*state).pos.x += (*state).step.x;
        (*state).t_max.x += (*state).t_delta.x;
        (*state).normal = vec3(-f32((*state).step.x), 0.0, 0.0);
    } else if (*state).t_max.y <= t_min {
        (*state).pos.y += (*state).step.y;
        (*state).t_max.y += (*state).t_delta.y;
        (*state).normal = vec3(0.0, -f32((*state).step.y), 0.0);
    } else {
        (*state).pos.z += (*state).step.z;
        (*state).t_max.z += (*state).t_delta.z;
        (*state).normal = vec3(0.0, 0.0, -f32((*state).step.z));
    }
    
    return t_min;
}

// Get voxel using Morton encoding
fn get_voxel(world_pos: vec3<i32>) -> VoxelData {
    let chunk_pos = world_pos / i32(CHUNK_SIZE);
    let local_pos = world_pos % i32(CHUNK_SIZE);
    
    let chunk_morton = morton_encode_3d(u32(chunk_pos.x), u32(chunk_pos.y), u32(chunk_pos.z));
    let voxel_morton = morton_encode_3d(u32(local_pos.x), u32(local_pos.y), u32(local_pos.z));
    
    let global_index = chunk_morton * (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) + voxel_morton;
    
    if global_index < arrayLength(&world_voxels) {
        return world_voxels[global_index];
    }
    
    return VoxelData(0u);
}

fn morton_encode_3d(x: u32, y: u32, z: u32) -> u32 {
    var xx = x & 0x3FFu;
    var yy = y & 0x3FFu;
    var zz = z & 0x3FFu;
    
    xx = (xx | (xx << 16u)) & 0x30000FFu;
    xx = (xx | (xx << 8u)) & 0x300F00Fu;
    xx = (xx | (xx << 4u)) & 0x30C30C3u;
    xx = (xx | (xx << 2u)) & 0x9249249u;
    
    yy = (yy | (yy << 16u)) & 0x30000FFu;
    yy = (yy | (yy << 8u)) & 0x300F00Fu;
    yy = (yy | (yy << 4u)) & 0x30C30C3u;
    yy = (yy | (yy << 2u)) & 0x9249249u;
    
    zz = (zz | (zz << 16u)) & 0x30000FFu;
    zz = (zz | (zz << 8u)) & 0x300F00Fu;
    zz = (zz | (zz << 4u)) & 0x30C30C3u;
    zz = (zz | (zz << 2u)) & 0x9249249u;
    
    return xx | (yy << 1u) | (zz << 2u);
}

// Ray-AABB intersection
fn ray_aabb_intersect(
    ray_origin: vec3<f32>,
    ray_inv_dir: vec3<f32>,
    aabb_min: vec3<f32>,
    aabb_max: vec3<f32>,
) -> vec2<f32> {
    let t1 = (aabb_min - ray_origin) * ray_inv_dir;
    let t2 = (aabb_max - ray_origin) * ray_inv_dir;
    
    let tmin = max(max(min(t1.x, t2.x), min(t1.y, t2.y)), min(t1.z, t2.z));
    let tmax = min(min(max(t1.x, t2.x), max(t1.y, t2.y)), max(t1.z, t2.z));
    
    return vec2(tmin, tmax);
}

// Traverse octree for ray intersection
fn octree_traverse(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    ray_inv_dir: vec3<f32>,
    max_distance: f32,
) -> bool {
    var stack: array<u32, 32>;
    var stack_ptr = 0u;
    
    // Start with root
    stack[0] = 0u;
    stack_ptr = 1u;
    
    while stack_ptr > 0u {
        stack_ptr -= 1u;
        let node_idx = stack[stack_ptr];
        
        if node_idx >= arrayLength(&octree_nodes) {
            continue;
        }
        
        let node = octree_nodes[node_idx];
        
        // Ray-AABB test
        let t = ray_aabb_intersect(ray_origin, ray_inv_dir, node.bbox_min, node.bbox_max);
        
        if t.y < 0.0 || t.x > t.y || t.x > max_distance {
            continue;
        }
        
        // Check if leaf
        if (node.metadata & 0xFFu) == 0u {
            // Leaf node - check occupancy
            if (node.metadata & 0xFF00u) != 0u {
                return true;
            }
        } else {
            // Internal node - push children (unrolled for WGSL compatibility)
            if node.children[0] != 0u && stack_ptr < 32u {
                stack[stack_ptr] = node.children[0];
                stack_ptr += 1u;
            }
            if node.children[1] != 0u && stack_ptr < 32u {
                stack[stack_ptr] = node.children[1];
                stack_ptr += 1u;
            }
            if node.children[2] != 0u && stack_ptr < 32u {
                stack[stack_ptr] = node.children[2];
                stack_ptr += 1u;
            }
            if node.children[3] != 0u && stack_ptr < 32u {
                stack[stack_ptr] = node.children[3];
                stack_ptr += 1u;
            }
            if node.children[4] != 0u && stack_ptr < 32u {
                stack[stack_ptr] = node.children[4];
                stack_ptr += 1u;
            }
            if node.children[5] != 0u && stack_ptr < 32u {
                stack[stack_ptr] = node.children[5];
                stack_ptr += 1u;
            }
            if node.children[6] != 0u && stack_ptr < 32u {
                stack[stack_ptr] = node.children[6];
                stack_ptr += 1u;
            }
            if node.children[7] != 0u && stack_ptr < 32u {
                stack[stack_ptr] = node.children[7];
                stack_ptr += 1u;
            }
        }
    }
    
    return false;
}

@compute @workgroup_size(64, 1, 1)
fn raycast_query(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let query_id = global_id.x;
    
    if query_id >= arrayLength(&queries) {
        return;
    }
    
    let query = queries[query_id];
    
    // Skip non-raycast queries
    if query.query_type != 0u {
        return;
    }
    
    var result: QueryResult;
    result.hit_distance = -1.0;
    result.block_id = 0u;
    
    // Early rejection using octree
    let ray_inv_dir = 1.0 / query.direction;
    if !octree_traverse(query.origin, query.direction, ray_inv_dir, query.max_distance) {
        results[query_id] = result;
        return;
    }
    
    // DDA traversal for exact hit
    var dda = init_dda(query.origin, query.direction);
    var distance = 0.0;
    
    while distance < query.max_distance {
        let voxel = get_voxel(dda.pos);
        let block_id = voxel.packed & 0xFFFFu;
        
        if block_id != 0u {
            // Hit!
            result.hit_distance = distance;
            result.hit_position = query.origin + query.direction * distance;
            result.hit_normal = dda.normal;
            result.block_id = block_id;
            result.chunk_index = u32(dda.pos.x / i32(CHUNK_SIZE) + 
                                   dda.pos.y / i32(CHUNK_SIZE) * 16 + 
                                   dda.pos.z / i32(CHUNK_SIZE) * 256);
            break;
        }
        
        distance = step_dda(&dda);
    }
    
    results[query_id] = result;
}

@compute @workgroup_size(64, 1, 1)
fn spherecast_query(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let query_id = global_id.x;
    
    if query_id >= arrayLength(&queries) || queries[query_id].query_type != 1u {
        return;
    }
    
    let query = queries[query_id];
    var result: QueryResult;
    result.hit_distance = -1.0;
    
    // Sphere cast using swept sphere algorithm
    let radius = query.radius;
    let origin = query.origin;
    let direction = query.direction;
    
    // Check voxels in a box around the swept sphere
    let sweep_min = min(origin, origin + direction * query.max_distance) - vec3(radius);
    let sweep_max = max(origin, origin + direction * query.max_distance) + vec3(radius);
    
    let min_voxel = vec3<i32>(floor(sweep_min));
    let max_voxel = vec3<i32>(ceil(sweep_max));
    
    var closest_distance = query.max_distance;
    var hit_found = false;
    
    for (var y = min_voxel.y; y <= max_voxel.y; y++) {
        for (var z = min_voxel.z; z <= max_voxel.z; z++) {
            for (var x = min_voxel.x; x <= max_voxel.x; x++) {
                let voxel_pos = vec3<i32>(x, y, z);
                let voxel = get_voxel(voxel_pos);
                
                if (voxel.packed & 0xFFFFu) != 0u {
                    // Ray-box intersection for swept sphere
                    let box_min = vec3<f32>(voxel_pos);
                    let box_max = box_min + vec3(1.0);
                    
                    // Expand box by sphere radius
                    let expanded_min = box_min - vec3(radius);
                    let expanded_max = box_max + vec3(radius);
                    
                    let t = ray_aabb_intersect(origin, 1.0 / direction, expanded_min, expanded_max);
                    
                    if t.y >= 0.0 && t.x <= t.y && t.x < closest_distance {
                        closest_distance = max(t.x, 0.0);
                        result.hit_distance = closest_distance;
                        result.hit_position = origin + direction * closest_distance;
                        result.block_id = voxel.packed & 0xFFFFu;
                        hit_found = true;
                    }
                }
            }
        }
    }
    
    if hit_found {
        // Calculate normal (simplified)
        let hit_voxel = vec3<i32>(floor(result.hit_position));
        let local_pos = result.hit_position - vec3<f32>(hit_voxel) - vec3(0.5);
        let abs_pos = abs(local_pos);
        
        if abs_pos.x > abs_pos.y && abs_pos.x > abs_pos.z {
            result.hit_normal = vec3(sign(local_pos.x), 0.0, 0.0);
        } else if abs_pos.y > abs_pos.z {
            result.hit_normal = vec3(0.0, sign(local_pos.y), 0.0);
        } else {
            result.hit_normal = vec3(0.0, 0.0, sign(local_pos.z));
        }
    }
    
    results[query_id] = result;
}

@compute @workgroup_size(64, 1, 1)
fn boxcast_query(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let query_id = global_id.x;
    
    if query_id >= arrayLength(&queries) || queries[query_id].query_type != 2u {
        return;
    }
    
    let query = queries[query_id];
    var result: QueryResult;
    result.hit_distance = -1.0;
    
    // Box cast - similar to sphere cast but with OBB
    // Simplified implementation - would need full OBB-voxel intersection
    
    results[query_id] = result;
}

@compute @workgroup_size(64, 1, 1)
fn overlap_query(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let query_id = global_id.x;
    
    if query_id >= arrayLength(&queries) || queries[query_id].query_type != 4u {
        return;
    }
    
    let query = queries[query_id];
    var result: QueryResult;
    result.hit_distance = -1.0;
    
    // Check for overlaps in a box region
    let min_pos = vec3<i32>(floor(query.origin - query.half_extents));
    let max_pos = vec3<i32>(ceil(query.origin + query.half_extents));
    
    for (var y = min_pos.y; y <= max_pos.y; y++) {
        for (var z = min_pos.z; z <= max_pos.z; z++) {
            for (var x = min_pos.x; x <= max_pos.x; x++) {
                let voxel = get_voxel(vec3<i32>(x, y, z));
                if (voxel.packed & 0xFFFFu) != 0u {
                    result.hit_distance = 0.0;
                    result.hit_position = query.origin;
                    result.block_id = voxel.packed & 0xFFFFu;
                    results[query_id] = result;
                    return;
                }
            }
        }
    }
    
    results[query_id] = result;
}