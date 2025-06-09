use earth_engine::{
    world::{ParallelWorld, ParallelWorldConfig, World, DefaultWorldGenerator},
    BlockId,
};
use cgmath::Point3;
use std::time::Instant;

fn main() {
    println!("Earth Engine - Parallel Chunk Generation Benchmark");
    println!("=================================================");
    
    let chunk_size = 32;
    let view_distance = 4;
    let chunks_to_generate = ((2 * view_distance + 1) as i32).pow(3);
    
    println!("Configuration:");
    println!("  Chunk size: {}x{}x{}", chunk_size, chunk_size, chunk_size);
    println!("  View distance: {} chunks", view_distance);
    println!("  Total chunks: {}", chunks_to_generate);
    println!("  CPU cores: {}", num_cpus::get());
    println!();
    
    // Create test blocks
    let grass_id = BlockId(1);
    let dirt_id = BlockId(2);
    let stone_id = BlockId(3);
    let water_id = BlockId(4);
    let sand_id = BlockId(5);
    
    // Test 1: Serial generation (old method)
    println!("Test 1: Serial Chunk Generation");
    println!("-------------------------------");
    
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    let mut serial_world = World::new_with_generator(chunk_size, view_distance, generator);
    
    let start_time = Instant::now();
    serial_world.update_loaded_chunks(Point3::new(0.0, 100.0, 0.0));
    let serial_time = start_time.elapsed();
    
    println!("  Time: {:.2}s", serial_time.as_secs_f32());
    println!("  Chunks/second: {:.2}", chunks_to_generate as f32 / serial_time.as_secs_f32());
    println!();
    
    // Test 2: Parallel generation with default config
    println!("Test 2: Parallel Generation (Default Config)");
    println!("-------------------------------------------");
    
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    let config = ParallelWorldConfig {
        view_distance,
        chunk_size,
        ..Default::default()
    };
    
    let parallel_world = ParallelWorld::new(generator, config);
    
    let start_time = Instant::now();
    parallel_world.pregenerate_spawn_area(Point3::new(0.0, 100.0, 0.0), view_distance);
    let parallel_time = start_time.elapsed();
    
    let metrics = parallel_world.get_performance_metrics();
    
    println!("  Time: {:.2}s", parallel_time.as_secs_f32());
    println!("  Chunks/second: {:.2}", metrics.chunks_per_second);
    println!("  Average chunk time: {:.2}ms", metrics.average_chunk_time.as_millis());
    println!();
    
    // Test 3: Parallel generation with max threads
    println!("Test 3: Parallel Generation (Max Threads)");
    println!("-----------------------------------------");
    
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    let config = ParallelWorldConfig {
        generation_threads: num_cpus::get(),
        mesh_threads: num_cpus::get(),
        chunks_per_frame: num_cpus::get() * 4,
        view_distance,
        chunk_size,
    };
    
    let parallel_world_max = ParallelWorld::new(generator, config);
    
    let start_time = Instant::now();
    parallel_world_max.pregenerate_spawn_area(Point3::new(0.0, 100.0, 0.0), view_distance);
    let parallel_max_time = start_time.elapsed();
    
    let metrics_max = parallel_world_max.get_performance_metrics();
    
    println!("  Time: {:.2}s", parallel_max_time.as_secs_f32());
    println!("  Chunks/second: {:.2}", metrics_max.chunks_per_second);
    println!("  Average chunk time: {:.2}ms", metrics_max.average_chunk_time.as_millis());
    println!();
    
    // Summary
    println!("Summary");
    println!("-------");
    let speedup_default = serial_time.as_secs_f32() / parallel_time.as_secs_f32();
    let speedup_max = serial_time.as_secs_f32() / parallel_max_time.as_secs_f32();
    
    println!("  Serial generation: {:.2}s", serial_time.as_secs_f32());
    println!("  Parallel (default): {:.2}s ({:.1}x speedup)", parallel_time.as_secs_f32(), speedup_default);
    println!("  Parallel (max): {:.2}s ({:.1}x speedup)", parallel_max_time.as_secs_f32(), speedup_max);
    
    // Test 4: Large world stress test
    println!("\nTest 4: Large World Stress Test");
    println!("--------------------------------");
    
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    let config = ParallelWorldConfig {
        generation_threads: num_cpus::get(),
        view_distance: 8, // Larger view distance
        chunk_size,
        ..Default::default()
    };
    
    let stress_world = ParallelWorld::new(generator, config);
    let large_chunks = ((2 * 8 + 1) as i32).pow(3);
    
    println!("  Generating {} chunks...", large_chunks);
    let start_time = Instant::now();
    stress_world.pregenerate_spawn_area(Point3::new(0.0, 100.0, 0.0), 8);
    let stress_time = start_time.elapsed();
    
    let stress_metrics = stress_world.get_performance_metrics();
    
    println!("  Time: {:.2}s", stress_time.as_secs_f32());
    println!("  Chunks/second: {:.2}", stress_metrics.chunks_per_second);
    println!("  Total chunks: {}", stress_metrics.loaded_chunks);
    println!("  Cached chunks: {}", stress_metrics.cached_chunks);
}