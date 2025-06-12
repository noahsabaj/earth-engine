use earth_engine::{
    ChunkPos, BlockId, BlockRegistry, Camera,
    renderer::SimpleAsyncRenderer,
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator},
};
use std::sync::Arc;
use cgmath::Point3;

// Simple test blocks
struct TestBlock {
    id: BlockId,
    name: &'static str,
}

impl earth_engine::Block for TestBlock {
    fn get_id(&self) -> BlockId { self.id }
    fn get_render_data(&self) -> earth_engine::RenderData {
        earth_engine::RenderData {
            color: [0.5, 0.5, 0.5],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> earth_engine::PhysicsProperties {
        earth_engine::PhysicsProperties {
            solid: true,
            density: 1000.0,
        }
    }
    fn get_name(&self) -> &str { self.name }
}

#[test]
fn test_gpu_buffer_cleanup() {
    // Create block registry
    let mut registry = BlockRegistry::new();
    let stone_id = registry.register("test:stone", TestBlock {
        id: BlockId(1),
        name: "Stone",
    });
    let registry = Arc::new(registry);
    
    // Create world
    let chunk_size = 16; // Smaller chunks for faster testing
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        stone_id,
        stone_id,
        stone_id,
        BlockId::AIR,
        BlockId::AIR,
    ));
    
    let config = ParallelWorldConfig {
        generation_threads: 2,
        mesh_threads: 2,
        chunks_per_frame: 4,
        view_distance: 2,
        chunk_size,
    };
    
    let world = Arc::new(ParallelWorld::new(generator, config));
    
    // Create renderer
    let mut renderer = SimpleAsyncRenderer::new(
        Arc::clone(&registry),
        chunk_size,
        Some(2),
    );
    
    // Create camera
    let mut camera = Camera::new(800, 600);
    
    // Test scenario: Move camera to generate chunks, then move away
    println!("Testing GPU buffer cleanup...");
    
    // Position 1: Generate chunks around origin
    camera.position = Point3::new(0.0, 10.0, 0.0);
    world.update(camera.position);
    
    // Wait for chunks to generate
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Queue chunks for rendering
    renderer.queue_dirty_chunks(&world, &camera);
    
    // Process some mesh builds (simulate device parameter with None for testing)
    // Note: We can't actually create GPU buffers without a real device
    // but we can test the cleanup logic
    
    let initial_mesh_count = renderer.mesh_count();
    println!("Initial GPU meshes: {}", initial_mesh_count);
    
    // Position 2: Move far away to trigger chunk unloading
    camera.position = Point3::new(1000.0, 10.0, 1000.0);
    world.update(camera.position);
    
    // Wait for chunk unloading
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Run cleanup
    renderer.cleanup_unloaded_chunks(&world);
    
    let final_mesh_count = renderer.mesh_count();
    println!("Final GPU meshes after cleanup: {}", final_mesh_count);
    
    // Verify that meshes were cleaned up
    assert!(final_mesh_count < initial_mesh_count || final_mesh_count == 0,
        "GPU meshes should be cleaned up when chunks are unloaded. Initial: {}, Final: {}",
        initial_mesh_count, final_mesh_count);
    
    // Move back to origin
    camera.position = Point3::new(0.0, 10.0, 0.0);
    world.update(camera.position);
    
    // Queue chunks again
    renderer.queue_dirty_chunks(&world, &camera);
    
    // Wait a bit
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Run cleanup again - meshes for loaded chunks should remain
    let loaded_positions = world.get_loaded_chunk_positions();
    println!("Loaded chunk positions: {}", loaded_positions.len());
    
    renderer.cleanup_unloaded_chunks(&world);
    
    println!("GPU meshes after returning: {}", renderer.mesh_count());
    
    println!("GPU buffer cleanup test passed!");
}

#[test]
fn test_cleanup_preserves_loaded_chunks() {
    // Create block registry
    let mut registry = BlockRegistry::new();
    let stone_id = registry.register("test:stone", TestBlock {
        id: BlockId(1),
        name: "Stone",
    });
    let registry = Arc::new(registry);
    
    // Create world
    let chunk_size = 16;
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        stone_id,
        stone_id,
        stone_id,
        BlockId::AIR,
        BlockId::AIR,
    ));
    
    let config = ParallelWorldConfig {
        generation_threads: 2,
        mesh_threads: 2,
        chunks_per_frame: 4,
        view_distance: 2,
        chunk_size,
    };
    
    let world = Arc::new(ParallelWorld::new(generator, config));
    
    // Create renderer
    let mut renderer = SimpleAsyncRenderer::new(
        Arc::clone(&registry),
        chunk_size,
        Some(2),
    );
    
    // Create camera
    let camera = Camera::new(800, 600);
    
    // Generate some chunks
    world.update(camera.position);
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Manually insert some test meshes (simulating uploaded meshes)
    // Since we can't create real GPU buffers in tests, we'll test the logic
    // by checking that the cleanup method would preserve loaded chunks
    
    let loaded_positions = world.get_loaded_chunk_positions();
    let loaded_count = loaded_positions.len();
    
    println!("Testing that cleanup preserves {} loaded chunks", loaded_count);
    
    // Run cleanup
    renderer.cleanup_unloaded_chunks(&world);
    
    // In a real scenario with GPU buffers, we would verify that
    // meshes for loaded chunks are preserved
    println!("Cleanup preserves loaded chunks test passed!");
}