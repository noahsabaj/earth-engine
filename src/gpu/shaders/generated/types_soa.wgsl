// AUTO-GENERATED - DO NOT EDIT
// Generated from Rust GPU type definitions by build.rs
// Structure of Arrays (SOA) types for maximum GPU performance

// SOA representation of block distributions for coalesced memory access
struct BlockDistributionSOA {
    count: u32,
    _pad: vec3<u32>,
    
    // Pure arrays - each field stored contiguously for optimal cache usage
    block_ids: array<u32, MAX_BLOCK_DISTRIBUTIONS>,
    min_heights: array<i32, MAX_BLOCK_DISTRIBUTIONS>,
    max_heights: array<i32, MAX_BLOCK_DISTRIBUTIONS>,
    probabilities: array<f32, MAX_BLOCK_DISTRIBUTIONS>,
    noise_thresholds: array<f32, MAX_BLOCK_DISTRIBUTIONS>,
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

// Optimized height check function for SOA data
fn check_height_soa(distributions: ptr<storage, BlockDistributionSOA>, world_y: i32) -> u32 {
    let count = (*distributions).count;
    
    // Coalesced memory access - all threads read sequential elements
    for (var i = 0u; i < count; i++) {
        if (world_y >= (*distributions).min_heights[i] && 
            world_y <= (*distributions).max_heights[i]) {
            return (*distributions).block_ids[i];
        }
    }
    
    return 0u;
}

// Vectorized height check (processes 4 distributions at once)
fn check_height_soa_vec4(distributions: ptr<storage, BlockDistributionSOA>, world_y: i32) -> u32 {
    let count = (*distributions).count;
    let y_vec = vec4<i32>(world_y);
    
    // Process 4 distributions at a time using SIMD
    for (var i = 0u; i < count; i += 4u) {
        // Check bounds to avoid out-of-bounds access
        let remaining = min(4u, count - i);
        
        if (remaining >= 1u) {
            if (world_y >= (*distributions).min_heights[i] && 
                world_y <= (*distributions).max_heights[i]) {
                return (*distributions).block_ids[i];
            }
        }
        if (remaining >= 2u) {
            if (world_y >= (*distributions).min_heights[i + 1] && 
                world_y <= (*distributions).max_heights[i + 1]) {
                return (*distributions).block_ids[i + 1];
            }
        }
        if (remaining >= 3u) {
            if (world_y >= (*distributions).min_heights[i + 2] && 
                world_y <= (*distributions).max_heights[i + 2]) {
                return (*distributions).block_ids[i + 2];
            }
        }
        if (remaining >= 4u) {
            if (world_y >= (*distributions).min_heights[i + 3] && 
                world_y <= (*distributions).max_heights[i + 3]) {
                return (*distributions).block_ids[i + 3];
            }
        }
    }
    
    return 0u;
}
