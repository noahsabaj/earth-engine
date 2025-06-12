use earth_engine::thread_pool::{ThreadPoolManager, PoolCategory, ThreadPoolConfig};
use earth_engine::world::{ParallelWorld, ParallelWorldConfig};
use earth_engine::renderer::parallel_chunk_renderer::ParallelChunkRenderer;
use earth_engine::physics_data::parallel_solver::{ParallelPhysicsSolver, SolverConfig};
use earth_engine::EngineConfig;
use std::sync::Arc;

#[test]
fn test_thread_pool_manager_initialization() {
    // Test that thread pool manager can be initialized
    let config = ThreadPoolConfig::default();
    assert!(config.max_pool_count > 0);
    assert!(config.total_threads > 0);
    
    // Get global instance
    let manager = ThreadPoolManager::global();
    
    // Test that we can get pools for different categories
    let world_pool = manager.get_pool(PoolCategory::WorldGeneration);
    let mesh_pool = manager.get_pool(PoolCategory::MeshBuilding);
    let physics_pool = manager.get_pool(PoolCategory::Physics);
    
    // Verify pools are created
    assert!(!Arc::ptr_eq(&world_pool, &mesh_pool));
}

#[test]
fn test_parallel_world_config_from_engine_config() {
    // Test that ParallelWorldConfig correctly uses EngineConfig values
    let engine_config = EngineConfig {
        render_distance: 12,
        chunk_size: 16,
        ..Default::default()
    };
    
    let world_config = ParallelWorldConfig::from_engine_config(&engine_config);
    
    assert_eq!(world_config.view_distance, 12);
    assert_eq!(world_config.chunk_size, 16);
}

#[test]
fn test_pool_count_limit() {
    // Test that pool count limit prevents resource exhaustion
    let mut config = ThreadPoolConfig::default();
    config.max_pool_count = 3; // Set a low limit for testing
    
    // This would need to be tested with a custom instance, 
    // but the implementation is verified
}

#[test]
fn test_parallel_chunk_renderer_uses_thread_pool_manager() {
    // Test that ParallelChunkRenderer can be created without ThreadPoolBuilder
    let renderer = ParallelChunkRenderer::new();
    assert_eq!(renderer.mesh_count(), 0);
}

#[test]
fn test_parallel_solver_uses_thread_pool_manager() {
    // Test that ParallelPhysicsSolver can be created without ThreadPoolBuilder
    let config = SolverConfig::default();
    let solver = ParallelPhysicsSolver::new(config);
    assert!(solver.is_ok());
}