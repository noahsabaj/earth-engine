//! GPU buffer management and type system
//!
//! This module provides a centralized, type-safe GPU buffer management system
//! with automatic WGSL alignment and compile-time validation.

pub mod buffer_manager;
pub mod preprocessor;
pub mod shader_bridge;
pub mod shader_includes;
pub mod soa;
pub mod types;
pub mod validation; // Pure Structure of Arrays implementation
                    // Constants are now in the root constants.rs file
pub mod buffer_layouts; // Centralized buffer layout definitions
pub mod error_recovery;
pub mod wgsl_generator; // Automatic WGSL generation from Rust types // GPU error recovery and prevention

// New automation system modules
pub mod automation; // Unified automation system entry point

pub use buffer_manager::{GpuBufferManager, GpuError};
pub use preprocessor::{preprocess_shader, preprocess_shader_content, WgslPreprocessor};
pub use types::{terrain, GpuData, TypedGpuBuffer};
pub use validation::validate_all_gpu_types;

// Re-export commonly used types
pub use types::terrain::{BlockDistribution, TerrainParams};

// Re-export SOA types for convenience
pub use soa::{
    BlockDistributionSOA, CpuGpuBridge, SoaBufferBuilder, SoaCompatible, TerrainParamsSOA,
};

// Re-export buffer layout types for convenience
pub use buffer_layouts::{
    bindings, calculations, CameraUniform, IndirectDrawCommand,
    InstanceData, VoxelData,
};

// Re-export automation types
pub use automation::{
    create_validated_shader, BindingAccess, GpuTypeInfo, TypedComputePipelineBuilder,
    TypedRenderPipelineBuilder, UnifiedGpuSystem,
};

// Re-export error recovery types
pub use error_recovery::{GpuErrorRecovery, GpuRecoveryError, GpuResultExt, SafeCommandEncoder};
