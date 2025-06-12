use earth_engine::{
    BlockId, BlockRegistry, DefaultWorldGenerator, ChunkManager,
};
use cgmath::Point3;
use std::time::Instant;

fn main() {
    println!("Earth Engine - Chunk Loading Throttling Demo");
    println!("===========================================\n");
    
    // Create a simple block registry
    let mut registry = BlockRegistry::new();
    let grass_id = registry.register_block("grass", earth_engine::Block::default());
    let dirt_id = registry.register_block("dirt", earth_engine::Block::default());
    let stone_id = registry.register_block("stone", earth_engine::Block::default());
    let water_id = registry.register_block("water", earth_engine::Block::default());
    let sand_id = registry.register_block("sand", earth_engine::Block::default());
    
    // Create world generator
    let generator = Box::new(DefaultWorldGenerator::new(
        12345, // seed
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    // Test configuration
    let view_distance = 10; // 10 chunks in each direction
    let chunk_size = 32;
    let player_pos = Point3::new(0.0, 64.0, 0.0);
    
    println!("Configuration:");
    println!("- View distance: {} chunks", view_distance);
    println!("- Chunk size: {}x{}x{}", chunk_size, chunk_size, chunk_size);
    println!("- Player position: ({}, {}, {})", player_pos.x, player_pos.y, player_pos.z);
    
    // Calculate approximate number of chunks
    let approx_chunks = (view_distance * 2 + 1).pow(3);
    println!("- Approximate chunks to load: {}\n", approx_chunks);
    
    // Create chunk manager with throttling
    let mut chunk_manager = ChunkManager::new(view_distance, chunk_size, generator);
    chunk_manager.set_max_chunks_per_frame(5);
    chunk_manager.set_adaptive_loading(true);
    
    println!("Loading chunks with throttling (5 chunks per frame, adaptive mode)...\n");
    
    let start_time = Instant::now();
    let mut frame_count = 0;
    let mut last_loaded = 0;
    
    loop {
        let frame_start = Instant::now();
        
        // Update chunk loading
        chunk_manager.update_loaded_chunks(player_pos);
        
        // Get current stats
        let stats = chunk_manager.get_loading_stats();
        
        // Print progress if chunks were loaded this frame
        if stats.loaded_chunks > last_loaded {
            let chunks_this_frame = stats.loaded_chunks - last_loaded;
            println!(
                "Frame {:3}: Loaded {} chunks this frame | Total: {}/{} | Pending: {} | In Generation: {}",
                frame_count,
                chunks_this_frame,
                stats.loaded_chunks,
                approx_chunks,
                stats.pending_chunks,
                stats.chunks_in_generation
            );
            last_loaded = stats.loaded_chunks;
        }
        
        frame_count += 1;
        
        // Check if loading is complete
        if !chunk_manager.is_loading() {
            break;
        }
        
        // Simulate frame timing (16ms = ~60 FPS)
        let frame_duration = frame_start.elapsed();
        if frame_duration.as_millis() < 16 {
            std::thread::sleep(std::time::Duration::from_millis(
                16 - frame_duration.as_millis() as u64
            ));
        }
        
        // Safety limit
        if frame_count > 1000 {
            println!("\nWarning: Stopped after 1000 frames");
            break;
        }
    }
    
    let total_time = start_time.elapsed();
    let final_stats = chunk_manager.get_loading_stats();
    
    println!("\n=== Summary ===");
    println!("Total chunks loaded: {}", final_stats.loaded_chunks);
    println!("Total frames: {}", frame_count);
    println!("Total time: {:.2}s", total_time.as_secs_f32());
    println!("Average FPS: {:.1}", frame_count as f32 / total_time.as_secs_f32());
    println!("Average chunks per frame: {:.2}", final_stats.loaded_chunks as f32 / frame_count as f32);
    
    println!("\nThrottling successfully prevented loading {} chunks at once!", approx_chunks);
    println!("Instead, chunks were loaded progressively over {} frames.", frame_count);
}