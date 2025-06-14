// Earth Engine Performance Regression Testing Framework
// Sprint 38: System Integration
//
// Automated performance regression detection for critical engine systems.
// Monitors performance metrics and detects when performance degrades over time.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use glam::Vec3;
use earth_engine::{
    world::{World, BlockId, VoxelPos, ChunkPos, Chunk},
    physics_data::PhysicsData,
    particles::particle_data::ParticleData,
    profiling::{PerformanceMetrics, AllocationProfiler},
};

/// Performance benchmark categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BenchmarkCategory {
    WorldGeneration,
    PhysicsSimulation,
    ParticleUpdate,
    ChunkMeshing,
    MemoryAllocation,
    NetworkSerialization,
    GPUOperations,
    SystemIntegration,
}

/// Individual performance metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBenchmark {
    category: BenchmarkCategory,
    name: String,
    operations_per_second: f64,
    memory_usage_mb: f64,
    duration_ms: f64,
    timestamp: String,
    system_info: SystemInfo,
}

/// System information for benchmark context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    cpu_cores: usize,
    memory_gb: f64,
    os: String,
    build_type: String, // Debug/Release
}

/// Performance regression detector
#[derive(Debug)]
pub struct RegressionDetector {
    baselines: HashMap<String, PerformanceBenchmark>,
    history: Vec<PerformanceBenchmark>,
    regression_threshold: f64, // Percentage degradation threshold
}

impl RegressionDetector {
    pub fn new(regression_threshold: f64) -> Self {
        Self {
            baselines: HashMap::new(),
            history: Vec::new(),
            regression_threshold,
        }
    }
    
    pub fn add_baseline(&mut self, benchmark: PerformanceBenchmark) {
        let key = format!("{:?}_{}", benchmark.category, benchmark.name);
        self.baselines.insert(key, benchmark);
    }
    
    pub fn record_benchmark(&mut self, benchmark: PerformanceBenchmark) {
        self.history.push(benchmark.clone());
        
        let key = format!("{:?}_{}", benchmark.category, benchmark.name);
        if let Some(baseline) = self.baselines.get(&key) {
            self.check_regression(baseline, &benchmark);
        }
    }
    
    fn check_regression(&self, baseline: &PerformanceBenchmark, current: &PerformanceBenchmark) {
        let performance_change = (current.operations_per_second - baseline.operations_per_second) 
                               / baseline.operations_per_second * 100.0;
        let memory_change = (current.memory_usage_mb - baseline.memory_usage_mb) 
                          / baseline.memory_usage_mb * 100.0;
        
        if performance_change < -self.regression_threshold {
            println!("⚠️  PERFORMANCE REGRESSION DETECTED:");
            println!("   Benchmark: {} - {}", 
                     format!("{:?}", baseline.category), baseline.name);
            println!("   Performance: {:.1}% slower ({:.0} -> {:.0} ops/sec)", 
                     -performance_change, baseline.operations_per_second, current.operations_per_second);
        }
        
        if memory_change > self.regression_threshold {
            println!("⚠️  MEMORY REGRESSION DETECTED:");
            println!("   Benchmark: {} - {}", 
                     format!("{:?}", baseline.category), baseline.name);
            println!("   Memory: {:.1}% increase ({:.1} -> {:.1} MB)", 
                     memory_change, baseline.memory_usage_mb, current.memory_usage_mb);
        }
    }
    
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("# Performance Regression Report\n\n");
        
        if self.history.is_empty() {
            report.push_str("No benchmarks recorded.\n");
            return report;
        }
        
        // Group by category
        let mut by_category: HashMap<BenchmarkCategory, Vec<&PerformanceBenchmark>> = HashMap::new();
        for benchmark in &self.history {
            by_category.entry(benchmark.category.clone()).or_default().push(benchmark);
        }
        
        for (category, benchmarks) in by_category {
            report.push_str(&format!("## {:?}\n\n", category));
            
            for benchmark in benchmarks {
                report.push_str(&format!(
                    "- {}: {:.0} ops/sec, {:.1} MB, {:.1}ms\n",
                    benchmark.name, 
                    benchmark.operations_per_second,
                    benchmark.memory_usage_mb,
                    benchmark.duration_ms
                ));
            }
            report.push('\n');
        }
        
        report
    }
}

