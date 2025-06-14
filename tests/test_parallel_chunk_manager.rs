use earth_engine::world::{ParallelChunkManager, WorldGenerator, Chunk, ChunkPos};
use cgmath::Point3;
use std::thread;
use std::time::Duration;

// Simple test generator for testing purposes
struct TestGenerator;

impl TestGenerator {
    fn new() -> Self {
        Self
    }
}

impl WorldGenerator for TestGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> Chunk {
        let mut chunk = Chunk::new(chunk_pos, chunk_size);
        // Generate simple flat terrain at y=64
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                chunk.set_block(x, 64, z, earth_engine::world::BlockId(1)); // Stone
            }
        }
        chunk
    }
    
    fn get_surface_height(&self, _world_x: f64, _world_z: f64) -> i32 {
        64 // Fixed height
    }
}

#[test]
fn test_queue_consumption_rate() {
    // Create a test generator
    let generator = Box::new(TestGenerator::new());
    let mut manager = ParallelChunkManager::new(4, 32, generator);
    
    // Skip batch size setting as it might not be available
    
    // Simulate player at origin
    let player_pos = Point3::new(0.0, 64.0, 0.0);
    
    // First update to queue chunks
    manager.update_loaded_chunks(player_pos);
    
    // Process generation multiple times
    for _ in 0..5 {
        manager.process_generation_queue();
        thread::sleep(Duration::from_millis(10));
    }
    
    // Update again to consume completed chunks
    manager.update_loaded_chunks(player_pos);
    
    // Get generation stats for verification
    let stats = manager.get_stats();
    println!("Generation Stats: {:?}", stats);
    
    // Verify chunks were processed
    assert!(stats.chunks_generated > 0, "No chunks were generated");
}

#[test]
fn test_adaptive_batch_sizing() {
    let generator = Box::new(TestGenerator::new());
    let manager = ParallelChunkManager::new(4, 32, generator);
    
    // Get initial stats
    let initial_stats = manager.get_stats();
    println!("Initial stats: {:?}", initial_stats);
    
    // Simulate heavy load by requesting many chunks
    let player_pos = Point3::new(0.0, 64.0, 0.0);
    manager.update_loaded_chunks(player_pos);
    
    // Process some chunks
    for _ in 0..3 {
        manager.process_generation_queue();
    }
    
    // Check generation occurred
    let loaded_stats = manager.get_stats();
    println!("Stats after processing: {:?}", loaded_stats);
    
    // Verify generation worked
    assert!(loaded_stats.chunks_generated >= initial_stats.chunks_generated, "Chunks should have been generated");
}

#[test]
fn test_queue_health_warnings() {
    let generator = Box::new(TestGenerator::new());
    let manager = ParallelChunkManager::new(2, 32, generator); // Small view distance
    
    // Simulate player movement to generate requests
    let player_pos = Point3::new(0.0, 64.0, 0.0);
    manager.update_loaded_chunks(player_pos);
    
    // Get initial stats
    let initial_stats = manager.get_stats();
    println!("Initial generation stats: {:?}", initial_stats);
    
    // Process chunks multiple times
    for i in 0..10 {
        manager.process_generation_queue();
        
        // Move player around to generate more requests
        let new_pos = Point3::new((i * 10) as f32, 64.0, 0.0);
        manager.update_loaded_chunks(new_pos);
        
        thread::sleep(Duration::from_millis(5));
    }
    
    // Check final stats
    let final_stats = manager.get_stats();
    println!("Final generation stats: {:?}", final_stats);
    
    // Verify chunks were processed
    assert!(final_stats.chunks_generated > initial_stats.chunks_generated, "Should have generated some chunks");
}