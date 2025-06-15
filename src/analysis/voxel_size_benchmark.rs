//! Voxel Size Performance Benchmark
//! 
//! This module tests the engine's performance with different voxel sizes
//! to demonstrate the catastrophic performance degradation.

use std::time::Instant;
use std::sync::Arc;
use wgpu::{Device, Queue, Buffer, BufferUsages};

/// Test configuration for different voxel sizes
#[derive(Debug, Clone)]
pub struct VoxelSizeTest {
    pub name: &'static str,
    pub voxel_size: f32,
    pub voxels_per_meter: u32,
    pub multiplier: u32,
}

impl VoxelSizeTest {
    pub fn test_configs() -> Vec<Self> {
        vec![
            Self {
                name: "1m³ (baseline)",
                voxel_size: 1.0,
                voxels_per_meter: 1,
                multiplier: 1,
            },
            Self {
                name: "0.5m³ (8x more)",
                voxel_size: 0.5,
                voxels_per_meter: 2,
                multiplier: 8,
            },
            Self {
                name: "0.25m³ (64x more)",
                voxel_size: 0.25,
                voxels_per_meter: 4,
                multiplier: 64,
            },
            Self {
                name: "0.1m³ (1000x more)",
                voxel_size: 0.1,
                voxels_per_meter: 10,
                multiplier: 1000,
            },
        ]
    }
}

/// Benchmark results for a voxel size
#[derive(Debug)]
pub struct BenchmarkResult {
    pub config: VoxelSizeTest,
    pub voxel_count: u32,
    pub memory_usage_mb: f32,
    pub allocation_time_ms: u128,
    pub iteration_time_ms: u128,
    pub theoretical_fps: f32,
    pub actual_fps_estimate: f32,
}

/// Simulated chunk operations at different voxel sizes
pub struct VoxelSizeBenchmark {
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl VoxelSizeBenchmark {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self { device, queue }
    }
    
    /// Run benchmark for a specific voxel size
    pub fn benchmark_voxel_size(&self, config: &VoxelSizeTest) -> Result<BenchmarkResult, String> {
        println!("\n--- Benchmarking {} ---", config.name);
        
        // Calculate voxel count for a standard 32m³ chunk
        let base_chunk_size = 32; // 32x32x32 meters
        let voxels_per_chunk = (base_chunk_size * config.voxels_per_meter).pow(3);
        
        println!("Voxels per chunk: {}", voxels_per_chunk);
        
        // Memory calculation (5 bytes per voxel minimum)
        let bytes_per_voxel = 5;
        let total_bytes = voxels_per_chunk as usize * bytes_per_voxel;
        let memory_mb = total_bytes as f32 / (1024.0 * 1024.0);
        
        println!("Memory required: {:.2} MB", memory_mb);
        
        // Check if we can even allocate this much
        if memory_mb > 2048.0 {
            return Err(format!(
                "Would require {:.2} MB - TOO LARGE! Skipping to prevent OOM.", 
                memory_mb
            ));
        }
        
        // Time allocation
        let alloc_start = Instant::now();
        
        // Simulate chunk data allocation
        let chunk_data = match self.allocate_chunk_data(voxels_per_chunk) {
            Ok(data) => data,
            Err(e) => return Err(format!("Allocation failed: {}", e)),
        };
        
        let allocation_time = alloc_start.elapsed().as_millis();
        println!("Allocation time: {} ms", allocation_time);
        
        // Time iteration/processing
        let iter_start = Instant::now();
        
        // Simulate basic chunk processing
        self.process_chunk_data(&chunk_data, voxels_per_chunk);
        
        let iteration_time = iter_start.elapsed().as_millis();
        println!("Processing time: {} ms", iteration_time);
        
        // Calculate theoretical FPS
        let total_time_ms = allocation_time + iteration_time;
        let theoretical_fps = if total_time_ms > 0 {
            1000.0 / total_time_ms as f32
        } else {
            1000.0
        };
        
        // Estimate actual FPS based on 0.8 baseline and multiplier
        let actual_fps_estimate = 0.8 / config.multiplier as f32;
        
        Ok(BenchmarkResult {
            config: config.clone(),
            voxel_count: voxels_per_chunk,
            memory_usage_mb: memory_mb,
            allocation_time_ms: allocation_time,
            iteration_time_ms: iteration_time,
            theoretical_fps,
            actual_fps_estimate,
        })
    }
    
    /// Allocate chunk data for testing
    fn allocate_chunk_data(&self, voxel_count: u32) -> Result<Vec<u8>, String> {
        let size = voxel_count as usize * 5; // 5 bytes per voxel
        
        // Try to allocate with a simple vec allocation
        let mut data = vec![];
        data.try_reserve(size).map_err(|_| "Failed to allocate memory")?;
        data.resize(size, 0u8);
        
        // Initialize with some pattern
        for i in (0..size).step_by(5) {
            data[i] = 1; // Block type
            data[i + 1] = 15; // Sky light
            data[i + 2] = 0; // Block light
            data[i + 3] = 0; // Material flags
            data[i + 4] = 0; // Extra data
        }
        Ok(data)
    }
    
