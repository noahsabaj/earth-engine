pub mod light_map;
pub mod propagation;
pub mod skylight;
pub mod time_of_day;
pub mod parallel_propagator;
pub mod concurrent_provider;

pub use light_map::{LightMap, LightLevel};
pub use propagation::LightPropagator;
pub use skylight::SkylightCalculator;
pub use time_of_day::{TimeOfDay, DayNightCycle};
pub use parallel_propagator::{
    ParallelLightPropagator, LightUpdate, ChunkLightData, 
    LightingStats, BatchLightCalculator, BlockProvider
};
pub use concurrent_provider::{
    ConcurrentBlockProvider, ParallelBlockProvider, TestBlockProvider
};

/// Maximum light level (full brightness)
pub const MAX_LIGHT_LEVEL: u8 = 15;

/// Minimum light level (complete darkness)
pub const MIN_LIGHT_LEVEL: u8 = 0;

/// Light falloff per block
pub const LIGHT_FALLOFF: u8 = 1;

/// Types of light in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    /// Sunlight/skylight that comes from above
    Sky,
    /// Block light from torches, lava, etc.
    Block,
}