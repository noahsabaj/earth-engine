//! GPU buffer management and type system
//! 
//! This module provides a centralized, type-safe GPU buffer management system
//! with automatic WGSL alignment and compile-time validation.

pub mod buffer_manager;
pub mod types;
pub mod validation;
pub mod shader_bridge;
pub mod preprocessor;

pub use buffer_manager::{GpuBufferManager, GpuError};
pub use types::{GpuData, TypedGpuBuffer, terrain};
pub use validation::validate_all_gpu_types;
pub use preprocessor::{preprocess_shader, preprocess_shader_content, WgslPreprocessor};

// Re-export commonly used types
pub use types::terrain::{BlockDistribution, TerrainParams};