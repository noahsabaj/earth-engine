// AUTO-GENERATED - DO NOT EDIT
// Generated from Rust GPU type definitions by build.rs

// Maximum number of block distributions (must match Rust)
const MAX_BLOCK_DISTRIBUTIONS: u32 = 16u;

// Generic block distribution rule
struct BlockDistribution {
    block_id: u32,
    min_height: i32,
    max_height: i32,
    probability: f32,
    noise_threshold: f32,
    // Padding for 48-byte alignment (encase handles this automatically)
    _padding: array<u32, 7>,  // 28 bytes to reach 48 total
}

// Terrain generation parameters  
struct TerrainParams {
    seed: u32,
    sea_level: f32,
    terrain_scale: f32,
    mountain_threshold: f32,
    cave_threshold: f32,
    num_distributions: u32,
    _padding: vec2<u32>,
    distributions: array<BlockDistribution, MAX_BLOCK_DISTRIBUTIONS>,
}

// Chunk metadata for GPU world buffer
struct ChunkMetadata {
    flags: u32,         // Bit 0: generated, Bit 1: modified, etc.
    timestamp: u32,     // Generation timestamp
    checksum: u32,      // For validation
    reserved: u32,
}
