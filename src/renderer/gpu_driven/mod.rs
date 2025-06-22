pub mod culling_pipeline;
pub mod gpu_driven_renderer;
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
pub mod indirect_commands;
pub mod instance_buffer;
pub mod lod_system;
// pub mod zero_alloc_gpu_renderer;  // Temporarily disabled due to compilation issues

#[cfg(test)]
mod tests;

pub use culling_pipeline::{CullingData, CullingPipeline};
pub use gpu_driven_renderer::{GpuDrivenRenderer, RenderObject, RenderStats};
pub use indirect_commands::{
    DrawMetadata, IndirectCommandBuffer, IndirectCommandManager, IndirectDrawCommand,
    IndirectDrawIndexedCommand,
};
pub use instance_buffer::{CullingInstanceData, InstanceBuffer, InstanceData, InstanceManager};
pub use lod_system::{LodLevel, LodSystem};
// pub use zero_alloc_gpu_renderer::{ZeroAllocRenderData, render_with_zero_allocations, render_with_pooled_collections};
