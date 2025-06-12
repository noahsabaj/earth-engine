mod chunk_renderer;
mod gpu_state;
mod gpu_diagnostics;
mod gpu_progress;
mod gpu_recovery;
mod mesh;
mod mesher;
mod pipeline;
mod selection_renderer;
mod parallel_chunk_renderer;
mod async_mesh_builder;
mod async_chunk_renderer;
mod world_adapter;
mod world_wrapper;
mod simple_async_renderer;
pub mod ui;
mod vertex;
mod vertex_soa;
mod mesh_soa;
mod compute_pipeline;
pub mod gpu_driven;
pub mod gpu_culling;
mod greedy_mesher;
mod optimized_greedy_mesher;
mod mesh_optimizer;
mod allocation_optimizations;
pub mod data_mesh_builder;
pub mod error;

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
pub use simple_async_renderer::SimpleAsyncRenderer;
pub use vertex::Vertex;
pub use vertex_soa::{VertexBufferSoA, VertexBufferStats};
pub use mesh_soa::{MeshSoA, MeshStats};
pub use compute_pipeline::{ComputePipelineManager, MeshGenerationOutput, GpuMeshGenerator};
pub use greedy_mesher::{GreedyMesher, GreedyMeshStats, GreedyQuad, FaceDirection};
pub use optimized_greedy_mesher::{OptimizedGreedyMesher, MeshGenerationPool};
pub use allocation_optimizations::{ObjectPool, PooledObject, StringPool, with_meshing_buffers, STRING_POOL};
pub use mesh_optimizer::{MeshOptimizer, MeshLod, OptimizedMesh, CacheStats};
pub use data_mesh_builder::{MeshBuffer, MeshBufferPool, MeshMetadata, MESH_BUFFER_POOL};
pub use gpu_diagnostics::{GpuDiagnostics, DiagnosticsReport, ValidationResult, OperationTestResult};
pub use gpu_progress::{GpuInitProgress, AsyncProgressReporter, with_timeout, ProgressCallback, LogProgressCallback};
pub use gpu_recovery::{GpuRecovery, FallbackSettings, GpuHealthMonitor};

pub struct Renderer {
    // Will be implemented
}

pub fn run<G: Game + 'static>(
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