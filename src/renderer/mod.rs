mod gpu_state;
mod gpu_diagnostics;
mod gpu_progress;
mod gpu_recovery;
mod mesh;
mod pipeline;
mod selection_renderer;
pub mod ui;
mod vertex;
mod vertex_soa;
mod mesh_soa;
mod soa_mesh_builder;
mod compute_pipeline;
pub mod gpu_driven;
pub mod gpu_culling;
pub mod allocation_optimizations;
mod zero_alloc_pools;
pub mod data_mesh_builder;
pub mod error;
mod simple_async_renderer;
pub mod mesh_optimizer;
mod preallocated_mesh_cache;
mod preallocated_texture_atlas;
mod lod_transition;
mod progressive_streaming;
pub mod chunk_mesh_adapter;
pub mod chunk_rendering;
pub mod gpu_meshing;

use crate::{EngineConfig};
use crate::game::GameData;
use anyhow::Result;
use winit::event_loop::EventLoop;

pub use gpu_state::{GpuState, CameraUniform};
pub use mesh::ChunkMesh;
pub use selection_renderer::SelectionRenderer;
pub use vertex::{Vertex, create_vertex, create_vertex_with_lighting};
pub use vertex_soa::{VertexBufferSoA, VertexBufferStats};
pub use mesh_soa::{MeshSoA, MeshStats};
pub use soa_mesh_builder::{MeshBuilderSoA, GreedyMeshBuilderSoA, MeshBuilderStats};
pub use compute_pipeline::{ComputePipelineManager, MeshGenerationOutput, GpuMeshGenerator};
pub use allocation_optimizations::{ObjectPool, PooledObject, StringPool, with_meshing_buffers, STRING_POOL};
pub use zero_alloc_pools::{VectorPool, HashMapPool, PooledVector, PooledHashMap, GameDataPools, GAME_POOLS};
pub use data_mesh_builder::{MeshBuffer, MeshBufferPool, MeshMetadata, MESH_BUFFER_POOL};
pub use gpu_diagnostics::{GpuDiagnostics, DiagnosticsReport, ValidationResult, OperationTestResult};
pub use gpu_progress::{GpuInitProgress, AsyncProgressReporter, with_timeout, ProgressCallback, LogProgressCallback};
pub use gpu_recovery::{GpuRecovery, FallbackSettings, GpuHealthMonitor};
pub use simple_async_renderer::SimpleAsyncRenderer;
pub use mesh_optimizer::MeshLod;
pub use chunk_mesh_adapter::{
    build_chunk_mesh_dop, mesh_buffer_to_chunk_mesh, 
    NeighborData, ChunkMeshBatch
};
pub use chunk_rendering::{
    ChunkRenderConfig, build_chunk_mesh_data, chunk_to_render_object,
    chunk_world_position, chunk_bounding_radius, chunk_distance_squared,
    calculate_chunk_lod, is_chunk_in_frustum, chunk_render_priority,
    batch_chunks_to_render_objects, filter_visible_chunks
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