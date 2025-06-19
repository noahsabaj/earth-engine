//! Block system for the unified world architecture
//! 
//! This module provides block definitions and registration for the GPU-first world system.

mod basic_blocks;

pub use basic_blocks::{
    GrassBlock, DirtBlock, StoneBlock, WaterBlock, SandBlock, GlowstoneBlock,
    register_basic_blocks,
};