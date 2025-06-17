use hearth_engine::{
    BlockId, BlockRegistry, DefaultWorldGenerator, ChunkManager, ChunkLoadingStats,
};
use cgmath::Point3;
use std::time::{Duration, Instant};

fn main() {
    env_logger::init();
    
    println!("Chunk Loading Throttling Demo");
    println!("=============================\n");
    
    // Create a simple block registry
    let mut registry = BlockRegistry::new();
    let grass_id = registry.register_block("grass", hearth_engine::Block::default());
    let dirt_id = registry.register_block("dirt", hearth_engine::Block::default());
    let stone_id = registry.register_block("stone", hearth_engine::Block::default());
    let water_id = registry.register_block("water", hearth_engine::Block::default());
    let sand_id = registry.register_block("sand", hearth_engine::Block::default());
    
    // Create world generator
    let generator = Box::new(DefaultWorldGenerator::new(
        12345, // seed
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    // Create chunk manager with different configurations
    let view_distance = 8; // 8 chunks in each direction
    let chunk_size = 32;
    
    println!("Testing different chunk loading configurations:");
    println!("View distance: {} chunks", view_distance);
    println!("Total chunks in view: ~{}", (view_distance * 2 + 1).pow(3));
    println!();
    
    // Test 1: No throttling (all chunks at once)
    test_chunk_loading("No Throttling", view_distance, chunk_size, generator.clone(), None, false);
    
    // Test 2: Fixed throttling (5 chunks per frame)
    test_chunk_loading("Fixed Throttling (5 chunks/frame)", view_distance, chunk_size, generator.clone(), Some(5), false);
    
    // Test 3: Adaptive throttling
    test_chunk_loading("Adaptive Throttling", view_distance, chunk_size, generator, Some(5), true);
}

fn test_chunk_loading(
    test_name: &str,
    view_distance: i32,
    chunk_size: u32,
    generator: Box<dyn hearth_engine::WorldGenerator>,
    max_chunks_per_frame: Option<usize>,
    adaptive: bool,
) {
    println!("\n{}", test_name);
    println!("{}", "-".repeat(test_name.len()));
    
    let mut chunk_manager = ChunkManager::new(view_distance, chunk_size, generator);
    
    // Configure chunk loading
    if let Some(max) = max_chunks_per_frame {
        chunk_manager.set_max_chunks_per_frame(max);
    } else {
        chunk_manager.set_max_chunks_per_frame(1000); // Effectively no limit
    }
    chunk_manager.set_adaptive_loading(adaptive);
    
    // Simulate player at origin
    let player_pos = Point3::new(0.0, 64.0, 0.0);
    
    // Simulate multiple frames
    let start_time = Instant::now();
    let mut frame_count = 0;
    let target_frame_time = Duration::from_millis(16); // 60 FPS
    
    loop {
        let frame_start = Instant::now();
        
        // Update chunk loading
        chunk_manager.update_loaded_chunks(player_pos);
        
        // Get stats
        let stats = chunk_manager.get_loading_stats();
        
        // Simulate frame time
        let frame_duration = frame_start.elapsed();
        if frame_duration < target_frame_time {
            std::thread::sleep(target_frame_time - frame_duration);
        }
        
        frame_count += 1;
        
        // Print progress every 10 frames
        if frame_count % 10 == 0 {
            println!(
                "Frame {}: Loaded: {}, Pending: {}, In Generation: {}",
                frame_count,
                stats.loaded_chunks,
                stats.pending_chunks,
                stats.chunks_in_generation
            );
        }
        
        // Stop when all chunks are loaded
        if !chunk_manager.is_loading() {
            break;
        }
        
        // Safety limit
        if frame_count > 1000 {
            println!("Warning: Stopped after 1000 frames");
            break;
        }
    }
    
    let total_time = start_time.elapsed();
    let final_stats = chunk_manager.get_loading_stats();
    
    println!("\nResults:");
    println!("  Total frames: {}", frame_count);
    println!("  Total time: {:.2}s", total_time.as_secs_f32());
    println!("  Chunks loaded: {}", final_stats.loaded_chunks);
    println!("  Average FPS: {:.1}", frame_count as f32 / total_time.as_secs_f32());
}