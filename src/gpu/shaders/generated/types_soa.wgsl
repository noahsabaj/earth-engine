// AUTO-GENERATED - DO NOT EDIT
// Generated from Rust GPU type definitions by build.rs
// Structure of Arrays (SOA) types for maximum GPU performance

// SOA representation of block distributions for coalesced memory access
struct BlockDistributionSOA {
    count: u32,
    _pad: vec3<u32>,
    
    // Pure arrays - each field stored contiguously for optimal cache usage
    block_ids: array<u32, 16>,
    min_heights: array<i32, 16>,
    max_heights: array<i32, 16>,
    probabilities: array<f32, 16>,
    noise_thresholds: array<f32, 16>,
}

// Chunk metadata for GPU world buffer
struct ChunkMetadata {
    flags: u32,         // Bit 0: generated, Bit 1: modified, etc.
    timestamp: u32,     // Generation timestamp
    checksum: u32,      // For validation
    reserved: u32,
}

// SOA terrain parameters with embedded distributions
struct TerrainParamsSOA {
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
}

// NOTE: check_height_soa removed - use check_height_soa_global in terrain_generation_soa.wgsl instead
// WGSL does not allow passing storage pointers as function parameters

// NOTE: check_height_soa_vec4 removed - use check_height_soa_global in terrain_generation_soa.wgsl instead
// WGSL does not allow passing storage pointers as function parameters
