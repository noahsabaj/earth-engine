//! GPU constants - Single source of truth for GPU/CPU shared constants
//! 
//! This module re-exports constants from the root constants.rs file and provides
//! functions to generate WGSL constant definitions from Rust constants,
//! ensuring consistency across the codebase.

use crate::BlockId;

// Import constants from single source of truth
include!("../../constants.rs");

// Re-export core constants for compatibility
pub use self::core::*;
pub use self::alignment::*;

// Export raw block constants at module level for WGSL generation
pub use self::blocks::*;

/// Block ID constants - Wrapped in BlockId type for type safety
pub mod typed_blocks {
    use crate::BlockId;
    use super::blocks;
    
    // Core engine blocks (0-99) - wrapped in BlockId type
    pub const AIR: BlockId = BlockId(blocks::AIR);
    pub const STONE: BlockId = BlockId(blocks::STONE);
    pub const DIRT: BlockId = BlockId(blocks::DIRT);
    pub const GRASS: BlockId = BlockId(blocks::GRASS);
    pub const WOOD: BlockId = BlockId(blocks::WOOD);
    pub const SAND: BlockId = BlockId(blocks::SAND);
    pub const WATER: BlockId = BlockId(blocks::WATER);
    pub const LEAVES: BlockId = BlockId(blocks::LEAVES);
    pub const GLASS: BlockId = BlockId(blocks::GLASS);
    pub const CHEST: BlockId = BlockId(blocks::CHEST);
    pub const LAVA: BlockId = BlockId(blocks::LAVA);
    pub const BRICK: BlockId = BlockId(blocks::BRICK);
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
        CHUNK_SIZE,
        CHUNK_SIZE_F32,
        VOXELS_PER_CHUNK,
        MAX_WORLD_SIZE,
        MAX_BLOCK_DISTRIBUTIONS as u32,
        AIR as u32,
        STONE as u32,
        DIRT as u32,
        GRASS as u32,
        WOOD as u32,
        SAND as u32,
        WATER as u32,
        LEAVES as u32,
        GLASS as u32,
        CHEST as u32,
        LAVA as u32,
        BRICK as u32,
        GAME_BLOCK_START as u32,
    )
}

// Re-export shader paths from single source of truth
pub use shader_paths::*;