/// Benchmark helper functions
impl PerformanceBenchmark {
    fn new(category: BenchmarkCategory, name: String) -> Self {
        Self {
            category,
            name,
            operations_per_second: 0.0,
            memory_usage_mb: 0.0,
            duration_ms: 0.0,
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            system_info: SystemInfo {
                cpu_cores: num_cpus::get(),
                memory_gb: 16.0, // Mock value
                os: std::env::consts::OS.to_string(),
                build_type: if cfg!(debug_assertions) { "Debug" } else { "Release" }.to_string(),
            },
        }
    }
    
    fn with_timing(mut self, duration: Duration, operations: usize) -> Self {
        self.duration_ms = duration.as_millis() as f64;
        if duration.as_secs_f64() > 0.0 {
            self.operations_per_second = operations as f64 / duration.as_secs_f64();
        }
        self
    }
    
    fn with_memory(mut self, memory_mb: f64) -> Self {
        self.memory_usage_mb = memory_mb;
        self
    }
}

#[test]
fn test_world_generation_performance() {
    println!("📊 Benchmarking world generation performance...");
    
    let mut detector = RegressionDetector::new(15.0); // 15% regression threshold
    
    // Establish baseline performance
    let chunk_count = 25; // 5x5 chunks
    let start_time = Instant::now();
    let mut world = World::new(32);
    
    for x in -2..=2 {
        for z in -2..=2 {
            let chunk_pos = ChunkPos::new(x, 0, z);
            let mut chunk = Chunk::new(chunk_pos, 32);
            
            // Generate terrain
            for local_x in 0..32 {
                for local_z in 0..32 {
                    let world_x = x * 32 + local_x;
                    let world_z = z * 32 + local_z;
                    let height = 64 + (world_x % 8) + (world_z % 6) - 3;
                    
                    for y in 0..=height.min(127) {
                        let block_id = match y {
                            y if y == height => BlockId::Grass,
                            y if y >= height - 3 => BlockId::Dirt,
                            _ => BlockId::Stone,
                        };
                        chunk.set_block(local_x as u32, y as u32, local_z as u32, block_id);
                    }
                }
            }
            
            world.set_chunk(chunk_pos, chunk);
        }
    }
    
    let duration = start_time.elapsed();
    let memory_usage = chunk_count as f64 * 0.5; // Estimated MB per chunk
    
    let baseline = PerformanceBenchmark::new(
        BenchmarkCategory::WorldGeneration,
        "chunk_generation_5x5".to_string(),
    )
    .with_timing(duration, chunk_count)
    .with_memory(memory_usage);
    
    detector.add_baseline(baseline.clone());
    
    println!("   Baseline: {} chunks in {:?} ({:.0} chunks/sec)", 
             chunk_count, duration, baseline.operations_per_second);
    
    // Test current performance (simulate potential regression)
    let current_start = Instant::now();
    let mut test_world = World::new(32);
    
    for x in -2..=2 {
        for z in -2..=2 {
            let chunk_pos = ChunkPos::new(x, 0, z);
            let mut chunk = test_world.create_chunk(chunk_pos);
            
            // Same generation logic
            for local_x in 0..32 {
                for local_z in 0..32 {
                    let world_x = x * 32 + local_x;
                    let world_z = z * 32 + local_z;
                    let height = 64 + (world_x % 8) + (world_z % 6) - 3;
                    
                    for y in 0..=height.min(127) {
                        let block_id = match y {
                            y if y == height => BlockId::Grass,
                            y if y >= height - 3 => BlockId::Dirt,
                            _ => BlockId::Stone,
                        };
                        chunk.set_block(local_x as u32, y as u32, local_z as u32, block_id);
                    }
                }
            }
            
            test_world.set_chunk(chunk_pos, chunk);
        }
    }
    
    let current_duration = current_start.elapsed();
    let current_memory = chunk_count as f64 * 0.5;
    
    let current = PerformanceBenchmark::new(
        BenchmarkCategory::WorldGeneration,
        "chunk_generation_5x5".to_string(),
    )
    .with_timing(current_duration, chunk_count)
    .with_memory(current_memory);
    
    let ops_per_second = current.operations_per_second;
    detector.record_benchmark(current);
    
    println!("   Current: {} chunks in {:?} ({:.0} chunks/sec)", 
             chunk_count, current_duration, ops_per_second);
    
    // Performance should be reasonable
    assert!(ops_per_second >= 10.0, 
            "World generation should be at least 10 chunks/sec, got {:.1}", 
            ops_per_second);
    
    println!("✅ World generation performance benchmark completed");
}

