//! GPU-first lighting system for the unified world architecture
//!
//! Complete lighting system migrated from CPU to GPU for optimal performance.
//! Provides time-of-day, light propagation, and skylight calculations.

mod skylight;
mod time_of_day;

use crate::lighting::{LIGHT_FALLOFF, MAX_LIGHT_LEVEL, MIN_LIGHT_LEVEL};
use crate::world::core::{BlockId, ChunkPos, VoxelPos};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;

pub use skylight::SkylightCalculator;
pub use time_of_day::*;

/// Types of light in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    /// Sunlight/skylight that comes from above
    Sky,
    /// Block light from torches, lava, etc.
    Block,
}

/// Light level (0-15) with separate sky and block light components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightLevel {
    /// Skylight level (0-15)
    pub sky: u8,
    /// Block light level (0-15)
    pub block: u8,
}

impl LightLevel {
    pub fn new(sky: u8, block: u8) -> Self {
        Self {
            sky: sky.min(15),
            block: block.min(15),
        }
    }

    /// Get the maximum light level from either source
    pub fn max_light(&self) -> u8 {
        self.sky.max(self.block)
    }

    /// Get combined light level for rendering
    pub fn combined(&self) -> u8 {
        self.sky.max(self.block)
    }

    /// Create a dark light level
    pub fn dark() -> Self {
        Self { sky: 0, block: 0 }
    }

    /// Create a fully lit skylight level
    pub fn full_sky() -> Self {
        Self { sky: 15, block: 0 }
    }
}

/// Light update request for GPU processing
#[derive(Debug, Clone)]
pub struct LightUpdate {
    pub pos: VoxelPos,
    pub light_type: LightType,
    pub level: u8,
    pub is_removal: bool,
}

/// Lighting system performance statistics
#[derive(Debug, Clone, Default)]
pub struct LightingStats {
    pub updates_processed: usize,
    pub chunks_affected: usize,
    pub total_propagation_time: Duration,
    pub updates_per_second: f32,
    pub cross_chunk_updates: usize,
}

/// Chunk light data for GPU processing
#[derive(Debug)]
pub struct ChunkLightData {
    pub chunk_pos: ChunkPos,
    pub light_data: Arc<RwLock<Vec<u8>>>, // Packed light data
    pub size: u32,
}

impl ChunkLightData {
    pub fn new(chunk_pos: ChunkPos, size: u32) -> Self {
        let total_size = (size * size * size) as usize;
        Self {
            chunk_pos,
            light_data: Arc::new(RwLock::new(vec![0; total_size])),
            size,
        }
    }
}

/// Thread-safe block data provider trait for lighting
pub trait BlockProvider: Send + Sync {
    fn get_block(&self, pos: VoxelPos) -> BlockId;
    fn is_transparent(&self, pos: VoxelPos) -> bool;
}
