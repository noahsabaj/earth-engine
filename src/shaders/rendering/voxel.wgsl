struct CameraUniform {
    view: mat4x4<f32>,              // View matrix (not used in this shader)
    projection: mat4x4<f32>,         // Projection matrix (not used in this shader)
    view_proj: mat4x4<f32>,          // Combined view-projection matrix
    position: vec3<f32>,             // Camera world position for fog calculation
    _padding: f32,                   // Padding to ensure 16-byte alignment
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) light: f32,
    @location(4) ao: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) light: f32,
    @location(4) ao: f32,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.normal = model.normal;
    out.world_pos = model.position;
    out.light = model.light;
    out.ao = model.ao;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Combine block/sky light with simple directional shading
    let light_dir = normalize(vec3<f32>(0.5, -1.0, 0.3));
    let directional = max(dot(in.normal, -light_dir), 0.0) * 0.3;
    
    // Use the per-vertex light level
    let block_light = in.light;
    
    // Apply ambient occlusion
    let ao_factor = in.ao;
    
    // Combine all lighting
    let final_light = (block_light + directional) * ao_factor;
    
    // Apply fog based on distance from camera
    // Calculate the distance from the fragment's world position to the camera position
    let fog_distance = length(in.world_pos - camera.position);
    
    // Exponential fog: factor approaches 0 as distance increases
    // The 0.002 coefficient controls fog density (higher = denser fog)
    let fog_factor = exp(-fog_distance * 0.002);
    
    // Mix between fog color (light blue) and the lit fragment color based on fog factor
    // fog_factor = 1.0 at camera position (no fog), approaches 0.0 at distance (full fog)
    let fog_color = vec3<f32>(0.7, 0.8, 0.9);
    let final_color = mix(fog_color, in.color * final_light, fog_factor);
    
    return vec4<f32>(final_color, 1.0);
}