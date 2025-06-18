//! Build script for generating WGSL type definitions from Rust GPU types

use std::{env, fs, path::Path};

// Import the constants module types for build-time generation
const CHUNK_SIZE: u32 = 32;
const MAX_BLOCK_DISTRIBUTIONS: usize = 16;

fn main() {
    // Only regenerate if GPU types change
    println!("cargo:rerun-if-changed=src/gpu/types");
    println!("cargo:rerun-if-changed=src/gpu/soa");
    println!("cargo:rerun-if-changed=src/gpu/shader_bridge.rs");
    println!("cargo:rerun-if-changed=src/gpu/constants.rs");
    
    // Get output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let soa_generated_path = Path::new(&out_dir).join("gpu_types_soa.wgsl");
    let constants_generated_path = Path::new(&out_dir).join("gpu_constants.wgsl");
    
    // Generate WGSL content (SOA only)
    let soa_wgsl_content = generate_soa_wgsl_types();
    let constants_wgsl_content = generate_wgsl_constants();
    
    // Write to output directory
    fs::write(&soa_generated_path, &soa_wgsl_content)
        .expect("Failed to write generated SOA WGSL");
    fs::write(&constants_generated_path, &constants_wgsl_content)
        .expect("Failed to write generated constants WGSL");
    
    // Also copy to src directory for shader includes
    let shader_dir = Path::new("src/gpu/shaders/generated");
    fs::create_dir_all(shader_dir)
        .expect("Failed to create shader directory");
    
    let soa_shader_path = shader_dir.join("types_soa.wgsl");
    fs::write(&soa_shader_path, &soa_wgsl_content)
        .expect("Failed to write SOA WGSL to src directory");
    
    let constants_shader_path = shader_dir.join("constants.wgsl");
    fs::write(&constants_shader_path, &constants_wgsl_content)
        .expect("Failed to write constants WGSL to src directory");
    
    println!("cargo:warning=Generated SOA WGSL types at {:?}", soa_shader_path);
    println!("cargo:warning=Generated constants WGSL at {:?}", constants_shader_path);
}


/// Generate SOA WGSL type definitions
fn generate_soa_wgsl_types() -> String {
    
    format!(r#"// AUTO-GENERATED - DO NOT EDIT
// Generated from Rust GPU type definitions by build.rs
// Structure of Arrays (SOA) types for maximum GPU performance

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

// Chunk metadata for GPU world buffer
struct ChunkMetadata {{
    flags: u32,         // Bit 0: generated, Bit 1: modified, etc.
    timestamp: u32,     // Generation timestamp
    checksum: u32,      // For validation
    reserved: u32,
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
"#)
}

/// Generate WGSL constants
fn generate_wgsl_constants() -> String {
    format!(r#"// AUTO-GENERATED GPU CONSTANTS - DO NOT EDIT
// Generated from src/gpu/constants.rs

// Core constants
const CHUNK_SIZE: u32 = {}u;
const CHUNK_SIZE_F: f32 = {}.0;
const VOXELS_PER_CHUNK: u32 = {}u;
const MAX_WORLD_SIZE: u32 = 512u;
const MAX_BLOCK_DISTRIBUTIONS: u32 = {}u;

// Block IDs - Single source of truth
const BLOCK_AIR: u32 = 0u;
const BLOCK_STONE: u32 = 1u;
const BLOCK_DIRT: u32 = 2u;
const BLOCK_GRASS: u32 = 3u;
const BLOCK_WOOD: u32 = 4u;
const BLOCK_SAND: u32 = 5u;
const BLOCK_WATER: u32 = 6u;
const BLOCK_LEAVES: u32 = 7u;
const BLOCK_GLASS: u32 = 8u;
const BLOCK_CHEST: u32 = 9u;
const BLOCK_LAVA: u32 = 10u;
const BLOCK_BRICK: u32 = 11u;

// Game blocks start at ID 100
const GAME_BLOCK_START: u32 = 100u;
"#, 
        CHUNK_SIZE,
        CHUNK_SIZE,
        CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE,
        MAX_BLOCK_DISTRIBUTIONS,
    )
}