use earth_engine::{
    lighting::{
        ParallelLightPropagator, TestBlockProvider,
        LightType, BatchLightCalculator,
        MAX_LIGHT_LEVEL,
    },
    world::{ChunkPos, VoxelPos},
    BlockId,
};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("Earth Engine - Parallel Lighting Benchmark");
    println!("=========================================");
    
    let chunk_size = 32;
    let world_size = 8; // 8x8x8 chunks
    let total_chunks = world_size * world_size * world_size;
    
    println!("Configuration:");
    println!("  Chunk size: {}x{}x{}", chunk_size, chunk_size, chunk_size);
    println!("  World size: {}x{}x{} chunks", world_size, world_size, world_size);
    println!("  Total chunks: {}", total_chunks);
    println!("  CPU cores: {}", num_cpus::get());
    println!();
    
    // Place some light sources
    let light_sources = vec![
        (VoxelPos::new(64, 64, 64), LightType::Block, MAX_LIGHT_LEVEL), // Center
        (VoxelPos::new(32, 80, 32), LightType::Block, MAX_LIGHT_LEVEL),
        (VoxelPos::new(96, 80, 96), LightType::Block, MAX_LIGHT_LEVEL),
        (VoxelPos::new(64, 100, 64), LightType::Block, MAX_LIGHT_LEVEL),
        (VoxelPos::new(128, 64, 128), LightType::Block, MAX_LIGHT_LEVEL),
    ];
    
    // Test 1: Parallel light propagation (1 thread)
    println!("Test 1: Parallel Light Propagation (1 thread)");
    println!("---------------------------------------------");
    
    let test_provider = Arc::new(TestBlockProvider::new(chunk_size));
    populate_test_provider(&test_provider, chunk_size, world_size);
    
    let parallel_propagator_1 = Arc::new(ParallelLightPropagator::new(
        test_provider.clone(),
        chunk_size,
        Some(1),
    ));
    
    // Add light sources
    for (pos, light_type, level) in &light_sources {
        parallel_propagator_1.add_light(*pos, *light_type, *level);
    }
    
    let start_time = Instant::now();
    parallel_propagator_1.process_updates(usize::MAX);
    let parallel_1_time = start_time.elapsed();
    
    let stats_1 = parallel_propagator_1.get_stats();
    println!("  Time: {:.2}s", parallel_1_time.as_secs_f32());
    println!("  Updates processed: {}", stats_1.updates_processed);
    println!("  Chunks affected: {}", stats_1.chunks_affected);
    println!("  Cross-chunk updates: {}", stats_1.cross_chunk_updates);
    println!();
    
    // Test 2: Parallel light propagation (optimal threads)
    println!("Test 2: Parallel Light Propagation (optimal threads)");
    println!("---------------------------------------------------");
    
    let test_provider_opt = Arc::new(TestBlockProvider::new(chunk_size));
    populate_test_provider(&test_provider_opt, chunk_size, world_size);
    
    let parallel_propagator_opt = Arc::new(ParallelLightPropagator::new(
        test_provider_opt.clone(),
        chunk_size,
        None, // Auto-detect optimal threads
    ));
    
    // Add light sources
    for (pos, light_type, level) in &light_sources {
        parallel_propagator_opt.add_light(*pos, *light_type, *level);
    }
    
    let start_time = Instant::now();
    parallel_propagator_opt.process_updates(usize::MAX);
    let parallel_opt_time = start_time.elapsed();
    
    let stats_opt = parallel_propagator_opt.get_stats();
    println!("  Time: {:.2}s", parallel_opt_time.as_secs_f32());
    println!("  Updates processed: {}", stats_opt.updates_processed);
    println!("  Chunks affected: {}", stats_opt.chunks_affected);
    println!("  Cross-chunk updates: {}", stats_opt.cross_chunk_updates);
    println!("  Updates/second: {:.0}", stats_opt.updates_per_second);
    println!();
    
    // Test 3: Skylight calculation
    println!("Test 3: Parallel Skylight Calculation");
    println!("------------------------------------");
    
    let skylight_provider = Arc::new(TestBlockProvider::new(chunk_size));
    populate_test_provider(&skylight_provider, chunk_size, world_size);
    
    let skylight_propagator = Arc::new(ParallelLightPropagator::new(
        skylight_provider.clone(),
        chunk_size,
        None,
    ));
    
    let batch_calculator = BatchLightCalculator::new(skylight_propagator.clone());
    
    // Generate list of all chunks
    let all_chunks: Vec<ChunkPos> = (0..world_size)
        .flat_map(|x| (0..world_size)
            .flat_map(move |y| (0..world_size)
                .map(move |z| ChunkPos::new(x as i32, y as i32, z as i32))))
        .collect();
    
    let start_time = Instant::now();
    batch_calculator.calculate_skylight_batch(all_chunks.clone());
    let skylight_time = start_time.elapsed();
    
    println!("  Chunks processed: {}", all_chunks.len());
    println!("  Time: {:.2}s", skylight_time.as_secs_f32());
    println!("  Chunks/second: {:.2}", all_chunks.len() as f32 / skylight_time.as_secs_f32());
    println!();
    
    // Summary
    println!("Summary");
    println!("-------");
    let speedup = parallel_1_time.as_secs_f32() / parallel_opt_time.as_secs_f32();
    
    println!("  Parallel (1 thread): {:.2}s", parallel_1_time.as_secs_f32());
    println!("  Parallel (optimal): {:.2}s ({:.1}x speedup vs 1 thread)", parallel_opt_time.as_secs_f32(), speedup);
    
    // Test 4: Stress test with many light sources
    println!("\nTest 4: Stress Test (100 light sources)");
    println!("---------------------------------------");
    
    let stress_provider = Arc::new(TestBlockProvider::new(chunk_size));
    populate_test_provider(&stress_provider, chunk_size, world_size);
    
    let stress_propagator = Arc::new(ParallelLightPropagator::new(
        stress_provider.clone(),
        chunk_size,
        None,
    ));
    
    // Add many light sources
    let mut rng = 12345u64;
    for _ in 0..100 {
        // Simple pseudo-random
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let x = (rng % (world_size as u64 * chunk_size as u64)) as i32;
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let y = (rng % (world_size as u64 * chunk_size as u64)) as i32;
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let z = (rng % (world_size as u64 * chunk_size as u64)) as i32;
        
        stress_propagator.add_light(
            VoxelPos::new(x, y, z),
            LightType::Block,
            MAX_LIGHT_LEVEL,
        );
    }
    
    let start_time = Instant::now();
    stress_propagator.process_updates(usize::MAX);
    let stress_time = start_time.elapsed();
    
    let stress_stats = stress_propagator.get_stats();
    println!("  Time: {:.2}s", stress_time.as_secs_f32());
    println!("  Updates processed: {}", stress_stats.updates_processed);
    println!("  Updates/second: {:.0}", stress_stats.updates_per_second);
    println!("  Cross-chunk updates: {}", stress_stats.cross_chunk_updates);
}


fn populate_test_provider(provider: &TestBlockProvider, chunk_size: u32, world_size: i32) {
    for x in 0..world_size {
        for y in 0..world_size {
            for z in 0..world_size {
                let mut blocks = vec![BlockId::AIR; (chunk_size * chunk_size * chunk_size) as usize];
                
                // Add floor at y=0
                if y == 0 {
                    for dx in 0..chunk_size {
                        for dz in 0..chunk_size {
                            let idx = (0 * chunk_size * chunk_size + dz * chunk_size + dx) as usize;
                            blocks[idx] = BlockId(1); // Stone
                        }
                    }
                }
                
                // Add walls
                if x == 0 || x == world_size - 1 || z == 0 || z == world_size - 1 {
                    for dy in 0..chunk_size / 2 {
                        for dx in 0..chunk_size {
                            let idx = (dy * chunk_size * chunk_size + 0 * chunk_size + dx) as usize;
                            blocks[idx] = BlockId(1);
                        }
                    }
                }
                
                provider.set_chunk(ChunkPos::new(x, y, z), blocks);
            }
        }
    }
}