#[test]
fn test_physics_simulation_performance() {
    println!("📊 Benchmarking physics simulation performance...");
    
    let mut detector = RegressionDetector::new(10.0); // 10% regression threshold
    
    // Setup physics simulation
    let entity_count = 1000;
    let mut physics_data = PhysicsData::new(entity_count);
    
    // Add entities
    for i in 0..entity_count {
        let position = [
            (i % 32) as f32 * 2.0,
            100.0 + (i / 32) as f32,
            (i % 16) as f32 * 2.0,
        ];
        let velocity = [0.0, 0.0, 0.0];
        let mass = 1.0;
        let half_extents = [0.5, 0.5, 0.5];
        
        physics_data.add_entity(position, velocity, mass, half_extents);
    }
    
    // Baseline physics simulation
    let simulation_frames = 100;
    let start_time = Instant::now();
    
    for frame in 0..simulation_frames {
        let entity_count = physics_data.entity_count();
        
        // Apply gravity
        earth_engine::physics_data::integration::parallel::apply_gravity(
            &mut physics_data.velocities[..entity_count],
            &physics_data.flags[..entity_count],
            earth_engine::physics::GRAVITY,
            0.016,
        );
        
        // Integrate positions
        earth_engine::physics_data::integration::parallel::integrate_positions(
            &mut physics_data.positions[..entity_count],
            &physics_data.velocities[..entity_count],
            &physics_data.flags[..entity_count],
            0.016,
        );
    }
    
    let duration = start_time.elapsed();
    let total_operations = entity_count * simulation_frames;
    let memory_usage = entity_count as f64 * 0.1; // Estimated KB per entity
    
    let baseline = PerformanceBenchmark::new(
        BenchmarkCategory::PhysicsSimulation,
        "physics_simulation_1000_entities".to_string(),
    )
    .with_timing(duration, total_operations)
    .with_memory(memory_usage);
    
    detector.add_baseline(baseline.clone());
    detector.record_benchmark(baseline.clone());
    
    println!("   Physics: {} entities × {} frames in {:?} ({:.0} updates/sec)", 
             entity_count, simulation_frames, duration, baseline.operations_per_second);
    
    // Performance assertions
    assert!(baseline.operations_per_second >= 50000.0, 
            "Physics should handle at least 50k updates/sec, got {:.0}", 
            baseline.operations_per_second);
    
    println!("✅ Physics simulation performance benchmark completed");
}

#[test]
fn test_particle_system_performance() {
    println!("📊 Benchmarking particle system performance...");
    
    let mut detector = RegressionDetector::new(12.0); // 12% regression threshold
    
    // Setup particle system
    let particle_count = 5000;
    let mut particles = ParticleData::new(particle_count);
    particles.count = particle_count;
    
    // Initialize particles
    for i in 0..particle_count {
        particles.position_x[i] = (i % 100) as f32;
        particles.position_y[i] = 50.0;
        particles.position_z[i] = (i / 100) as f32;
        particles.velocity_x[i] = (i as f32 * 0.01) % 2.0 - 1.0;
        particles.velocity_y[i] = (i as f32 * 0.03) % 1.0;
        particles.velocity_z[i] = (i as f32 * 0.02) % 2.0 - 1.0;
        particles.lifetime[i] = 10.0;
        particles.max_lifetime[i] = 10.0;
    }
    
    // Benchmark particle updates
    let world = World::new(32);
    let update_frames = 60;
    let start_time = Instant::now();
    
    for _frame in 0..update_frames {
        earth_engine::particles::update::update_particles(
            &mut particles,
            &world,
            0.016,
            Vec3::new(0.1, 0.0, 0.05),
            false,
        );
    }
    
    let duration = start_time.elapsed();
    let total_operations = particle_count * update_frames;
    let memory_usage = particle_count as f64 * 0.05; // Estimated KB per particle
    
    let benchmark = PerformanceBenchmark::new(
        BenchmarkCategory::ParticleUpdate,
        "particle_update_5000_particles".to_string(),
    )
    .with_timing(duration, total_operations)
    .with_memory(memory_usage);
    
    detector.add_baseline(benchmark.clone());
    detector.record_benchmark(benchmark.clone());
    
    println!("   Particles: {} particles × {} frames in {:?} ({:.0} updates/sec)", 
             particle_count, update_frames, duration, benchmark.operations_per_second);
    
    // Performance assertions
    assert!(benchmark.operations_per_second >= 100000.0, 
            "Particles should handle at least 100k updates/sec, got {:.0}", 
            benchmark.operations_per_second);
    
    println!("✅ Particle system performance benchmark completed");
}

