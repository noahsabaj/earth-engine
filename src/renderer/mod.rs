pub mod allocation_optimizations;
// Removed: chunk_mesh_adapter (CPU mesh building)
// Removed: chunk_rendering (CPU chunk rendering)
mod compute_pipeline;
// Removed: data_mesh_builder (CPU mesh building)
pub mod error;
pub mod renderer_data;
pub mod renderer_operations;
pub mod gpu_culling;
mod gpu_diagnostics;
pub mod gpu_driven;
pub mod gpu_meshing;
mod gpu_progress;
mod gpu_recovery;
// mod gpu_state; // Removed - using DOP modules instead
pub mod gpu_state_data;
pub mod gpu_state_operations;
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
// Removed: simple_async_renderer (placeholder module)
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
    ObjectPool, PooledObject, StringPool, MESHING_BUFFERS,
};
pub use renderer_operations::with_meshing_buffers;
// CPU mesh generation exports removed - use GPU meshing instead
pub use compute_pipeline::{ComputePipelineManager, GpuMeshGenerator, MeshGenerationOutput};
pub use gpu_diagnostics::{
    DiagnosticsReport, GpuDiagnostics, OperationTestResult, ValidationResult,
};
pub use gpu_progress::{
    AsyncProgressReporter, GpuInitProgress, LogProgressCallback, ProgressCallback,
};
pub use gpu_recovery::{FallbackSettings, GpuHealthMonitor, GpuRecovery};
// pub use gpu_state::{CameraUniform, GpuState}; // Migrated to DOP modules
pub use gpu_state_data::{CameraUniform as CameraUniformData, GpuStateBuffers, MeshOffsetInfo};
pub use gpu_state_operations::*;
pub use mesh::ChunkMesh;
pub use mesh_optimizer::MeshLod;
pub use mesh_soa::{MeshSoA, MeshStats};
pub use selection_renderer::SelectionRenderer;
// Removed: SimpleAsyncRenderer (placeholder module)
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

    let result = pollster::block_on(gpu_state_operations::run_app(event_loop, config, game));

    match &result {
        Ok(_) => log::info!("[renderer::run] gpu_state_operations::run_app completed successfully"),
        Err(e) => log::error!("[renderer::run] gpu_state_operations::run_app failed: {}", e),
    }

    result
}

/// DOP version of run that accepts shared engine buffers
pub fn run_with_buffers<G: GameData + 'static>(
    event_loop: EventLoop<()>,
    config: EngineConfig,
    game: G,
    buffers: crate::SharedEngineBuffers,
) -> Result<()> {
    log::info!("[renderer::run_with_buffers] Starting renderer initialization with DOP buffers");
    log::debug!("[renderer::run_with_buffers] Config: {:?}", config);

    // Pass buffers to GPU state
    log::info!("[renderer::run_with_buffers] Calling pollster::block_on with gpu_state::run_app_with_buffers");

    let result = pollster::block_on(gpu_state_operations::run_app_with_buffers(event_loop, config, game, buffers));

    match &result {
        Ok(_) => log::info!("[renderer::run_with_buffers] gpu_state_operations::run_app_with_buffers completed successfully"),
        Err(e) => log::error!("[renderer::run_with_buffers] gpu_state_operations::run_app_with_buffers failed: {}", e),
    }

    result
}
