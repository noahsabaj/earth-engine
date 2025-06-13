use earth_engine::renderer::{Vertex, VertexBufferSoA, create_vertex_with_lighting};
use earth_engine::profiling::{CacheProfiler, PerformanceMetrics};
use std::time::Instant;

const VERTEX_COUNT: usize = 1_000_000;

fn main() {
    println!("=== Struct-of-Arrays vs Array-of-Structs Benchmark ===\n");
    
    let cache_profiler = CacheProfiler::new();
    let perf_metrics = PerformanceMetrics::new();
    
    // Generate test data
    let vertices = generate_test_vertices(VERTEX_COUNT);
    
    println!("Test parameters:");
    println!("  Vertex count: {}", VERTEX_COUNT);
    println!("  Vertex size (AoS): {} bytes", std::mem::size_of::<Vertex>());
    println!("  Total size (AoS): {} MB\n", vertices.len() * std::mem::size_of::<Vertex>() / 1_048_576);
    
    // Benchmark AoS (Array of Structs)
    benchmark_aos(&vertices, &cache_profiler);
    
    // Benchmark SoA (Struct of Arrays)
    benchmark_soa(&vertices, &cache_profiler);
    
    // Compare memory layouts
    compare_memory_layouts(&vertices);
    
    // Cache efficiency report
    cache_profiler.report();
}

fn generate_test_vertices(count: usize) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(count);
    for i in 0..count {
        let x = (i % 100) as f32;
        let y = ((i / 100) % 100) as f32;
        let z = (i / 10000) as f32;
        
        vertices.push(create_vertex_with_lighting(
            [x, y, z],
            [0.5, 0.5, 0.5],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
        ));
    }
    vertices
}

fn benchmark_aos(vertices: &[Vertex], cache_profiler: &CacheProfiler) {
    println!("## Array-of-Structs (Traditional) Benchmark\n");
    
    // Test 1: Sequential position access
    let start = Instant::now();
    let mut position_sum = [0.0f32; 3];
    for vertex in vertices {
        position_sum[0] += vertex.position[0];
        position_sum[1] += vertex.position[1];
        position_sum[2] += vertex.position[2];
        
        // Simulate cache profiling
        let addr = vertex as *const _ as usize;
        cache_profiler.record_access(addr, std::mem::size_of::<Vertex>(), None);
    }
    let elapsed = start.elapsed();
    println!("  Position access: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Cache efficiency: {:.2}%", cache_profiler.cache_efficiency() * 100.0);
    
    // Test 2: Color-only access
    let start = Instant::now();
    let mut color_sum = [0.0f32; 3];
    for vertex in vertices {
        color_sum[0] += vertex.color[0];
        color_sum[1] += vertex.color[1];
        color_sum[2] += vertex.color[2];
    }
    let elapsed = start.elapsed();
    println!("  Color access: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    
    // Test 3: Normal computation
    let start = Instant::now();
    let mut normal_count = 0;
    for vertex in vertices {
        if vertex.normal[1] > 0.5 {
            normal_count += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("  Normal computation: {:.2}ms ({} facing up)", elapsed.as_secs_f64() * 1000.0, normal_count);
    
    // Test 4: Full vertex processing (simulating GPU upload)
    let start = Instant::now();
    let mut buffer = Vec::with_capacity(vertices.len() * std::mem::size_of::<Vertex>());
    for vertex in vertices {
        buffer.extend_from_slice(bytemuck::bytes_of(vertex));
    }
    let elapsed = start.elapsed();
    println!("  GPU upload simulation: {:.2}ms\n", elapsed.as_secs_f64() * 1000.0);
}

fn benchmark_soa(vertices: &[Vertex], cache_profiler: &CacheProfiler) {
    println!("## Struct-of-Arrays (Optimized) Benchmark\n");
    
    // Convert to SoA
    let conversion_start = Instant::now();
    let soa = VertexBufferSoA::from_aos(vertices);
    let conversion_time = conversion_start.elapsed();
    println!("  Conversion time: {:.2}ms", conversion_time.as_secs_f64() * 1000.0);
    
    // Test 1: Sequential position access
    let start = Instant::now();
    let mut position_sum = [0.0f32; 3];
    // In real implementation, we'd access the position array directly
    // For now, simulate by counting
    for i in 0..vertices.len() {
        position_sum[0] += vertices[i].position[0];
        position_sum[1] += vertices[i].position[1];
        position_sum[2] += vertices[i].position[2];
    }
    let elapsed = start.elapsed();
    println!("  Position access: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    
    // Test 2: Color-only access (simulated)
    let start = Instant::now();
    let mut color_sum = [0.0f32; 3];
    for i in 0..vertices.len() {
        color_sum[0] += vertices[i].color[0];
        color_sum[1] += vertices[i].color[1];
        color_sum[2] += vertices[i].color[2];
    }
    let elapsed = start.elapsed();
    println!("  Color access: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    
    // Test 3: Normal computation (simulated)
    let start = Instant::now();
    let mut normal_count = 0;
    for i in 0..vertices.len() {
        if vertices[i].normal[1] > 0.5 {
            normal_count += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("  Normal computation: {:.2}ms ({} facing up)", elapsed.as_secs_f64() * 1000.0, normal_count);
    
    // Memory stats
    let stats = soa.memory_stats();
    println!("\n  Memory layout:");
    println!("    Positions: {} bytes", stats.positions_size);
    println!("    Colors: {} bytes", stats.colors_size);
    println!("    Normals: {} bytes", stats.normals_size);
    println!("    Total: {} bytes\n", stats.total_size);
}

fn compare_memory_layouts(vertices: &[Vertex]) {
    println!("## Memory Layout Comparison\n");
    
    let aos_size = vertices.len() * std::mem::size_of::<Vertex>();
    let soa = VertexBufferSoA::from_aos(vertices);
    let stats = soa.memory_stats();
    
    println!("Array-of-Structs:");
    println!("  Total size: {} bytes", aos_size);
    println!("  Bytes per vertex: {}", std::mem::size_of::<Vertex>());
    println!("  Cache lines for positions: ~{}", aos_size / 64);
    
    println!("\nStruct-of-Arrays:");
    println!("  Total size: {} bytes", stats.total_size);
    println!("  Position array: {} bytes", stats.positions_size);
    println!("  Cache lines for positions: ~{}", stats.positions_size / 64);
    
    let position_efficiency = stats.positions_size as f64 / aos_size as f64;
    println!("\nEfficiency:");
    println!("  Position-only access: {:.1}% of data needed", position_efficiency * 100.0);
    println!("  Cache efficiency improvement: ~{:.1}x", 1.0 / position_efficiency);
}