// Volume rendering shader for transparent fluids

struct FluidVoxel {
    packed_data: u32,
    velocity_x: f32,
    velocity_y: f32,
    velocity_z: f32,
    pressure: f32,
}

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

struct RenderParams {
    water_refraction: f32,
    water_opacity: f32,
    lava_opacity: f32,
    oil_opacity: f32,
    smoothing_factor: f32,
    foam_threshold: f32,
    reflection_strength: f32,
    _padding: f32,
}

@group(0) @binding(0) var<storage, read> fluid_buffer: array<FluidVoxel>;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<uniform> params: RenderParams;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen quad vertices
@vertex
fn volume_vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // Generate fullscreen triangle
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    
    output.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    output.uv = vec2<f32>(x, 1.0 - y);
    
    return output;
}

// Ray-volume intersection
fn ray_box_intersection(ray_origin: vec3<f32>, ray_dir: vec3<f32>, box_min: vec3<f32>, box_max: vec3<f32>) -> vec2<f32> {
    let inv_dir = 1.0 / ray_dir;
    let t1 = (box_min - ray_origin) * inv_dir;
    let t2 = (box_max - ray_origin) * inv_dir;
    
    let t_min = min(t1, t2);
    let t_max = max(t1, t2);
    
    let t_near = max(max(t_min.x, t_min.y), t_min.z);
    let t_far = min(min(t_max.x, t_max.y), t_max.z);
    
    return vec2<f32>(max(t_near, 0.0), t_far);
}

// Sample fluid at position
fn sample_fluid(pos: vec3<f32>, grid_size: vec3<u32>) -> FluidVoxel {
    let grid_pos = vec3<u32>(pos);
    
    if (all(grid_pos < grid_size)) {
        let idx = grid_pos.x + grid_pos.y * grid_size.x + grid_pos.z * grid_size.x * grid_size.y;
        if (idx < arrayLength(&fluid_buffer)) {
            return fluid_buffer[idx];
        }
    }
    
    // Return empty fluid
    var empty: FluidVoxel;
    empty.packed_data = 0u;
    empty.velocity_x = 0.0;
    empty.velocity_y = 0.0;
    empty.velocity_z = 0.0;
    empty.pressure = 0.0;
    return empty;
}

// Extract fluid properties
fn get_fluid_type(packed: u32) -> u32 {
    return packed & 0xFFu;
}

fn get_fluid_level(packed: u32) -> f32 {
    return f32((packed >> 8u) & 0xFFu) / 255.0;
}

fn get_fluid_temperature(packed: u32) -> f32 {
    return f32((packed >> 16u) & 0xFFu) / 255.0;
}

// Volume rendering
@fragment
fn volume_fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Ray setup
    let ndc = vec2<f32>(input.uv.x * 2.0 - 1.0, 1.0 - input.uv.y * 2.0);
    let ray_clip = vec4<f32>(ndc, -1.0, 1.0);
    let ray_view = camera.view_proj * ray_clip;
    let ray_dir = normalize((ray_view.xyz / ray_view.w) - camera.position);
    
    // Volume bounds
    let grid_size = vec3<u32>(128u, 64u, 128u);
    let volume_min = vec3<f32>(0.0);
    let volume_max = vec3<f32>(grid_size);
    
    // Ray-volume intersection
    let t_range = ray_box_intersection(camera.position, ray_dir, volume_min, volume_max);
    
    if (t_range.x >= t_range.y) {
        discard;
    }
    
    // Ray marching parameters
    let num_samples = 64u;
    let step_size = (t_range.y - t_range.x) / f32(num_samples);
    
    // Accumulate color and opacity
    var accumulated_color = vec3<f32>(0.0);
    var accumulated_opacity = 0.0;
    
    // March through volume
    for (var i = 0u; i < num_samples; i++) {
        let t = t_range.x + f32(i) * step_size;
        let sample_pos = camera.position + ray_dir * t;
        
        // Sample fluid
        let fluid = sample_fluid(sample_pos, grid_size);
        let fluid_type = get_fluid_type(fluid.packed_data);
        let fluid_level = get_fluid_level(fluid.packed_data);
        
        if (fluid_type > 0u && fluid_level > 0.1) {
            // Get fluid color and opacity
            var sample_color = vec3<f32>(0.0);
            var sample_opacity = 0.0;
            
            switch fluid_type {
                case 1u: { // Water
                    sample_color = vec3<f32>(0.2, 0.5, 0.8);
                    sample_opacity = params.water_opacity * fluid_level * 0.1;
                    
                    // Add refraction distortion
                    let velocity = vec3<f32>(fluid.velocity_x, fluid.velocity_y, fluid.velocity_z);
                    sample_color += velocity * 0.1;
                }
                case 5u: { // Steam
                    sample_color = vec3<f32>(0.9, 0.9, 0.9);
                    sample_opacity = 0.05 * fluid_level;
                    
                    // Temperature-based color
                    let temp = get_fluid_temperature(fluid.packed_data);
                    sample_color *= 1.0 + temp * 0.5;
                }
                default: {}
            }
            
            // Accumulate with alpha blending
            let weight = sample_opacity * (1.0 - accumulated_opacity);
            accumulated_color += sample_color * weight;
            accumulated_opacity += weight;
            
            // Early exit if opaque
            if (accumulated_opacity > 0.95) {
                break;
            }
        }
    }
    
    return vec4<f32>(accumulated_color, accumulated_opacity);
}