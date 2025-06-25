// Data and operations modules (DOP style)
pub mod gpu_driven_renderer_data;
pub mod gpu_driven_renderer_operations;

/// GPU-driven rendering system
///
/// This module implements a modern GPU-driven rendering pipeline where
/// the GPU decides what to draw using indirect draw commands.
///
/// Key components:
/// - Indirect draw commands generated on GPU
/// - Instance data stored in GPU buffers
/// - Compute shader culling
/// - Multi-threaded command generation
/// - Zero CPU draw calls per frame
///
/// This module follows Data-Oriented Programming (DOP) principles:
/// - All data structures are in gpu_driven_renderer_data.rs
/// - All operations are pure functions in gpu_driven_renderer_operations.rs
/// - ZERO self references throughout the module
// pub mod zero_alloc_gpu_renderer;  // Removed - using DOP modules instead
pub mod zero_alloc_gpu_renderer_data;
pub mod zero_alloc_gpu_renderer_operations;

#[cfg(test)]
mod tests;

// Re-export data structures
pub use gpu_driven_renderer_data::{
    GpuDrivenRendererData, RenderStats, RenderObject, GpuDrivenFrameState,
    GpuBufferRefs, RenderConfig, FrameCounter,
    // Culling types
    CameraData, DrawCount, CullingStats, CullingPipelineData, CullingMetadataData,
    // Indirect command types
    IndirectCommandBufferData, IndirectCommandManagerData,
    // LOD types
    LodLevel, LodConfig, LodSystemData, LodSelection, LodTransition,
    // Instance types
    InstanceBufferData, InstanceData, InstanceManagerData,
};

// Re-export all operations
pub use gpu_driven_renderer_operations::*;

// Re-export types from gpu buffer layouts
pub use crate::gpu::buffer_layouts::{
    DrawMetadata, IndirectDrawCommand, IndirectDrawIndexedCommand,
};
// Zero-allocation renderer modules
pub use zero_alloc_gpu_renderer_data::ZeroAllocRenderData;
pub use zero_alloc_gpu_renderer_operations::{render_with_zero_allocations, render_with_pooled_collections};
pub use zero_alloc_gpu_renderer_data::*;
pub use zero_alloc_gpu_renderer_operations::*;
