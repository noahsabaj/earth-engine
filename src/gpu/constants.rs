//! GPU constants - Single source of truth for GPU/CPU shared constants
//! 
//! This module defines constants that are used in both GPU shaders and CPU code.
//! It provides functions to generate WGSL constant definitions from Rust constants,
//! ensuring consistency across the codebase.

use crate::BlockId;

/// Core GPU/World constants
pub mod core {
    /// Chunk dimensions
    pub const CHUNK_SIZE: u32 = 32;
    pub const CHUNK_SIZE_F32: f32 = 32.0;
    pub const VOXELS_PER_CHUNK: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    
    /// World limits
    pub const MAX_WORLD_SIZE: u32 = 512; // 512³ chunks
    pub const MAX_BLOCK_DISTRIBUTIONS: usize = 16;
}

/// Block ID constants - Single source of truth
pub mod blocks {
    use crate::BlockId;
    
    // Core engine blocks (0-99)
    pub const AIR: BlockId = BlockId(0);
    pub const STONE: BlockId = BlockId(1);
    pub const DIRT: BlockId = BlockId(2);
    pub const GRASS: BlockId = BlockId(3);
    pub const WOOD: BlockId = BlockId(4);
    pub const SAND: BlockId = BlockId(5);
    pub const WATER: BlockId = BlockId(6);
    pub const LEAVES: BlockId = BlockId(7);
    pub const GLASS: BlockId = BlockId(8);
    pub const CHEST: BlockId = BlockId(9);
    pub const LAVA: BlockId = BlockId(10);
    pub const BRICK: BlockId = BlockId(11);
    
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

/// Generate WGSL constants file content
pub fn generate_wgsl_constants() -> String {
    format!(r#"// AUTO-GENERATED GPU CONSTANTS - DO NOT EDIT
// Generated from src/gpu/constants.rs

// Core constants
const CHUNK_SIZE: u32 = {}u;
const CHUNK_SIZE_F: f32 = {};
const VOXELS_PER_CHUNK: u32 = {}u;
const MAX_WORLD_SIZE: u32 = {}u;
const MAX_BLOCK_DISTRIBUTIONS: u32 = {}u;

// Block IDs
const BLOCK_AIR: u32 = {}u;
const BLOCK_STONE: u32 = {}u;
const BLOCK_DIRT: u32 = {}u;
const BLOCK_GRASS: u32 = {}u;
const BLOCK_WOOD: u32 = {}u;
const BLOCK_SAND: u32 = {}u;
const BLOCK_WATER: u32 = {}u;
const BLOCK_LEAVES: u32 = {}u;
const BLOCK_GLASS: u32 = {}u;
const BLOCK_CHEST: u32 = {}u;
const BLOCK_LAVA: u32 = {}u;
const BLOCK_BRICK: u32 = {}u;

// Game blocks start at ID 100
const GAME_BLOCK_START: u32 = {}u;
"#, 
        core::CHUNK_SIZE,
        core::CHUNK_SIZE_F32,
        core::VOXELS_PER_CHUNK,
        core::MAX_WORLD_SIZE,
        core::MAX_BLOCK_DISTRIBUTIONS,
        blocks::AIR.0,
        blocks::STONE.0,
        blocks::DIRT.0,
        blocks::GRASS.0,
        blocks::WOOD.0,
        blocks::SAND.0,
        blocks::WATER.0,
        blocks::LEAVES.0,
        blocks::GLASS.0,
        blocks::CHEST.0,
        blocks::LAVA.0,
        blocks::BRICK.0,
        blocks::GAME_BLOCK_START,
    )
}

/// Shader path constants - Single source of truth for shader locations
pub mod shader_paths {
    /// Base shader directory (relative to src/)
    pub const SHADER_BASE: &str = "gpu/shaders";
    
    /// SOA shader paths
    pub const SOA_TERRAIN_GENERATION: &str = "gpu/shaders/soa/terrain_generation_soa.wgsl";
    
    /// Generated shader paths
    pub const GENERATED_TYPES: &str = "gpu/shaders/generated/types.wgsl";
    pub const GENERATED_TYPES_SOA: &str = "gpu/shaders/generated/types_soa.wgsl";
    pub const GENERATED_CONSTANTS: &str = "gpu/shaders/generated/constants.wgsl";
    
    /// Common shader includes
    pub const PERLIN_NOISE: &str = "renderer/shaders/perlin_noise.wgsl";
}