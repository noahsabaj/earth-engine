// GPU mesh rendering shader - reads from separate vertex attribute buffers

struct CameraUniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// Separate vertex attribute buffers from GPU mesh generation
@group(1) @binding(0) var<storage, read> positions: array<vec3<f32>>;
@group(1) @binding(1) var<storage, read> normals: array<vec3<f32>>;
@group(1) @binding(2) var<storage, read> uvs: array<vec2<f32>>;
@group(1) @binding(3) var<storage, read> colors: array<vec4<f32>>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Read vertex attributes from separate buffers
    let position = positions[vertex_idx];
    let normal = normals[vertex_idx];
    let color = colors[vertex_idx];
    
    // Transform position to clip space
    out.clip_position = camera.view_proj * vec4<f32>(position, 1.0);
    out.world_position = position;
    out.world_normal = normal;
    out.color = color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diffuse = max(dot(normalize(in.world_normal), light_dir), 0.0);
    let ambient = 0.2;
    let light = ambient + diffuse * 0.8;
    
    // Apply lighting to color
    return vec4<f32>(in.color.rgb * light, in.color.a);
}