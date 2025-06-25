//! Block system for the unified world architecture
//!
//! This module provides block definitions and registration for the GPU-first world system.

mod basic_blocks;
pub mod block_data;

pub use basic_blocks::register_basic_blocks;
