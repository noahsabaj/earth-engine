//! Build script for generating WGSL type definitions from Rust GPU types

use std::{env, fs, path::Path};

fn main() {
    // Only regenerate if GPU types change
    println!("cargo:rerun-if-changed=src/gpu/types");
    println!("cargo:rerun-if-changed=src/gpu/soa");
    println!("cargo:rerun-if-changed=src/gpu/shader_bridge.rs");
    
    // Get output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let generated_path = Path::new(&out_dir).join("gpu_types.wgsl");
    let soa_generated_path = Path::new(&out_dir).join("gpu_types_soa.wgsl");
    
    // Generate WGSL content
    let wgsl_content = generate_wgsl_types();
    let soa_wgsl_content = generate_soa_wgsl_types();
    
    // Write to output directory
    fs::write(&generated_path, &wgsl_content)
        .expect("Failed to write generated WGSL");
    fs::write(&soa_generated_path, &soa_wgsl_content)
        .expect("Failed to write generated SOA WGSL");
    
    // Also copy to src directory for shader includes
    let shader_dir = Path::new("src/gpu/shaders/generated");
    fs::create_dir_all(shader_dir)
        .expect("Failed to create shader directory");
    
    let shader_path = shader_dir.join("types.wgsl");
    fs::write(&shader_path, &wgsl_content)
        .expect("Failed to write WGSL to src directory");
    
    let soa_shader_path = shader_dir.join("types_soa.wgsl");
    fs::write(&soa_shader_path, &soa_wgsl_content)
        .expect("Failed to write SOA WGSL to src directory");
    
    println!("cargo:warning=Generated WGSL types at {:?}", shader_path);
    println!("cargo:warning=Generated SOA WGSL types at {:?}", soa_shader_path);
}

/// Generate WGSL type definitions
/// 
/// Note: In a real implementation, this would use the shader_bridge module.
/// For now, we inline the generation to avoid circular dependencies.
fn generate_wgsl_types() -> String {
    const MAX_BLOCK_DISTRIBUTIONS: usize = 16;
    
    format!(r#"// AUTO-GENERATED - DO NOT EDIT
// Generated from Rust GPU type definitions by build.rs

// Maximum number of block distributions (must match Rust)
const MAX_BLOCK_DISTRIBUTIONS: u32 = {}u;

// Generic block distribution rule
struct BlockDistribution {{
    block_id: u32,
    min_height: i32,
    max_height: i32,
    probability: f32,
    noise_threshold: f32,
    // Padding to reach 48 bytes total (7 * 4 = 28 bytes of padding)
    // Using individual fields instead of array to avoid WGSL uniform buffer
    // array alignment requirements (arrays in uniforms need 16-byte aligned elements)
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,
    _pad5: u32,
    _pad6: u32,
}}

// Terrain generation parameters  
struct TerrainParams {{
    seed: u32,
    sea_level: f32,
    terrain_scale: f32,
    mountain_threshold: f32,
    cave_threshold: f32,
    num_distributions: u32,
    _padding: vec2<u32>,
    distributions: array<BlockDistribution, MAX_BLOCK_DISTRIBUTIONS>,
}}

// Chunk metadata for GPU world buffer
struct ChunkMetadata {{
    flags: u32,         // Bit 0: generated, Bit 1: modified, etc.
    timestamp: u32,     // Generation timestamp
    checksum: u32,      // For validation
    reserved: u32,
}}
"#, MAX_BLOCK_DISTRIBUTIONS)
}

/// Generate SOA WGSL type definitions
fn generate_soa_wgsl_types() -> String {
    const MAX_BLOCK_DISTRIBUTIONS: usize = 16;
    
    format!(r#"// AUTO-GENERATED - DO NOT EDIT
// Generated from Rust GPU type definitions by build.rs
// Structure of Arrays (SOA) types for maximum GPU performance

// Maximum number of block distributions (must match Rust)
const MAX_BLOCK_DISTRIBUTIONS: u32 = {}u;

// SOA representation of block distributions for coalesced memory access
struct BlockDistributionSOA {{
    count: u32,
    _pad: vec3<u32>,
    
    // Pure arrays - each field stored contiguously for optimal cache usage
    block_ids: array<u32, MAX_BLOCK_DISTRIBUTIONS>,
    min_heights: array<i32, MAX_BLOCK_DISTRIBUTIONS>,
    max_heights: array<i32, MAX_BLOCK_DISTRIBUTIONS>,
    probabilities: array<f32, MAX_BLOCK_DISTRIBUTIONS>,
    noise_thresholds: array<f32, MAX_BLOCK_DISTRIBUTIONS>,
}}

// SOA terrain parameters with embedded distributions
struct TerrainParamsSOA {{
    // Scalar parameters
    seed: u32,
    sea_level: f32,
    terrain_scale: f32,
    mountain_threshold: f32,
    cave_threshold: f32,
    num_distributions: u32,
    _pad: vec2<u32>,
    
    // Embedded SOA distributions
    distributions: BlockDistributionSOA,
}}

// Optimized height check function for SOA data
fn check_height_soa(distributions: ptr<storage, BlockDistributionSOA>, world_y: i32) -> u32 {{
    let count = (*distributions).count;
    
    // Coalesced memory access - all threads read sequential elements
    for (var i = 0u; i < count; i++) {{
        if (world_y >= (*distributions).min_heights[i] && 
            world_y <= (*distributions).max_heights[i]) {{
            return (*distributions).block_ids[i];
        }}
    }}
    
    return 0u;
}}

// Vectorized height check (processes 4 distributions at once)
fn check_height_soa_vec4(distributions: ptr<storage, BlockDistributionSOA>, world_y: i32) -> u32 {{
    let count = (*distributions).count;
    let y_vec = vec4<i32>(world_y);
    
    // Process 4 distributions at a time using SIMD
    for (var i = 0u; i < count; i += 4u) {{
        // Check bounds to avoid out-of-bounds access
        let remaining = min(4u, count - i);
        
        if (remaining >= 1u) {{
            if (world_y >= (*distributions).min_heights[i] && 
                world_y <= (*distributions).max_heights[i]) {{
                return (*distributions).block_ids[i];
            }}
        }}
        if (remaining >= 2u) {{
            if (world_y >= (*distributions).min_heights[i + 1] && 
                world_y <= (*distributions).max_heights[i + 1]) {{
                return (*distributions).block_ids[i + 1];
            }}
        }}
        if (remaining >= 3u) {{
            if (world_y >= (*distributions).min_heights[i + 2] && 
                world_y <= (*distributions).max_heights[i + 2]) {{
                return (*distributions).block_ids[i + 2];
            }}
        }}
        if (remaining >= 4u) {{
            if (world_y >= (*distributions).min_heights[i + 3] && 
                world_y <= (*distributions).max_heights[i + 3]) {{
                return (*distributions).block_ids[i + 3];
            }}
        }}
    }}
    
    return 0u;
}}
"#, MAX_BLOCK_DISTRIBUTIONS)
}