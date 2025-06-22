//! GPU automation system modules
//!
//! This module exports all the automated GPU systems that eliminate manual operations

pub mod auto_bindings;
pub mod auto_layout;
pub mod auto_wgsl;
pub mod bind_group_macros;
pub mod binding_manager;
pub mod layout_derive;
pub mod registry;
pub mod safe_pipeline;
pub mod shader_validator;
pub mod typed_bindings;
pub mod unified_system;

// Re-export main types
pub use auto_bindings::BindingUsage;
pub use registry::{
    create_gpu_shader, generate_all_gpu_types, generate_gpu_constants, generate_shader_bindings,
    initialize_gpu_registry,
};
pub use safe_pipeline::{
    create_validated_shader, TypedComputePipelineBuilder, TypedRenderPipelineBuilder,
};
pub use typed_bindings::BindingSlot;
pub use unified_system::{BindingAccess, GpuTypeInfo, UnifiedGpuSystem};
