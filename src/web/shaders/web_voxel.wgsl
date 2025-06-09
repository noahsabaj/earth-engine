// Web voxel rendering shader
// Optimized for browser performance with minimal state changes

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) ao: f32,
    @location(4) light_level: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) ao: f32,
    @location(4) light_level: f32,
}

// Camera uniforms (temporarily hardcoded for Sprint 22)
const view_proj = mat4x4<f32>(
    vec4<f32>(1.0, 0.0, 0.0, 0.0),
    vec4<f32>(0.0, 1.0, 0.0, 0.0),
    vec4<f32>(0.0, 0.0, 1.0, 0.0),
    vec4<f32>(0.0, 0.0, 0.0, 1.0)
);

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position
    let world_pos = in.position;
    out.clip_position = view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    
    // Pass through attributes
    out.normal = in.normal;
    out.uv = in.uv;
    out.ao = in.ao;
    out.light_level = in.light_level;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Basic block color (grass-like for testing)
    let base_color = vec3<f32>(0.3, 0.7, 0.2);
    
    // Apply lighting
    let ambient = 0.2;
    let diffuse = max(dot(in.normal, vec3<f32>(0.5, 0.8, 0.3)), 0.0);
    let light = ambient + diffuse * in.light_level;
    
    // Apply ambient occlusion
    let ao_factor = mix(0.5, 1.0, in.ao);
    
    // Final color
    let final_color = base_color * light * ao_factor;
    
    return vec4<f32>(final_color, 1.0);
}