// Shader Snippets - Reusable WGSL code fragments
// Pure data, no functions or classes

export const SHADER_SNIPPETS = {
    // Morton encoding for 32-bit coordinates
    mortonEncode: `
        fn morton_encode_3d(x: u32, y: u32, z: u32) -> u32 {
            // TEMPORARY: Simple linear indexing for debugging
            return y * 256u * 256u + z * 256u + x;
            
            // 32-bit Morton encoding for 10-bit coordinates (up to 1024)
            var xx = x & 0x3FFu; // 10 bits
            var yy = y & 0x3FFu;
            var zz = z & 0x3FFu;
            
            // Spread bits - adapted for 32-bit arithmetic
            xx = (xx | (xx << 16u)) & 0x030000FFu;
            xx = (xx | (xx << 8u))  & 0x0300F00Fu;
            xx = (xx | (xx << 4u))  & 0x030C30C3u;
            xx = (xx | (xx << 2u))  & 0x09249249u;
            
            yy = (yy | (yy << 16u)) & 0x030000FFu;
            yy = (yy | (yy << 8u))  & 0x0300F00Fu;
            yy = (yy | (yy << 4u))  & 0x030C30C3u;
            yy = (yy | (yy << 2u))  & 0x09249249u;
            
            zz = (zz | (zz << 16u)) & 0x030000FFu;
            zz = (zz | (zz << 8u))  & 0x0300F00Fu;
            zz = (zz | (zz << 4u))  & 0x030C30C3u;
            zz = (zz | (zz << 2u))  & 0x09249249u;
            
            return xx | (yy << 1u) | (zz << 2u);
        }
    `,
    
    // Simple noise function
    noise3d: `
        fn hash(p: vec3<f32>) -> f32 {
            var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
            p3 += dot(p3, p3.yxz + 33.33);
            return fract((p3.x + p3.y) * p3.z);
        }
        
        fn noise3d(p: vec3<f32>) -> f32 {
            let i = floor(p);
            let f = fract(p);
            let u = f * f * (3.0 - 2.0 * f);
            
            return mix(
                mix(mix(hash(i + vec3<f32>(0,0,0)), hash(i + vec3<f32>(1,0,0)), u.x),
                    mix(hash(i + vec3<f32>(0,1,0)), hash(i + vec3<f32>(1,1,0)), u.x), u.y),
                mix(mix(hash(i + vec3<f32>(0,0,1)), hash(i + vec3<f32>(1,0,1)), u.x),
                    mix(hash(i + vec3<f32>(0,1,1)), hash(i + vec3<f32>(1,1,1)), u.x), u.y),
                u.z
            );
        }
    `,
    
    // Camera uniforms struct
    cameraStruct: `
        struct CameraUniforms {
            view: mat4x4<f32>,
            projection: mat4x4<f32>,
            viewProjection: mat4x4<f32>,
            position: vec4<f32>,
        }
    `,
    
    // Vertex struct
    vertexStruct: `
        struct Vertex {
            position: vec3<f32>,
            normal: vec3<f32>,
            uv: vec2<f32>,
            color: u32,
        }
    `,
    
    // Color unpacking
    unpackColor: `
        fn unpack_color(packed: u32) -> vec4<f32> {
            return vec4<f32>(
                f32(packed & 0xFFu) / 255.0,           // r
                f32((packed >> 8u) & 0xFFu) / 255.0,   // g
                f32((packed >> 16u) & 0xFFu) / 255.0,  // b
                f32((packed >> 24u) & 0xFFu) / 255.0   // a
            );
        }
    `,
    
    // Indirect draw struct
    indirectDraw: `
        struct DrawIndexedIndirect {
            index_count: u32,
            instance_count: u32,
            first_index: u32,
            base_vertex: i32,
            first_instance: u32,
        }
    `
};