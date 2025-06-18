//! Pure Structure of Arrays (SOA) GPU architecture implementation
//! 
//! This module provides a data-oriented approach to GPU buffer management,
//! optimizing for cache efficiency and memory bandwidth by storing data
//! in structure-of-arrays format rather than array-of-structures.

pub mod types;
pub mod layouts;
pub mod builders;
pub mod bridge;
pub mod compatibility;
pub mod benchmarks;

pub use types::{SoaCompatible, BlockDistributionSOA, TerrainParamsSOA};
pub use layouts::{SoaLayoutManager, AccessPattern};
pub use builders::SoaBufferBuilder;
pub use bridge::CpuGpuBridge;
pub use compatibility::{UnifiedGpuBuffer, BufferLayoutPreference, SoaMigrationHelper};
pub use benchmarks::{SoaBenchmarkSuite, SoaBenchmarkResults, SoaBenchmarkReport};

// Re-export for convenience
pub use crate::gpu::constants::MAX_BLOCK_DISTRIBUTIONS;