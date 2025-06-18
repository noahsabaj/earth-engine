//! GPU buffer management and type system
//! 
//! This module provides a centralized, type-safe GPU buffer management system
//! with automatic WGSL alignment and compile-time validation.

pub mod buffer_manager;
pub mod types;
pub mod validation;
pub mod shader_bridge;
pub mod preprocessor;
pub mod shader_includes;
pub mod soa; // Pure Structure of Arrays implementation
pub mod constants; // Single source of truth for GPU constants
pub mod buffer_layouts; // Centralized buffer layout definitions
pub mod wgsl_generator; // Automatic WGSL generation from Rust types

// New automation system modules
pub mod automation; // Unified automation system entry point

pub use buffer_manager::{GpuBufferManager, GpuError};
pub use types::{GpuData, TypedGpuBuffer, terrain};
pub use validation::validate_all_gpu_types;
pub use preprocessor::{preprocess_shader, preprocess_shader_content, WgslPreprocessor};

// Re-export commonly used types
pub use types::terrain::{BlockDistribution, TerrainParams};

// Re-export SOA types for convenience
pub use soa::{SoaCompatible, BlockDistributionSOA, TerrainParamsSOA, SoaBufferBuilder, CpuGpuBridge};

// Re-export buffer layout types for convenience
pub use buffer_layouts::{
    VoxelData, InstanceData, CameraUniform, IndirectDrawCommand,
    bindings, calculations, constants as buffer_constants,
};

// Re-export automation types
pub use automation::{
    UnifiedGpuSystem, GpuTypeInfo, BindingAccess,
    TypedRenderPipelineBuilder, TypedComputePipelineBuilder, create_validated_shader,
};