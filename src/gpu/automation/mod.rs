//! GPU automation system modules
//! 
//! This module exports all the automated GPU systems that eliminate manual operations

pub mod auto_wgsl;
pub mod binding_manager;
pub mod typed_bindings;
pub mod auto_bindings;
pub mod shader_validator;
pub mod safe_pipeline;
pub mod auto_layout;
pub mod layout_derive;
pub mod unified_system;
pub mod registry;
pub mod bind_group_macros;

// Re-export main types
pub use unified_system::{UnifiedGpuSystem, GpuTypeInfo, BindingAccess};
pub use safe_pipeline::{TypedRenderPipelineBuilder, TypedComputePipelineBuilder, create_validated_shader};
pub use auto_bindings::BindingUsage;
pub use typed_bindings::BindingSlot;
pub use registry::{
    initialize_gpu_registry, generate_all_gpu_types, 
    generate_shader_bindings, generate_gpu_constants, create_gpu_shader,
};