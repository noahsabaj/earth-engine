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

pub use indirect_commands::{IndirectDrawCommand, IndirectCommandBuffer};
pub use instance_buffer::{InstanceData, InstanceBuffer};
pub use culling_pipeline::{CullingPipeline, CullingData};
pub use gpu_driven_renderer::{GpuDrivenRenderer, RenderStats};
pub use lod_system::{LodLevel, LodSystem};