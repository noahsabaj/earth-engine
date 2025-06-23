//! Pure Structure of Arrays (SOA) GPU architecture implementation
//!
//! This module provides a data-oriented approach to GPU buffer management,
//! optimizing for cache efficiency and memory bandwidth by storing data
//! in structure-of-arrays format rather than array-of-structures.

pub mod benchmarks;
pub mod bridge;
pub mod builders;
pub mod compatibility;
pub mod layouts;
pub mod types;

pub use benchmarks::{SoaBenchmarkReport, SoaBenchmarkResults, SoaBenchmarkSuite};
pub use bridge::CpuGpuBridge;
pub use builders::SoaBufferBuilder;
pub use compatibility::{BufferLayoutPreference, SoaMigrationHelper, UnifiedGpuBuffer};
pub use layouts::{AccessPattern, SoaLayoutManager};
pub use types::{BlockDistributionSOA, SoaCompatible, TerrainParamsSOA};

// Re-export for convenience
pub use crate::constants::core::MAX_BLOCK_DISTRIBUTIONS;
