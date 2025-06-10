mod chunk_renderer;
mod gpu_state;
mod mesh;
mod mesher;
mod pipeline;
mod selection_renderer;
mod parallel_chunk_renderer;
mod async_mesh_builder;
mod async_chunk_renderer;
pub mod ui;
mod vertex;
mod vertex_soa;
mod mesh_soa;
mod compute_pipeline;
pub mod gpu_driven;
pub mod gpu_culling;

use crate::{EngineConfig, Game};
use anyhow::Result;
use winit::event_loop::EventLoop;

pub use chunk_renderer::ChunkRenderer;
pub use gpu_state::GpuState;
pub use mesh::ChunkMesh;
pub use mesher::ChunkMesher;
pub use selection_renderer::SelectionRenderer;
pub use parallel_chunk_renderer::ParallelChunkRenderer;
pub use async_mesh_builder::{AsyncMeshBuilder, MeshBuildRequest, CompletedMesh, MeshBuildStats};
pub use async_chunk_renderer::{AsyncChunkRenderer, RenderStats};
pub use vertex::Vertex;
pub use vertex_soa::{VertexBufferSoA, VertexBufferStats};
pub use mesh_soa::{MeshSoA, MeshStats};
pub use compute_pipeline::{ComputePipelineManager, MeshGenerationOutput, GpuMeshGenerator};

pub struct Renderer {
    // Will be implemented
}

pub fn run<G: Game + 'static>(
    event_loop: EventLoop<()>,
    config: EngineConfig,
    game: G,
) -> Result<()> {
    // Will be implemented - this is the main entry point
    pollster::block_on(gpu_state::run_app(event_loop, config, game))
}