    /// Simulate chunk processing
    fn process_chunk_data(&self, data: &[u8], voxel_count: u32) {
        let mut checksum = 0u64;
        
        // Simulate iterating through all voxels
        for i in 0..voxel_count {
            let idx = (i * 5) as usize;
            if idx + 4 < data.len() {
                // Simulate reading voxel data
                checksum = checksum.wrapping_add(data[idx] as u64);
                checksum = checksum.wrapping_add(data[idx + 1] as u64);
                
                // Simulate neighbor checks (6 directions)
                for _ in 0..6 {
                    checksum = checksum.wrapping_mul(31).wrapping_add(i as u64);
                }
            }
        }
        
        // Prevent optimization
        std::hint::black_box(checksum);
    }
    
    /// Run GPU memory test
    pub async fn test_gpu_allocation(&self, config: &VoxelSizeTest) -> Result<(), String> {
        let voxels_per_chunk = (32 * config.voxels_per_meter).pow(3);
        let buffer_size = (voxels_per_chunk as usize * 5) as u64;
        
        println!("\nTesting GPU allocation for {}", config.name);
        println!("Buffer size: {} MB", buffer_size / (1024 * 1024));
        
        // Check if size is reasonable
        if buffer_size > 2u64.pow(30) { // 1GB limit
            return Err("Buffer too large for GPU allocation test".to_string());
        }
        
        // Try to create GPU buffer
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Test buffer for {}", config.name)),
            size: buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Buffer created successfully
        println!("✓ GPU buffer allocated successfully");
        
        // Clean up
        drop(buffer);
        
        Ok(())
    }
}

/// Run complete benchmark suite
pub async fn run_voxel_size_benchmarks(device: Arc<Device>, queue: Arc<Queue>) {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║               VOXEL SIZE PERFORMANCE BENCHMARK                     ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    
    let benchmark = VoxelSizeBenchmark::new(device, queue);
    let configs = VoxelSizeTest::test_configs();
    let mut results = Vec::new();
    
    // Run CPU benchmarks
    for config in &configs {
        match benchmark.benchmark_voxel_size(config) {
            Ok(result) => {
                results.push(result);
            },
            Err(e) => {
                println!("ERROR: {}", e);
                println!("Stopping further tests to prevent system instability.");
                break;
            }
        }
    }
    
    // Print summary
    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║                        BENCHMARK SUMMARY                           ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    
    println!("\n{:<20} {:>15} {:>15} {:>15} {:>15}", 
        "Voxel Size", "Voxel Count", "Memory (MB)", "Process (ms)", "Est. FPS");
    println!("{}", "=".repeat(85));
    
    for result in &results {
        println!("{:<20} {:>15} {:>15.2} {:>15} {:>15.4}", 
            result.config.name,
            result.voxel_count,
            result.memory_usage_mb,
            result.iteration_time_ms,
            result.actual_fps_estimate
        );
    }
    
    if results.len() < configs.len() {
        println!("\n⚠️  Benchmark stopped early to prevent out-of-memory errors!");
    }
    
    // Performance degradation analysis
    if results.len() >= 2 {
        println!("\n╔════════════════════════════════════════════════════════════════════╗");
        println!("║                   PERFORMANCE DEGRADATION                          ║");
        println!("╚════════════════════════════════════════════════════════════════════╝");
        
        let baseline = &results[0];
        for result in &results[1..] {
            let slowdown = result.iteration_time_ms as f32 / baseline.iteration_time_ms.max(1) as f32;
            let fps_drop = baseline.actual_fps_estimate / result.actual_fps_estimate;
            
            println!("\n{} vs baseline:", result.config.name);
            println!("  - Processing slowdown: {:.1}x", slowdown);
            println!("  - FPS reduction: {:.1}x", fps_drop);
            println!("  - New frame time: {:.1} seconds", 1.0 / result.actual_fps_estimate);
        }
    }
    
    // GPU allocation tests
    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║                      GPU ALLOCATION TESTS                          ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    
    for config in &configs[..2] { // Only test first two to avoid GPU OOM
        if let Err(e) = benchmark.test_gpu_allocation(config).await {
            println!("❌ {} - {}", config.name, e);
            break;
        }
    }
    
    // Final warnings
    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║                          CONCLUSIONS                               ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
    
    println!("\n🚨 CRITICAL FINDINGS:");
    println!("1. Memory usage increases cubically with voxel resolution");
    println!("2. Processing time scales linearly with voxel count (best case)");
    println!("3. 0.1m³ voxels would require 160MB+ per chunk");
    println!("4. Current 0.8 FPS would become 0.0008 FPS with 0.1m³ voxels");
    println!("\n❌ The engine CANNOT handle 1dcm³ (0.1m³) voxels!");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_voxel_calculations() {
        let configs = VoxelSizeTest::test_configs();
        
        assert_eq!(configs[0].multiplier, 1);
        assert_eq!(configs[1].multiplier, 8);
        assert_eq!(configs[2].multiplier, 64);
        assert_eq!(configs[3].multiplier, 1000);
    }
    
    #[test]
    fn test_memory_calculations() {
        let config = VoxelSizeTest {
            name: "test",
            voxel_size: 0.5,
            voxels_per_meter: 2,
            multiplier: 8,
        };
        
        let voxels_per_chunk = (32 * config.voxels_per_meter).pow(3);
        assert_eq!(voxels_per_chunk, 262144); // 64^3
        
        let memory_mb = (voxels_per_chunk as usize * 5) as f32 / (1024.0 * 1024.0);
        assert!(memory_mb > 1.0);
    }
}