use earth_engine::world::parallel_chunk_manager::{ParallelChunkManager, QueueMetrics};
use earth_engine::world::generation::TestGenerator;
use earth_engine::ChunkPos;
use cgmath::Point3;
use std::thread;
use std::time::Duration;

#[test]
fn test_queue_consumption_rate() {
    // Create a test generator
    let generator = Box::new(TestGenerator::new());
    let mut manager = ParallelChunkManager::new(4, 32, generator);
    
    // Set a reasonable batch size
    manager.set_batch_size(4);
    
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
    
    // Check queue metrics
    let metrics = manager.get_queue_metrics();
    println!("Queue Metrics: {:?}", metrics);
    
    // Log comprehensive stats
    manager.log_queue_stats();
    
    // Verify chunks were loaded
    assert!(manager.loaded_chunk_count() > 0, "No chunks were loaded");
    
    // Verify completed queue is being consumed
    assert!(metrics.completed_queue_length < metrics.max_queue_size / 2, 
            "Completed queue is backing up: {} / {}", 
            metrics.completed_queue_length, metrics.max_queue_size);
}

#[test]
fn test_adaptive_batch_sizing() {
    let generator = Box::new(TestGenerator::new());
    let manager = ParallelChunkManager::new(4, 32, generator);
    
    // Get initial metrics
    let initial_metrics = manager.get_queue_metrics();
    println!("Initial batch size: {}", initial_metrics.current_batch_size);
    
    // Simulate heavy load by pre-generating many chunks
    let center = ChunkPos::new(0, 0, 0);
    manager.pregenerate_chunks(center, 5);
    
    // Process some chunks
    for _ in 0..3 {
        manager.process_generation_queue();
    }
    
    // Check if batch size adapts
    let loaded_metrics = manager.get_queue_metrics();
    println!("Dynamic batch size under load: {}", loaded_metrics.dynamic_batch_size);
    
    // Dynamic batch size should adjust based on queue depth
    assert!(loaded_metrics.dynamic_batch_size > 0, "Dynamic batch size should be positive");
}

#[test]
fn test_queue_health_warnings() {
    let generator = Box::new(TestGenerator::new());
    let manager = ParallelChunkManager::new(2, 32, generator); // Small view distance
    
    // Generate a lot of chunk requests to fill the queue
    for i in -10..=10 {
        for j in -10..=10 {
            for k in -10..=10 {
                manager.queue_chunk_generation(ChunkPos::new(i, j, k), i.abs() + j.abs() + k.abs());
            }
        }
    }
    
    // Check queue health - should trigger warnings
    let metrics = manager.get_queue_metrics();
    println!("Queue usage after heavy load - Gen: {:.1}%, Comp: {:.1}%", 
             metrics.generation_queue_usage, metrics.completed_queue_usage);
    
    // Process chunks to clear the queue
    for _ in 0..20 {
        manager.process_generation_queue();
        manager.update_loaded_chunks(Point3::new(0.0, 0.0, 0.0));
    }
}