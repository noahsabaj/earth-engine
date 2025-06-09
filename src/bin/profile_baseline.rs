use earth_engine::profiling::{CacheProfiler, MemoryProfiler, PerformanceMetrics};
use earth_engine::world::{
    Block, BlockId, Chunk, ChunkPos, VoxelPos, World,
    ParallelWorld, ParallelChunkManager, ParallelWorldConfig,
    BlockRegistry, DefaultWorldGenerator,
};
use earth_engine::renderer::{
    AsyncMeshBuilder,
    AsyncChunkRenderer,
};
use earth_engine::lighting::parallel_propagator::{ParallelLightPropagator, BlockProvider};
use earth_engine::lighting::LightType;
use earth_engine::lighting::concurrent_provider::ParallelBlockProvider;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("=== Earth Engine Baseline Performance Profile ===\n");
    
    let cache_profiler = CacheProfiler::new();
    let memory_profiler = MemoryProfiler::new();
    let perf_metrics = PerformanceMetrics::new();
    
    // Test parameters
    let world_size = 9; // 9x9x9 chunks = 729 chunks
    let chunk_size = 32;
    
    println!("Test parameters:");
    println!("  World size: {}x{}x{} chunks", world_size, world_size, world_size);
    println!("  Chunk size: {}x{}x{} voxels", chunk_size, chunk_size, chunk_size);
    println!("  Total voxels: {}\n", world_size * world_size * world_size * chunk_size * chunk_size * chunk_size);
    
    // Profile chunk generation
    profile_chunk_generation(&cache_profiler, &memory_profiler, &perf_metrics, world_size);
    
    // Profile mesh building
    profile_mesh_building(&cache_profiler, &memory_profiler, &perf_metrics, world_size);
    
    // Profile lighting
    profile_lighting(&cache_profiler, &memory_profiler, &perf_metrics);
    
    // Profile chunk access patterns
    profile_chunk_access_patterns(&cache_profiler, &memory_profiler);
    
    // Generate reports
    println!("\n=== PROFILING RESULTS ===\n");
    
    memory_profiler.identify_hot_paths(100, 10);
    memory_profiler.report();
    cache_profiler.report();
    perf_metrics.report();
    
    // Save metrics to file for comparison
    save_baseline_metrics(&cache_profiler, &memory_profiler, &perf_metrics);
}

fn profile_chunk_generation(
    cache_profiler: &CacheProfiler,
    memory_profiler: &MemoryProfiler,
    perf_metrics: &PerformanceMetrics,
    world_size: usize,
) {
    println!("Profiling chunk generation...");
    
    // Create world generator
    let generator = Box::new(DefaultWorldGenerator::new(
        42, // seed
        BlockId::GRASS,
        BlockId::DIRT,
        BlockId::STONE,
        BlockId::WATER,
        BlockId::SAND,
    ));
    
    let config = ParallelWorldConfig::default();
    let world = ParallelWorld::new(generator, config);
    
    // ParallelWorld already contains a chunk manager internally, so we don't need to create one
    
    let start = Instant::now();
    
    // Generate chunks
    let mut chunk_positions = Vec::new();
    for x in 0..world_size {
        for y in 0..world_size {
            for z in 0..world_size {
                chunk_positions.push(ChunkPos::new(x as i32, y as i32, z as i32));
            }
        }
    }
    
    // Profile memory access during generation
    let scope = earth_engine::profiling::memory_profiler::ProfileScope::new(memory_profiler, "chunk_generation");
    
    // Use pregenerate_chunks to trigger generation
    // This generates chunks in a radius around a center point
    let center = ChunkPos::new(world_size as i32 / 2, world_size as i32 / 2, world_size as i32 / 2);
    world.chunk_manager().pregenerate_chunks(center, world_size as i32 / 2);
    
    // Wait a bit for generation to complete (in real usage, this would be event-driven)
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Collect generated chunks
    let mut generated_count = 0;
    for pos in &chunk_positions {
        if world.chunk_manager().get_chunk(*pos).is_some() {
            generated_count += 1;
        }
    }
    
    perf_metrics.record_chunk_generation(generated_count as u64);
    
    // Analyze chunk memory layout if we have chunks
    if let Some(chunk_arc) = world.chunk_manager().get_chunk(chunk_positions[0]) {
        let chunk = chunk_arc.read();
        analyze_chunk_memory_layout(&*chunk, cache_profiler, memory_profiler);
    }
    
    drop(scope);
    
    let elapsed = start.elapsed();
    println!("  Generated {} chunks in {:.2}s", generated_count, elapsed.as_secs_f64());
}

