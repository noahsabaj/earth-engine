// Foam particle rendering shader

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
    @location(0) center: vec3<f32>,
    @location(1) size: f32,
    @location(2) lifetime: f32,
}

// Hash function for pseudo-random numbers
fn hash(p: vec3<u32>) -> f32 {
    var h = (p.x ^ (p.y << 5u) ^ (p.z << 11u)) * 0x9e3779b9u;
    h = h ^ (h >> 16u);
    h = h * 0x85ebca6bu;
    h = h ^ (h >> 13u);
    h = h * 0xc2b2ae35u;
    h = h ^ (h >> 16u);
    return f32(h) / 4294967296.0;
}

// Generate foam particles from high turbulence areas
@vertex
fn foam_vertex(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    var output: VertexOutput;
    
    // Grid size
    let grid_size = vec3<u32>(128u, 64u, 128u);
    
    // Find foam generation points (high velocity/turbulence areas)
    let scan_step = 4u; // Sample every 4th voxel for performance
    let foam_idx = instance_index;
    
    var foam_count = 0u;
    var foam_pos = vec3<f32>(0.0);
    var foam_velocity = vec3<f32>(0.0);
    
    // Scan for foam locations
    for (var z = 0u; z < grid_size.z; z += scan_step) {
        for (var y = 0u; y < grid_size.y; y += scan_step) {
            for (var x = 0u; x < grid_size.x; x += scan_step) {
                let idx = x + y * grid_size.x + z * grid_size.x * grid_size.y;
                
                if (idx < arrayLength(&fluid_buffer)) {
                    let fluid = fluid_buffer[idx];
                    let fluid_type = fluid.packed_data & 0xFFu;
                    
                    // Only generate foam for water
                    if (fluid_type == 1u) {
                        let velocity = vec3<f32>(fluid.velocity_x, fluid.velocity_y, fluid.velocity_z);
                        let speed = length(velocity);
                        
                        // Check if above foam threshold
                        if (speed > params.foam_threshold) {
                            if (foam_count == foam_idx) {
                                foam_pos = vec3<f32>(f32(x), f32(y), f32(z));
                                foam_velocity = velocity;
                                break;
                            }
                            foam_count++;
                        }
                    }
                }
            }
            if (foam_count > foam_idx) { break; }
        }
        if (foam_count > foam_idx) { break; }
    }
    
    // Generate billboard quad vertices
    let quad_positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    
    let vertex_pos = quad_positions[vertex_index % 6u];
    
    // Add random offset for variety
    let random_offset = vec3<f32>(
        hash(vec3<u32>(foam_idx, 0u, 0u)) - 0.5,
        hash(vec3<u32>(foam_idx, 1u, 0u)) - 0.5,
        hash(vec3<u32>(foam_idx, 2u, 0u)) - 0.5
    ) * 0.5;
    
    // Particle size based on velocity
    let speed = length(foam_velocity);
    let particle_size = mix(0.1, 0.3, smoothstep(params.foam_threshold, params.foam_threshold * 3.0, speed));
    
    // Lifetime simulation
    let lifetime = hash(vec3<u32>(foam_idx, 3u, 0u));
    
    // World position with animation
    let world_pos = foam_pos + random_offset + foam_velocity * lifetime * 0.1;
    
    // Billboard transform
    let view_pos = camera.view_proj * vec4<f32>(world_pos, 1.0);
    let screen_offset = vertex_pos * particle_size;
    
    output.position = view_pos + vec4<f32>(screen_offset * view_pos.w * 0.01, 0.0, 0.0);
    output.center = world_pos;
    output.size = particle_size;
    output.lifetime = lifetime;
    
    return output;
}

// Foam fragment shader
@fragment
fn foam_fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Fade out over lifetime
    let alpha = 1.0 - input.lifetime;
    
    // Soft particle edges
    let center_dist = length(input.position.xy - input.center.xy) / input.size;
    let edge_fade = 1.0 - smoothstep(0.5, 1.0, center_dist);
    
    // Foam color (bright white with slight blue tint)
    let foam_color = vec3<f32>(0.95, 0.98, 1.0);
    
    // Size-based opacity (smaller particles are more transparent)
    let size_opacity = smoothstep(0.05, 0.3, input.size);
    
    // Final opacity
    let final_alpha = alpha * edge_fade * size_opacity * 0.7;
    
    return vec4<f32>(foam_color, final_alpha);
}