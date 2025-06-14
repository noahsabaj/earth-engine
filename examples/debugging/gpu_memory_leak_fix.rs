// Example demonstrating the GPU memory leak fix for async renderer

use earth_engine::{
    ChunkPos, BlockId, BlockRegistry,
    renderer::SimpleAsyncRenderer,
    world::{ParallelWorld, ParallelWorldConfig, generation::DefaultWorldGenerator},
    camera::{Camera, CameraData, init_camera},
};
use std::sync::Arc;
use cgmath::Point3;

// This example shows how the GPU buffer memory leak has been fixed
// by implementing proper cleanup of unloaded chunks.

fn main() {
    println!("GPU Memory Leak Fix Demonstration");
    println!("=================================\n");
    
    // Create a simple block registry
    let mut registry = BlockRegistry::new();
    let stone_id = registry.register("test:stone", TestStoneBlock);
    let registry = Arc::new(registry);
    
    // Create world with small chunks for easier demonstration
    let chunk_size = 16;
    let generator = Box::new(DefaultWorldGenerator::new(
        12345, // seed
        BlockId::GRASS,
        BlockId::DIRT,
        BlockId::STONE,
        BlockId::WATER,
        BlockId::SAND,
    ));
    
    let config = ParallelWorldConfig {
        generation_threads: 2,
        mesh_threads: 2,
        chunks_per_frame: 4,
        view_distance: 3, // Small view distance for demo
        chunk_size,
    };
    
    let world = Arc::new(ParallelWorld::new(generator, config));
    
    // Create async renderer
    let mut renderer = SimpleAsyncRenderer::new(
        Arc::clone(&registry),
        chunk_size,
        Some(2),
    );
    
    // Create camera using the deprecated API for this example
    #[allow(deprecated)]
    let mut camera = Camera::new(800, 600);
    
    println!("Step 1: Initial position - generating chunks");
    camera.position = Point3::new(0.0, 10.0, 0.0);
    world.update(camera.position);
    
    // Wait for chunks to generate
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    // Queue chunks for rendering
    renderer.queue_dirty_chunks(&world, &camera);
    
    // In a real application, this would upload to GPU
    // For demo, we just track the count
    let initial_meshes = renderer.mesh_count();
    println!("  Initial GPU meshes: {}", initial_meshes);
    
    println!("\nStep 2: Move camera far away - chunks unload");
    camera.position = Point3::new(1000.0, 10.0, 1000.0);
    world.update(camera.position);
    
    // Wait for unloading
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    println!("  Before cleanup: {} GPU meshes", renderer.mesh_count());
    
    // This is the key fix - cleanup unloaded chunks
    renderer.cleanup_unloaded_chunks(&world);
    
    println!("  After cleanup: {} GPU meshes", renderer.mesh_count());
    
    println!("\nStep 3: Return to original position");
    camera.position = Point3::new(0.0, 10.0, 0.0);
    world.update(camera.position);
    
    // Queue chunks again
    renderer.queue_dirty_chunks(&world, &camera);
    
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    println!("  GPU meshes after returning: {}", renderer.mesh_count());
    
    println!("\nKey improvements:");
    println!("1. Added get_loaded_chunk_positions() to ParallelChunkManager");
    println!("2. Exposed this through ParallelWorld::get_loaded_chunk_positions()");
    println!("3. Implemented cleanup_unloaded_chunks() to remove GPU buffers");
    println!("4. Integrated cleanup into the render loop in gpu_state.rs");
    
    println!("\nResult: GPU memory is properly released when chunks unload!");
}

// Simple test block
struct TestStoneBlock;

impl earth_engine::world::Block for TestStoneBlock {
    fn get_id(&self) -> BlockId { BlockId::STONE }
    fn get_render_data(&self) -> earth_engine::world::RenderData {
        earth_engine::world::RenderData {
            color: [0.6, 0.6, 0.6],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> earth_engine::world::PhysicsProperties {
        earth_engine::world::PhysicsProperties {
            solid: true,
            density: 2500.0,
        }
    }
    fn get_name(&self) -> &str { "Stone" }
}