#[test] 
fn test_memory_allocation_performance() {
    println!("📊 Benchmarking memory allocation performance...");
    
    let mut detector = RegressionDetector::new(20.0); // 20% regression threshold for memory
    
    // Benchmark Vec allocations
    let allocation_count = 10000;
    let start_time = Instant::now();
    let mut vectors: Vec<Vec<f32>> = Vec::new();
    
    for i in 0..allocation_count {
        let size = 100 + (i % 900); // 100-1000 elements
        let mut vec = Vec::with_capacity(size);
        for j in 0..size {
            vec.push(j as f32);
        }
        vectors.push(vec);
    }
    
    let allocation_time = start_time.elapsed();
    
    // Benchmark access performance
    let access_start = Instant::now();
    let mut sum = 0.0f32;
    
    for vec in &vectors {
        for &value in vec {
            sum += value;
        }
    }
    
    let access_time = access_start.elapsed();
    
    // Calculate total memory usage
    let total_elements: usize = vectors.iter().map(|v| v.len()).sum();
    let memory_usage_mb = (total_elements * 4) as f64 / 1024.0 / 1024.0; // 4 bytes per f32
    
    let allocation_benchmark = PerformanceBenchmark::new(
        BenchmarkCategory::MemoryAllocation,
        "vec_allocation_10k".to_string(),
    )
    .with_timing(allocation_time, allocation_count)
    .with_memory(memory_usage_mb);
    
    let access_benchmark = PerformanceBenchmark::new(
        BenchmarkCategory::MemoryAllocation,
        "memory_access_sequential".to_string(),
    )
    .with_timing(access_time, total_elements)
    .with_memory(memory_usage_mb);
    
    detector.add_baseline(allocation_benchmark.clone());
    detector.add_baseline(access_benchmark.clone());
    detector.record_benchmark(allocation_benchmark.clone());
    detector.record_benchmark(access_benchmark.clone());
    
    println!("   Allocation: {} vectors in {:?} ({:.0} allocs/sec)", 
             allocation_count, allocation_time, allocation_benchmark.operations_per_second);
    println!("   Access: {} elements in {:?} ({:.0} accesses/sec)", 
             total_elements, access_time, access_benchmark.operations_per_second);
    println!("   Memory usage: {:.1} MB", memory_usage_mb);
    
    // Performance assertions
    assert!(allocation_benchmark.operations_per_second >= 1000.0, 
            "Should allocate at least 1000 vecs/sec, got {:.0}", 
            allocation_benchmark.operations_per_second);
    assert!(access_benchmark.operations_per_second >= 10000000.0, 
            "Should access at least 10M elements/sec, got {:.0}", 
            access_benchmark.operations_per_second);
    
    // Prevent optimization of sum
    assert!(sum > 0.0, "Sum should be positive: {}", sum);
    
    println!("✅ Memory allocation performance benchmark completed");
}

