use hearth_engine::world::{ParallelWorld, ParallelWorldConfig};
use hearth_engine::world::generation::TerrainGenerator;
use cgmath::Point3;
use std::time::Instant;

fn main() {
    env_logger::init();
    
    // Create world configuration
    let config = ParallelWorldConfig {
        generation_threads: 4,
        mesh_threads: 4,
        chunks_per_frame: 16,
        view_distance: 8,
        chunk_size: 32,
    };
    
    // Create terrain generator
    let generator = Box::new(TerrainGenerator::new(12345));
    
    // Create parallel world
    let world = ParallelWorld::new(generator, config);
    
    // Test non-blocking spawn pregeneration
    println!("Testing non-blocking spawn pregeneration...");
    let spawn_pos = Point3::new(0.0, 64.0, 0.0);
    
    match world.pregenerate_spawn_area(spawn_pos, 3) {
        Ok(handle) => {
            println!("Spawn generation started successfully!");
            
            // Monitor progress
            while !handle.is_complete() {
                println!("Progress: {:.1}% ({}/{} chunks)", 
                    handle.progress_percent(),
                    handle.chunks_generated(),
                    handle.total_chunks
                );
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            
            println!("Spawn generation completed in {:.2}s", handle.elapsed().as_secs_f32());
        }
        Err(e) => {
            eprintln!("Failed to start spawn generation: {}", e);
        }
    }
    
    // Test blocking spawn pregeneration
    println!("\nTesting blocking spawn pregeneration...");
    let start = Instant::now();
    
    match world.pregenerate_spawn_area_blocking(spawn_pos, 2) {
        Ok(()) => {
            println!("Blocking spawn generation completed in {:.2}s", start.elapsed().as_secs_f32());
        }
        Err(e) => {
            eprintln!("Blocking spawn generation failed: {}", e);
        }
    }
    
    // Get performance metrics
    let metrics = world.get_performance_metrics();
    println!("\nPerformance metrics:");
    println!("  Loaded chunks: {}", metrics.loaded_chunks);
    println!("  Chunks per second: {:.2}", metrics.chunks_per_second);
    println!("  Average chunk time: {:?}", metrics.average_chunk_time);
}