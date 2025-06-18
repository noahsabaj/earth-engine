// AUTO-GENERATED - DO NOT EDIT
// Generated from Rust GPU type definitions by build.rs

// Generic block distribution rule
struct BlockDistribution {
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