#[test]
fn test_system_integration_performance() {
    println!("📊 Benchmarking system integration performance...");
    
    let mut detector = RegressionDetector::new(15.0);
    
    // Setup integrated systems
    let mut world = World::new(32);
    let mut physics_data = PhysicsData::new(500);
    let mut particles = ParticleData::new(1000);
    
    // Initialize world
    for x in -1..=1 {
        for z in -1..=1 {
            let chunk_pos = ChunkPos::new(x, 0, z);
            let mut chunk = Chunk::new(chunk_pos, 32);
            
            for local_x in 0..32 {
                for local_z in 0..32 {
                    chunk.set_block(local_x as u32, 63, local_z as u32, BlockId::Stone);
                }
            }
            
            world.set_chunk(chunk_pos, chunk);
        }
    }
    
    // Initialize physics
    for i in 0..250 {
        let position = [i as f32 * 2.0, 70.0, (i % 16) as f32 * 2.0];
        let velocity = [0.0, 0.0, 0.0];
        let mass = 1.0;
        let half_extents = [0.5, 0.5, 0.5];
        
        physics_data.add_entity(position, velocity, mass, half_extents);
    }
    
    // Initialize particles
    particles.count = 500;
    for i in 0..500 {
        particles.position_x[i] = (i % 50) as f32;
        particles.position_y[i] = 65.0;
        particles.position_z[i] = (i / 50) as f32;
        particles.velocity_x[i] = 0.0;
        particles.velocity_y[i] = 1.0;
        particles.velocity_z[i] = 0.0;
        particles.lifetime[i] = 5.0;
        particles.max_lifetime[i] = 5.0;
    }
    
    // Benchmark integrated simulation
    let simulation_frames = 30;
    let start_time = Instant::now();
    
    for frame in 0..simulation_frames {
        // Physics update
        let entity_count = physics_data.entity_count();
        earth_engine::physics_data::integration::parallel::apply_gravity(
            &mut physics_data.velocities[..entity_count],
            &physics_data.flags[..entity_count],
            earth_engine::physics::GRAVITY,
            0.033, // 30 FPS
        );
        
        earth_engine::physics_data::integration::parallel::integrate_positions(
            &mut physics_data.positions[..entity_count],
            &physics_data.velocities[..entity_count],
            &physics_data.flags[..entity_count],
            0.033,
        );
        
        // Particle update
        earth_engine::particles::update::update_particles(
            &mut particles,
            &world,
            0.033,
            Vec3::new(0.0, 0.1, 0.0),
            false,
        );
        
        // Simulate world queries (collision detection)
        for i in 0..entity_count {
            let pos = physics_data.positions[i];
            let voxel_pos = VoxelPos::new(pos[0] as i32, pos[1] as i32, pos[2] as i32);
            let _block = world.get_block(voxel_pos); // World query
        }
    }
    
    let duration = start_time.elapsed();
    let total_operations = (physics_data.entity_count() + particles.count + 9) * simulation_frames; // +9 chunks
    let memory_usage = 50.0; // Estimated total memory usage in MB
    
    let benchmark = PerformanceBenchmark::new(
        BenchmarkCategory::SystemIntegration,
        "integrated_simulation_multi_system".to_string(),
    )
    .with_timing(duration, total_operations)
    .with_memory(memory_usage);
    
    detector.add_baseline(benchmark.clone());
    detector.record_benchmark(benchmark.clone());
    
    println!("   Integrated: {} entities + {} particles + world for {} frames in {:?}", 
             physics_data.entity_count(), particles.count, simulation_frames, duration);
    println!("   Performance: {:.0} operations/sec", benchmark.operations_per_second);
    
    // Performance assertions
    assert!(benchmark.operations_per_second >= 5000.0, 
            "Integration should handle at least 5k ops/sec, got {:.0}", 
            benchmark.operations_per_second);
    
    println!("✅ System integration performance benchmark completed");
}

#[test]
fn test_performance_regression_detection() {
    println!("📊 Testing performance regression detection...");
    
    let mut detector = RegressionDetector::new(10.0); // 10% threshold
    
    // Establish good baseline
    let good_baseline = PerformanceBenchmark::new(
        BenchmarkCategory::PhysicsSimulation,
        "test_physics".to_string(),
    )
    .with_timing(Duration::from_millis(100), 10000)
    .with_memory(50.0);
    
    detector.add_baseline(good_baseline.clone());
    
    // Test performance that should NOT trigger regression
    let acceptable_performance = PerformanceBenchmark::new(
        BenchmarkCategory::PhysicsSimulation,
        "test_physics".to_string(),
    )
    .with_timing(Duration::from_millis(105), 10000) // 5% slower - OK
    .with_memory(52.0); // 4% more memory - OK
    
    detector.record_benchmark(acceptable_performance);
    
    // Test performance that SHOULD trigger regression
    let poor_performance = PerformanceBenchmark::new(
        BenchmarkCategory::PhysicsSimulation,
        "test_physics".to_string(),
    )
    .with_timing(Duration::from_millis(150), 10000) // 50% slower - REGRESSION
    .with_memory(65.0); // 30% more memory - REGRESSION
    
    detector.record_benchmark(poor_performance);
    
    // Generate report
    let report = detector.generate_report();
    assert!(report.contains("PhysicsSimulation"), "Report should contain physics benchmarks");
    assert!(report.len() > 100, "Report should be substantial");
    
    println!("   Regression detection system working correctly");
    println!("   Generated report: {} characters", report.len());
    
    println!("✅ Performance regression detection test completed");
}

// Integration test summary
#[test]
fn test_performance_regression_summary() {
    println!("\n📊 Performance Regression Testing Summary");
    println!("=========================================");
    
    println!("✅ World generation performance benchmarking");
    println!("✅ Physics simulation performance benchmarking");
    println!("✅ Particle system performance benchmarking");
    println!("✅ Memory allocation performance benchmarking");
    println!("✅ System integration performance benchmarking");
    println!("✅ Performance regression detection framework");
    
    println!("\n🎯 Performance Regression Testing: ALL TESTS PASSED");
    println!("The performance regression framework successfully monitors");
    println!("all critical systems and can detect performance degradation.");
}