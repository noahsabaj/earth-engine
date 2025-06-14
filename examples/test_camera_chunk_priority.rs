/// Test camera chunk prioritization system
/// 
/// This test verifies that chunks around the camera position are always loaded
/// and prioritized for generation.

use earth_engine::{
    world::{World, ChunkPos, DefaultWorldGenerator},
    BlockId,
};
use cgmath::Point3;
use std::time::Instant;

fn main() {
    env_logger::init();
    
    println!("Camera Chunk Prioritization Test");
    println!("=================================\n");
    
    // Use BlockIds directly for the generator
    let grass_id = BlockId::GRASS;
    let dirt_id = BlockId::DIRT;
    let stone_id = BlockId::STONE;
    let water_id = BlockId::WATER;
    let sand_id = BlockId::SAND;
    
    // Create world generator
    let generator = Box::new(DefaultWorldGenerator::new(
        12345, // seed
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    // Create world with camera-aware chunk loading
    let view_distance = 4; // 4 chunks in each direction
    let chunk_size = 32;
    
    println!("Creating world with view distance: {} chunks", view_distance);
    println!("Chunk size: {}", chunk_size);
    
    let mut world = World::new_with_generator(chunk_size, view_distance, generator);
    
    // Test camera positions to verify chunk loading
    let test_positions = vec![
        Point3::new(0.0, 64.0, 0.0),           // Origin
        Point3::new(100.0, 64.0, 100.0),       // Different chunk
        Point3::new(-50.0, 64.0, -50.0),       // Negative coordinates
        Point3::new(200.0, 64.0, 200.0),       // Far from origin
    ];
    
    for (i, camera_pos) in test_positions.iter().enumerate() {
        println!("\nTest #{}: Camera at position {:?}", i + 1, camera_pos);
        
        // Calculate expected camera chunk
        let camera_chunk = ChunkPos::new(
            (camera_pos.x / chunk_size as f32).floor() as i32,
            (camera_pos.y / chunk_size as f32).floor() as i32,
            (camera_pos.z / chunk_size as f32).floor() as i32,
        );
        println!("Expected camera chunk: {:?}", camera_chunk);
        
        // Test camera chunk loading
        let start_time = Instant::now();
        let camera_chunk_loaded = world.ensure_camera_chunk_loaded(*camera_pos);
        let load_time = start_time.elapsed();
        
        println!("Camera chunk loaded: {} (time: {:.2}ms)", 
                camera_chunk_loaded, load_time.as_millis());
        
        // Verify the chunk is actually loaded
        let is_loaded = world.has_chunk(camera_chunk);
        println!("Chunk verified as loaded: {}", is_loaded);
        
        // Update world to trigger chunk loading system
        world.update_loaded_chunks(*camera_pos);
        
        // Check surrounding chunks in 3x3x3 area
        let mut critical_chunks_loaded = 0;
        let mut total_critical_chunks = 0;
        
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let chunk_pos = ChunkPos::new(
                        camera_chunk.x + dx,
                        camera_chunk.y + dy,
                        camera_chunk.z + dz,
                    );
                    total_critical_chunks += 1;
                    
                    if world.has_chunk(chunk_pos) {
                        critical_chunks_loaded += 1;
                    }
                }
            }
        }
        
        println!("Critical chunks (3x3x3) loaded: {}/{}", 
                critical_chunks_loaded, total_critical_chunks);
        
        // Wait a bit to allow chunk loading to process
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        // Check again after update
        world.update_loaded_chunks(*camera_pos);
        let mut updated_chunks_loaded = 0;
        
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let chunk_pos = ChunkPos::new(
                        camera_chunk.x + dx,
                        camera_chunk.y + dy,
                        camera_chunk.z + dz,
                    );
                    
                    if world.has_chunk(chunk_pos) {
                        updated_chunks_loaded += 1;
                    }
                }
            }
        }
        
        println!("Critical chunks after update: {}/{}", 
                updated_chunks_loaded, total_critical_chunks);
        
        // Test multiple updates to see if chunk loading stabilizes
        for update_round in 1..=3 {
            world.update_loaded_chunks(*camera_pos);
            std::thread::sleep(std::time::Duration::from_millis(50));
            
            let mut round_chunks_loaded = 0;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        let chunk_pos = ChunkPos::new(
                            camera_chunk.x + dx,
                            camera_chunk.y + dy,
                            camera_chunk.z + dz,
                        );
                        
                        if world.has_chunk(chunk_pos) {
                            round_chunks_loaded += 1;
                        }
                    }
                }
            }
            
            println!("  Update round {}: {}/{} critical chunks loaded", 
                    update_round, round_chunks_loaded, total_critical_chunks);
        }
        
        println!("Total loaded chunks: {}", world.loaded_chunk_count());
    }
    
    println!("\nCamera chunk prioritization test completed!");
    println!("Check the log output for camera-critical chunk loading messages.");
}