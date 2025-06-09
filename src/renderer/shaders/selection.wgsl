struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct ModelUniform {
    model: mat4x4<f32>,
    progress: f32,
};

@group(1) @binding(0)
var<uniform> model_uniform: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model_uniform.model * vec4<f32>(input.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Change color from white to red based on breaking progress
    // White (1,1,1) -> Red (1,0,0)
    let red = 1.0;
    let green = 1.0 - model_uniform.progress;
    let blue = 1.0 - model_uniform.progress;
    return vec4<f32>(red, green, blue, 0.8);
}