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
pub mod culling_pipeline;
pub mod gpu_driven_renderer;
pub mod lod_system;
pub mod zero_alloc_gpu_renderer;

#[cfg(test)]
mod tests;

pub use indirect_commands::{IndirectDrawCommand, IndirectCommandBuffer, IndirectCommandManager, IndirectDrawIndexedCommand, DrawMetadata};
pub use instance_buffer::{InstanceData, InstanceBuffer, InstanceManager, CullingInstanceData};
pub use culling_pipeline::{CullingPipeline, CullingData};
pub use gpu_driven_renderer::{GpuDrivenRenderer, RenderStats, RenderObject};
pub use lod_system::{LodLevel, LodSystem};
pub use zero_alloc_gpu_renderer::{ZeroAllocRenderData, render_with_zero_allocations, render_with_pooled_collections};