fn profile_mesh_building(
    cache_profiler: &CacheProfiler,
    memory_profiler: &MemoryProfiler,
    perf_metrics: &PerformanceMetrics,
    world_size: usize,
) {
    println!("\nProfiling mesh building...");
    
    // Create world with generator and config
    let generator = Box::new(DefaultWorldGenerator::new(
        42,
        BlockId::GRASS,
        BlockId::DIRT,
        BlockId::STONE,
        BlockId::WATER,
        BlockId::SAND,
    ));
    let config = ParallelWorldConfig::default();
    let world = Arc::new(ParallelWorld::new(generator, config));
    
    // Generate some chunks first
    let mut chunk_positions = Vec::new();
    for x in 0..5 {
        for y in 0..5 {
            for z in 0..5 {
                chunk_positions.push(ChunkPos::new(x, y, z));
            }
        }
    }
    
    // Generate chunks
    let center = ChunkPos::new(2, 2, 2);
    world.chunk_manager().pregenerate_chunks(center, 3);
    
    // Wait for generation
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    // Profile mesh building
    let registry = Arc::new(BlockRegistry::new());
    let mesh_builder = AsyncMeshBuilder::new(registry, 32, Some(4));
    let start = Instant::now();
    
    let scope = earth_engine::profiling::memory_profiler::ProfileScope::new(memory_profiler, "mesh_building");
    
    // Build meshes
    for pos in &chunk_positions {
        if let Some(chunk) = world.chunk_manager().get_chunk(*pos) {
            // Get neighbors
            let neighbors = [
                world.chunk_manager().get_chunk(ChunkPos::new(pos.x + 1, pos.y, pos.z)),
                world.chunk_manager().get_chunk(ChunkPos::new(pos.x - 1, pos.y, pos.z)),
                world.chunk_manager().get_chunk(ChunkPos::new(pos.x, pos.y + 1, pos.z)),
                world.chunk_manager().get_chunk(ChunkPos::new(pos.x, pos.y - 1, pos.z)),
                world.chunk_manager().get_chunk(ChunkPos::new(pos.x, pos.y, pos.z + 1)),
                world.chunk_manager().get_chunk(ChunkPos::new(pos.x, pos.y, pos.z - 1)),
            ];
            mesh_builder.queue_chunk(*pos, chunk, 0, neighbors);
        }
    }
    
    // Wait for completion
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let mesh_count = chunk_positions.len();
    perf_metrics.record_mesh_build(mesh_count as u64);
    
    drop(scope);
    
    let elapsed = start.elapsed();
    println!("  Built {} meshes in {:.2}s", mesh_count, elapsed.as_secs_f64());
}

fn profile_lighting(
    cache_profiler: &CacheProfiler,
    memory_profiler: &MemoryProfiler,
    perf_metrics: &PerformanceMetrics,
) {
    println!("\nProfiling lighting system...");
    
    // Create world
    let generator = Box::new(DefaultWorldGenerator::new(
        42,
        BlockId::GRASS,
        BlockId::DIRT,
        BlockId::STONE,
        BlockId::WATER,
        BlockId::SAND,
    ));
    let config = ParallelWorldConfig::default();
    let world = Arc::new(ParallelWorld::new(generator, config));
    
    // Create block provider for the propagator
    let block_provider = Arc::new(ParallelBlockProvider::new(world.chunk_manager_arc()));
    let propagator = ParallelLightPropagator::new(block_provider, 32, Some(8));
    
    // Generate test chunks
    let positions: Vec<_> = (0..3).flat_map(|x| {
        (0..3).flat_map(move |y| {
            (0..3).map(move |z| ChunkPos::new(x, y, z))
        })
    }).collect();
    
    let center = ChunkPos::new(1, 1, 1);
    world.chunk_manager().pregenerate_chunks(center, 2);
    
    // Wait for generation
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    let start = Instant::now();
    let scope = earth_engine::profiling::memory_profiler::ProfileScope::new(memory_profiler, "lighting_propagation");
    
    // Add some light sources
    let light_sources = vec![
        VoxelPos::new(16, 16, 16),
        VoxelPos::new(48, 48, 48),
        VoxelPos::new(80, 80, 80),
    ];
    
    // Profile light propagation memory access
    for source in &light_sources {
        memory_profiler.record_access_pattern("lighting_propagation", 
            earth_engine::profiling::AccessPattern::Random);
    }
    
    perf_metrics.record_light_update(light_sources.len() as u64);
    
    drop(scope);
    
    let elapsed = start.elapsed();
    println!("  Processed {} light sources in {:.2}s", light_sources.len(), elapsed.as_secs_f64());
}

