// GPU-driven rendering shader with instancing

struct CameraUniform {
    view_proj: mat4x4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
}

struct InstanceInput {
    // Model matrix (4 vec4s)
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    // Instance color
    @location(9) instance_color: vec4<f32>,
    // Custom data
    @location(10) custom_data: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) custom_data: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Reconstruct model matrix
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    
    // Transform vertex to world space
    let world_position = model_matrix * vec4<f32>(vertex.position, 1.0);
    out.world_position = world_position.xyz;
    
    // Transform to clip space
    out.clip_position = camera.view_proj * world_position;
    
    // Transform normal to world space (assuming uniform scale)
    let normal_matrix = mat3x3<f32>(
        model_matrix[0].xyz,
        model_matrix[1].xyz,
        model_matrix[2].xyz,
    );
    out.world_normal = normalize(normal_matrix * vertex.normal);
    
    // Combine vertex and instance colors
    out.color = vec4<f32>(vertex.color, 1.0) * instance.instance_color;
    
    // Pass through custom data
    out.custom_data = instance.custom_data;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(0.5, -1.0, 0.3));
    let ambient = 0.2;
    let diffuse = max(dot(in.world_normal, -light_dir), 0.0);
    let lighting = ambient + diffuse * 0.8;
    
    // Apply lighting to color
    var final_color = in.color;
    final_color.r = final_color.r * lighting;
    final_color.g = final_color.g * lighting;
    final_color.b = final_color.b * lighting;
    
    // Optional: Use custom data for effects
    // For example, custom_data.x could be emission strength
    if (in.custom_data.x > 0.0) {
        let emission = in.color.rgb * in.custom_data.x;
        final_color.r = final_color.r + emission.r;
        final_color.g = final_color.g + emission.g;
        final_color.b = final_color.b + emission.b;
    }
    
    return final_color;
}

// Alternative fragment shader for LOD visualization
@fragment
fn fs_main_lod_debug(in: VertexOutput) -> @location(0) vec4<f32> {
    // Color based on LOD level (stored in custom_data.y)
    let lod_level = u32(in.custom_data.y);
    
    var lod_color: vec3<f32>;
    switch (lod_level) {
        case 0u: { lod_color = vec3<f32>(0.0, 1.0, 0.0); }  // Green = LOD 0
        case 1u: { lod_color = vec3<f32>(1.0, 1.0, 0.0); }  // Yellow = LOD 1
        case 2u: { lod_color = vec3<f32>(1.0, 0.5, 0.0); }  // Orange = LOD 2
        default: { lod_color = vec3<f32>(1.0, 0.0, 0.0); }  // Red = LOD 3+
    }
    
    // Mix with original color
    let final_color = mix(in.color.rgb, lod_color, 0.5);
    
    return vec4<f32>(final_color, in.color.a);
}