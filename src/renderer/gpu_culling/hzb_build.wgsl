/// Hierarchical Z-Buffer Build Shader
/// 
/// Builds mip chain for HZB by taking maximum depth in 2x2 regions.
/// This creates a conservative depth pyramid for occlusion culling.

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<r32float, write>;

// Build one mip level from the previous
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let output_coord = global_id.xy;
    let output_dims = textureDimensions(output_texture);
    
    // Check bounds
    if (output_coord.x >= output_dims.x || output_coord.y >= output_dims.y) {
        return;
    }
    
    // Sample 2x2 region from input texture
    let input_coord = output_coord * 2u;
    
    // Take maximum (furthest) depth for conservative culling
    var max_depth = 0.0;
    max_depth = max(max_depth, textureLoad(input_texture, input_coord + vec2<u32>(0u, 0u), 0).r);
    max_depth = max(max_depth, textureLoad(input_texture, input_coord + vec2<u32>(1u, 0u), 0).r);
    max_depth = max(max_depth, textureLoad(input_texture, input_coord + vec2<u32>(0u, 1u), 0).r);
    max_depth = max(max_depth, textureLoad(input_texture, input_coord + vec2<u32>(1u, 1u), 0).r);
    
    // Write to output
    textureStore(output_texture, output_coord, vec4<f32>(max_depth, 0.0, 0.0, 0.0));
}

/// Copy depth buffer to HZB mip 0
@compute @workgroup_size(8, 8)
fn copy_depth(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coord = global_id.xy;
    let dims = textureDimensions(output_texture);
    
    if (coord.x >= dims.x || coord.y >= dims.y) {
        return;
    }
    
    // Sample depth and write to HZB
    let depth = textureLoad(input_texture, coord, 0).r;
    textureStore(output_texture, coord, vec4<f32>(depth, 0.0, 0.0, 0.0));
}