fn profile_chunk_access_patterns(
    cache_profiler: &CacheProfiler,
    memory_profiler: &MemoryProfiler,
) {
    println!("\nProfiling chunk access patterns...");
    
    let chunk = Chunk::new(ChunkPos::new(0, 0, 0), 32);
    let size = 32;
    
    // Sequential access (cache-friendly)
    println!("  Testing sequential access...");
    let mut addresses = Vec::new();
    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let index = (x * size * size + y * size + z) as usize;
                addresses.push(index);
            }
        }
    }
    
    cache_profiler.analyze_array_access(&[0u8; 32768], &addresses);
    let pattern = memory_profiler.analyze_access_pattern(&addresses);
    memory_profiler.record_access_pattern("chunk_sequential_access", pattern);
    
    // Random access (cache-unfriendly) 
    println!("  Testing random access...");
    addresses.clear();
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let mut indices: Vec<_> = (0..32768).collect();
    indices.shuffle(&mut rng);
    addresses.extend(&indices[0..1000]);
    
    cache_profiler.analyze_array_access(&[0u8; 32768], &addresses);
    let pattern = memory_profiler.analyze_access_pattern(&addresses);
    memory_profiler.record_access_pattern("chunk_random_access", pattern);
}

fn analyze_chunk_memory_layout(
    chunk: &Chunk,
    cache_profiler: &CacheProfiler,
    memory_profiler: &MemoryProfiler,
) {
    println!("  Analyzing chunk memory layout...");
    
    // Analyze how chunk data is accessed during common operations
    let size = 32;
    let mut addresses = Vec::new();
    
    // Simulate mesh generation access pattern
    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let index = (x + y * size + z * size * size) as usize;
                addresses.push(index);
            }
        }
    }
    
    let pattern = memory_profiler.analyze_access_pattern(&addresses);
    memory_profiler.record_access_pattern("chunk_mesh_generation", pattern);
    
    // Record cache efficiency
    let efficiency = (cache_profiler.cache_efficiency() * 100.0) as u64;
    performance_metrics().record_cache_efficiency(efficiency);
}

fn save_baseline_metrics(
    cache_profiler: &CacheProfiler,
    memory_profiler: &MemoryProfiler,
    perf_metrics: &PerformanceMetrics,
) {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = File::create("baseline_metrics.txt").unwrap();
    
    writeln!(file, "=== Baseline Performance Metrics ===").unwrap();
    writeln!(file, "Date: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")).unwrap();
    writeln!(file, "\nCache Efficiency: {:.2}%", cache_profiler.cache_efficiency() * 100.0).unwrap();
    writeln!(file, "Average FPS: {:.2}", perf_metrics.average_fps()).unwrap();
    writeln!(file, "Chunks per second: {:.2}", perf_metrics.chunks_per_second()).unwrap();
    
    writeln!(file, "\nHot Paths:").unwrap();
    for hot_path in memory_profiler.hot_paths() {
        writeln!(file, "  {} - {} calls, {:.2}ms avg", 
            hot_path.function, 
            hot_path.call_count,
            hot_path.avg_time.as_secs_f64() * 1000.0
        ).unwrap();
    }
    
    println!("\nBaseline metrics saved to baseline_metrics.txt");
}

// Helper function to get global performance metrics
fn performance_metrics() -> &'static PerformanceMetrics {
    static METRICS: std::sync::OnceLock<PerformanceMetrics> = std::sync::OnceLock::new();
    METRICS.get_or_init(PerformanceMetrics::new)
}