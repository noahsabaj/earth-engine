pub mod allocation_optimizations;
pub mod chunk_mesh_adapter;
pub mod chunk_rendering;
mod compute_pipeline;
pub mod data_mesh_builder;
pub mod error;
pub mod gpu_culling;
mod gpu_diagnostics;
pub mod gpu_driven;
pub mod gpu_meshing;
mod gpu_progress;
mod gpu_recovery;
mod gpu_state;
mod lod_transition;
mod mesh;
pub mod mesh_optimizer;
mod mesh_soa;
mod mesh_utils;
mod pipeline;
mod preallocated_mesh_cache;
mod preallocated_texture_atlas;
mod progressive_streaming;
mod selection_renderer;
mod simple_async_renderer;
mod soa_mesh_builder;
pub mod ui;
mod vertex;
mod vertex_soa;
mod zero_alloc_pools;

use crate::game::GameData;
use crate::EngineConfig;
use anyhow::Result;
use winit::event_loop::EventLoop;

pub use allocation_optimizations::{
    with_meshing_buffers, ObjectPool, PooledObject, StringPool, STRING_POOL,
};
pub use chunk_mesh_adapter::{
    build_chunk_mesh_dop, mesh_buffer_to_chunk_mesh, ChunkMeshBatch, NeighborData,
};
pub use chunk_rendering::{
    batch_chunks_to_render_objects, build_chunk_mesh_data, calculate_chunk_lod,
    chunk_bounding_radius, chunk_distance_squared, chunk_render_priority, chunk_to_render_object,
    chunk_world_position, filter_visible_chunks, is_chunk_in_frustum, ChunkRenderConfig,
};
pub use compute_pipeline::{ComputePipelineManager, GpuMeshGenerator, MeshGenerationOutput};
pub use data_mesh_builder::{MeshBuffer, MeshBufferPool, MeshMetadata, MESH_BUFFER_POOL};
pub use gpu_diagnostics::{
    DiagnosticsReport, GpuDiagnostics, OperationTestResult, ValidationResult,
};
pub use gpu_progress::{
    with_timeout, AsyncProgressReporter, GpuInitProgress, LogProgressCallback, ProgressCallback,
};
pub use gpu_recovery::{FallbackSettings, GpuHealthMonitor, GpuRecovery};
pub use gpu_state::{CameraUniform, GpuState};
pub use mesh::ChunkMesh;
pub use mesh_optimizer::MeshLod;
pub use mesh_soa::{MeshSoA, MeshStats};
pub use selection_renderer::SelectionRenderer;
pub use simple_async_renderer::SimpleAsyncRenderer;
pub use soa_mesh_builder::{GreedyMeshBuilderSoA, MeshBuilderSoA, MeshBuilderStats};
pub use vertex::{create_vertex, create_vertex_with_lighting, Vertex};
pub use vertex_soa::{VertexBufferSoA, VertexBufferStats};
pub use zero_alloc_pools::{
    GameDataPools, HashMapPool, PooledHashMap, PooledVector, VectorPool, GAME_POOLS,
};

pub struct Renderer {
    // Will be implemented
}

pub fn run<G: GameData + 'static>(
    event_loop: EventLoop<()>,
    config: EngineConfig,
    game: G,
) -> Result<()> {
    log::info!("[renderer::run] Starting renderer initialization");
    log::debug!("[renderer::run] Config: {:?}", config);

    // Will be implemented - this is the main entry point
    log::info!("[renderer::run] Calling pollster::block_on with gpu_state::run_app");

    let result = pollster::block_on(gpu_state::run_app(event_loop, config, game));

    match &result {
        Ok(_) => log::info!("[renderer::run] gpu_state::run_app completed successfully"),
        Err(e) => log::error!("[renderer::run] gpu_state::run_app failed: {}", e),
    }

    result
}
