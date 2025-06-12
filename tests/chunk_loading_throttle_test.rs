use earth_engine::{
    BlockId, BlockRegistry, DefaultWorldGenerator, ChunkManager, ParallelChunkManager,
};
use cgmath::Point3;
use std::time::Instant;

#[test]
fn test_chunk_loading_throttling() {
    // Create a simple block registry
    let mut registry = BlockRegistry::new();
    let grass_id = registry.register_block("grass", earth_engine::Block::default());
    let dirt_id = registry.register_block("dirt", earth_engine::Block::default());
    let stone_id = registry.register_block("stone", earth_engine::Block::default());
    let water_id = registry.register_block("water", earth_engine::Block::default());
    let sand_id = registry.register_block("sand", earth_engine::Block::default());
    
    // Create world generator
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    let view_distance = 3; // Small for testing
    let chunk_size = 16;
    
    let mut chunk_manager = ChunkManager::new(view_distance, chunk_size, generator);
    chunk_manager.set_max_chunks_per_frame(2); // Only load 2 chunks per frame
    
    let player_pos = Point3::new(0.0, 64.0, 0.0);
    
    // First update should queue chunks but only load 2
    chunk_manager.update_loaded_chunks(player_pos);
    let stats1 = chunk_manager.get_loading_stats();
    assert!(stats1.loaded_chunks <= 2, "Should load at most 2 chunks in first frame");
    assert!(stats1.pending_chunks > 0, "Should have pending chunks");
    
    // Continue loading
    let mut frame_count = 0;
    while chunk_manager.is_loading() && frame_count < 100 {
        chunk_manager.update_loaded_chunks(player_pos);
        frame_count += 1;
    }
    
    let final_stats = chunk_manager.get_loading_stats();
    assert!(final_stats.pending_chunks == 0, "All chunks should be loaded");
    assert!(final_stats.loaded_chunks > 0, "Should have loaded chunks");
    
    println!("Loaded {} chunks in {} frames", final_stats.loaded_chunks, frame_count);
}

#[test]
fn test_parallel_chunk_manager_throttling() {
    // Create world generator
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        BlockId::new(1),
        BlockId::new(2),
        BlockId::new(3),
        BlockId::new(4),
        BlockId::new(5),
    ));
    
    let view_distance = 4;
    let chunk_size = 16;
    
    let mut chunk_manager = ParallelChunkManager::new(view_distance, chunk_size, generator);
    chunk_manager.set_batch_size(3); // Process 3 chunks per batch
    
    let player_pos = Point3::new(0.0, 64.0, 0.0);
    
    // Update and process chunks
    let start = Instant::now();
    chunk_manager.update_loaded_chunks(player_pos);
    chunk_manager.process_generation_queue();
    
    // Check that processing is throttled
    let queue_len = chunk_manager.get_queue_length();
    println!("Queue length after first batch: {}", queue_len);
    
    // Continue processing
    let mut iterations = 0;
    while chunk_manager.get_queue_length() > 0 && iterations < 50 {
        chunk_manager.process_generation_queue();
        chunk_manager.update_loaded_chunks(player_pos);
        iterations += 1;
    }
    
    let elapsed = start.elapsed();
    let stats = chunk_manager.get_stats();
    
    println!("Parallel chunk loading stats:");
    println!("  Chunks generated: {}", stats.chunks_generated);
    println!("  Total time: {:.2}s", elapsed.as_secs_f32());
    println!("  Chunks per second: {:.1}", stats.chunks_per_second);
    println!("  Average chunk time: {:?}", stats.average_chunk_time);
    
    assert!(stats.chunks_generated > 0, "Should have generated chunks");
}

#[test]
fn test_chunk_priority_loading() {
    // Create world generator
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        BlockId::new(1),
        BlockId::new(2),
        BlockId::new(3),
        BlockId::new(4),
        BlockId::new(5),
    ));
    
    let view_distance = 5;
    let chunk_size = 16;
    
    let mut chunk_manager = ChunkManager::new(view_distance, chunk_size, generator);
    chunk_manager.set_max_chunks_per_frame(1); // Load one chunk at a time to test priority
    
    let player_pos = Point3::new(0.0, 64.0, 0.0);
    
    // Track the order of chunk loading
    let mut loaded_chunks = Vec::new();
    let player_chunk_pos = earth_engine::ChunkPos::new(0, 4, 0); // Y=4 for height 64
    
    // Load chunks one by one
    while chunk_manager.is_loading() && loaded_chunks.len() < 10 {
        chunk_manager.update_loaded_chunks(player_pos);
        
        // Check which chunks are loaded
        for x in -view_distance..=view_distance {
            for y in -view_distance..=view_distance {
                for z in -view_distance..=view_distance {
                    let chunk_pos = earth_engine::ChunkPos::new(x, y + 4, z);
                    if chunk_manager.get_chunk(chunk_pos).is_some() && !loaded_chunks.contains(&chunk_pos) {
                        loaded_chunks.push(chunk_pos);
                        let distance = chunk_pos.distance_squared_to(player_chunk_pos);
                        println!("Loaded chunk at {:?} with distance² = {}", chunk_pos, distance);
                    }
                }
            }
        }
    }
    
    // Verify that closer chunks were loaded first
    for i in 1..loaded_chunks.len().min(5) {
        let dist_prev = loaded_chunks[i-1].distance_squared_to(player_chunk_pos);
        let dist_curr = loaded_chunks[i].distance_squared_to(player_chunk_pos);
        assert!(
            dist_prev <= dist_curr,
            "Chunks should be loaded in order of distance (closer first)"
        );
    }
}