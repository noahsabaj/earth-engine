// Hearth Engine Constants - SINGLE SOURCE OF TRUTH
// 
// This file contains ALL constants used throughout the engine.
// Both CPU and GPU code include this file to ensure perfect consistency.
// 
// CRITICAL: Do NOT define constants anywhere else in the codebase!

/// Core GPU/World constants
pub mod core {
    /// Chunk dimensions - 1dcm³ (10cm) voxels with 50×50×50 chunks (5m³ per chunk)
    pub const CHUNK_SIZE: u32 = 50;
    pub const CHUNK_SIZE_F32: f32 = 50.0;
    pub const VOXELS_PER_CHUNK: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    
    /// World limits
    pub const MAX_WORLD_SIZE: u32 = 512; // 512³ chunks
    pub const MAX_BLOCK_DISTRIBUTIONS: usize = 16;
}

/// Block ID constants - Single source of truth (raw u16 values)
pub mod blocks {
    // Core engine blocks (0-99)
    pub const AIR: u16 = 0;
    pub const STONE: u16 = 1;
    pub const DIRT: u16 = 2;
    pub const GRASS: u16 = 3;
    pub const WOOD: u16 = 4;
    pub const SAND: u16 = 5;
    pub const WATER: u16 = 6;
    pub const LEAVES: u16 = 7;
    pub const GLASS: u16 = 8;
    pub const CHEST: u16 = 9;
    pub const LAVA: u16 = 10;
    pub const BRICK: u16 = 11;
    
    // Reserved for game-specific blocks (100+)
    // Games can define their own blocks starting from ID 100
    pub const GAME_BLOCK_START: u16 = 100;
}

/// GPU buffer alignment requirements
pub mod alignment {
    /// WGSL requires 16-byte alignment for storage buffers
    pub const STORAGE_BUFFER_ALIGN: u64 = 16;
    
    /// Uniform buffers require 256-byte alignment
    pub const UNIFORM_BUFFER_ALIGN: u64 = 256;
}

/// Shader path constants - Single source of truth for shader locations
pub mod shader_paths {
    /// Base shader directory (relative to src/)
    pub const SHADER_BASE: &str = "gpu/shaders";
    
    /// SOA shader paths
    pub const SOA_TERRAIN_GENERATION: &str = "gpu/shaders/soa/terrain_generation_soa.wgsl";
    
    /// Generated shader paths (SOA only for optimal performance)
    pub const GENERATED_TYPES_SOA: &str = "gpu/shaders/generated/types_soa.wgsl";
    pub const GENERATED_CONSTANTS: &str = "gpu/shaders/generated/constants.wgsl";
    
    /// Common shader includes
    pub const PERLIN_NOISE: &str = "renderer/shaders/perlin_noise.wgsl";
}