// Fluid surface reconstruction and rendering shader

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
@group(0) @binding(3) var env_map: texture_cube<f32>;
@group(0) @binding(4) var env_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) fluid_type: f32,
    @location(3) velocity: vec3<f32>,
}

// Extract fluid data
fn get_fluid_type(packed: u32) -> u32 {
    return packed & 0xFFu;
}

fn get_fluid_level(packed: u32) -> f32 {
    return f32((packed >> 8u) & 0xFFu) / 255.0;
}

fn get_fluid_temperature(packed: u32) -> f32 {
    return f32((packed >> 16u) & 0xFFu) / 255.0;
}

// Marching cubes vertex generation
@vertex
fn surface_vertex(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // Generate vertices for marching cubes
    // This is a simplified version - full implementation would use lookup tables
    let cell_index = vertex_index / 36u; // 12 triangles * 3 vertices
    let local_vertex = vertex_index % 36u;
    
    // Calculate cell position
    let grid_size = vec3<u32>(128u, 64u, 128u); // Example size
    let cell_z = cell_index / (grid_size.x * grid_size.y);
    let cell_y = (cell_index % (grid_size.x * grid_size.y)) / grid_size.x;
    let cell_x = cell_index % grid_size.x;
    
    let cell_pos = vec3<f32>(f32(cell_x), f32(cell_y), f32(cell_z));
    
    // Sample fluid at cell corners
    let corner_indices = array<vec3<u32>, 8>(
        vec3<u32>(0u, 0u, 0u),
        vec3<u32>(1u, 0u, 0u),
        vec3<u32>(1u, 1u, 0u),
        vec3<u32>(0u, 1u, 0u),
        vec3<u32>(0u, 0u, 1u),
        vec3<u32>(1u, 0u, 1u),
        vec3<u32>(1u, 1u, 1u),
        vec3<u32>(0u, 1u, 1u)
    );
    
    // Calculate surface normal using gradient
    var normal = vec3<f32>(0.0);
    var fluid_type = 0u;
    var velocity = vec3<f32>(0.0);
    
    for (var i = 0u; i < 8u; i++) {
        let corner = cell_pos + vec3<f32>(corner_indices[i]);
        let idx = u32(corner.x + corner.y * f32(grid_size.x) + corner.z * f32(grid_size.x * grid_size.y));
        
        if (idx < arrayLength(&fluid_buffer)) {
            let fluid = fluid_buffer[idx];
            let level = get_fluid_level(fluid.packed_data);
            
            if (level > 0.5) {
                fluid_type = get_fluid_type(fluid.packed_data);
                velocity = vec3<f32>(fluid.velocity_x, fluid.velocity_y, fluid.velocity_z);
                
                // Contribute to normal calculation
                normal += vec3<f32>(corner_indices[i]) * level;
            }
        }
    }
    
    // Normalize and flip normal
    normal = normalize(normal - vec3<f32>(0.5));
    
    // Generate vertex position (simplified)
    let vertex_offset = f32(local_vertex) * 0.1;
    output.world_pos = cell_pos + vec3<f32>(vertex_offset);
    output.position = camera.view_proj * vec4<f32>(output.world_pos, 1.0);
    output.normal = normal;
    output.fluid_type = f32(fluid_type);
    output.velocity = velocity;
    
    return output;
}

// Fluid surface shading
@fragment
fn surface_fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let fluid_type = u32(input.fluid_type);
    
    // Base colors for fluid types
    var base_color = vec3<f32>(0.0);
    var opacity = 1.0;
    var emission = vec3<f32>(0.0);
    
    switch fluid_type {
        case 1u: { // Water
            base_color = vec3<f32>(0.2, 0.5, 0.8);
            opacity = params.water_opacity;
        }
        case 3u: { // Lava
            base_color = vec3<f32>(1.0, 0.3, 0.0);
            opacity = params.lava_opacity;
            emission = base_color * 5.0;
        }
        case 4u: { // Oil
            base_color = vec3<f32>(0.1, 0.1, 0.1);
            opacity = params.oil_opacity;
        }
        case 5u: { // Steam
            base_color = vec3<f32>(0.9, 0.9, 0.9);
            opacity = 0.3;
        }
        default: {
            base_color = vec3<f32>(0.5, 0.5, 0.5);
        }
    }
    
    // Calculate view direction
    let view_dir = normalize(camera.position - input.world_pos);
    
    // Simple lighting
    let light_dir = normalize(vec3<f32>(1.0, 2.0, 1.0));
    let n_dot_l = max(dot(input.normal, light_dir), 0.0);
    
    // Fresnel effect for water
    var fresnel = 1.0;
    if (fluid_type == 1u) {
        let n_dot_v = max(dot(input.normal, view_dir), 0.0);
        fresnel = pow(1.0 - n_dot_v, 2.0);
    }
    
    // Reflection for water and oil
    var reflection = vec3<f32>(0.0);
    if (fluid_type == 1u || fluid_type == 4u) {
        let reflect_dir = reflect(-view_dir, input.normal);
        reflection = textureSample(env_map, env_sampler, reflect_dir).rgb * params.reflection_strength;
    }
    
    // Combine lighting
    var final_color = base_color * n_dot_l + emission;
    final_color = mix(final_color, reflection, fresnel * 0.5);
    
    // Add foam based on velocity magnitude
    let speed = length(input.velocity);
    if (speed > params.foam_threshold && fluid_type == 1u) {
        let foam = smoothstep(params.foam_threshold, params.foam_threshold * 2.0, speed);
        final_color = mix(final_color, vec3<f32>(1.0), foam * 0.5);
    }
    
    return vec4<f32>(final_color, opacity);
}