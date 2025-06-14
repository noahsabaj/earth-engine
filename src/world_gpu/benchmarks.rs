use std::sync::Arc;
use std::time::Instant;
use crate::world::ChunkPos;
use super::{
    WorldBuffer, WorldBufferDescriptor,
    TerrainGenerator, TerrainParams,
    ChunkModifier, ModificationCommand,
    GpuLighting,
};

/// Performance benchmarking for GPU world system
pub struct GpuWorldBenchmarks {
    device: Arc<wgpu::Device>,
    queue: wgpu::Queue,
}

impl GpuWorldBenchmarks {
    pub fn new(device: Arc<wgpu::Device>, queue: wgpu::Queue) -> Self {
        Self { device, queue }
    }
    
    /// Run all benchmarks and print results
    pub fn run_all_benchmarks(&self) {
        println!("=== GPU World Performance Benchmarks ===\n");
        
        self.benchmark_terrain_generation();
        self.benchmark_chunk_modifications();
        self.benchmark_ambient_occlusion();
        self.benchmark_memory_throughput();
        
        println!("\n=== Benchmark Complete ===");
    }
    
    /// Benchmark terrain generation performance
    fn benchmark_terrain_generation(&self) {
        println!("## Terrain Generation Benchmark");
        
        let world_buffer = WorldBuffer::new(self.device.clone(), &WorldBufferDescriptor {
            view_distance: 8,
            enable_atomics: true,
            enable_readback: false,
        });
        
        let terrain_gen = TerrainGenerator::new(self.device.clone());
        terrain_gen.update_params(&self.queue, &TerrainParams::default());
        
        // Test different batch sizes
        let batch_sizes = [1, 10, 50, 100, 500, 1000];
        
        for &batch_size in &batch_sizes {
            let chunks: Vec<ChunkPos> = (0..batch_size)
                .map(|i| ChunkPos {
                    x: (i % 32) as i32,
                    y: 0,
                    z: (i / 32) as i32,
                })
                .collect();
            
            let start = Instant::now();
            
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Benchmark Encoder"),
            });
            
            terrain_gen.generate_chunks(&mut encoder, &world_buffer, &chunks);
            
            self.queue.submit(std::iter::once(encoder.finish()));
            self.device.poll(wgpu::Maintain::Wait);
            
            let elapsed = start.elapsed();
            let chunks_per_second = batch_size as f64 / elapsed.as_secs_f64();
            let ms_per_chunk = elapsed.as_millis() as f64 / batch_size as f64;
            
            println!("  Batch size {:4}: {:.2} chunks/sec ({:.2} ms/chunk)", 
                     batch_size, chunks_per_second, ms_per_chunk);
        }
        
        // Calculate voxels per second for best batch size
        let voxels_per_chunk = 32 * 32 * 32;
        let best_batch = 1000;
        let chunks: Vec<ChunkPos> = (0..best_batch)
            .map(|i| ChunkPos {
                x: (i % 32) as i32,
                y: 0,
                z: (i / 32) as i32,
            })
            .collect();
        
        let start = Instant::now();
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: None,
        });
        terrain_gen.generate_chunks(&mut encoder, &world_buffer, &chunks);
        self.queue.submit(std::iter::once(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);
        let elapsed = start.elapsed();
        
        let total_voxels = best_batch * voxels_per_chunk;
        let voxels_per_second = total_voxels as f64 / elapsed.as_secs_f64();
        println!("  Peak performance: {:.2}M voxels/sec", voxels_per_second / 1_000_000.0);
        println!();
    }
    
    /// Benchmark chunk modification performance
    fn benchmark_chunk_modifications(&self) {
        println!("## Chunk Modification Benchmark");
        
        let world_buffer = WorldBuffer::new(self.device.clone(), &WorldBufferDescriptor {
            view_distance: 8,
            enable_atomics: true,
            enable_readback: false,
        });
        
        let modifier = ChunkModifier::new(self.device.clone());
        
        // Test different modification counts
        let mod_counts = [100, 1000, 5000, 10000];
        
        for &count in &mod_counts {
            // Create random modifications
            let commands: Vec<ModificationCommand> = (0..count)
                .map(|i| {
                    let x = (i % 1000) as i32;
                    let y = ((i / 1000) % 256) as i32;
                    let z = ((i / 256000) % 1000) as i32;
                    
                    match i % 3 {
                        0 => ModificationCommand::set_block(x, y, z, 1),
                        1 => ModificationCommand::break_block(x, y, z),
                        _ => ModificationCommand::explode(x, y, z, 5.0),
                    }
                })
                .collect();
            
            let start = Instant::now();
            
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: None,
            });
            
            modifier.apply_modifications(&mut encoder, &self.queue, &world_buffer, &commands);
            
            self.queue.submit(std::iter::once(encoder.finish()));
            self.device.poll(wgpu::Maintain::Wait);
            
            let elapsed = start.elapsed();
            let mods_per_second = count as f64 / elapsed.as_secs_f64();
            
            println!("  {} modifications: {:.0} mods/sec ({:.3} ms total)",
                     count, mods_per_second, elapsed.as_millis());
        }
        println!();
    }
    
    /// Benchmark ambient occlusion calculation
    fn benchmark_ambient_occlusion(&self) {
        println!("## Ambient Occlusion Benchmark");
        
        let world_buffer = WorldBuffer::new(self.device.clone(), &WorldBufferDescriptor {
            view_distance: 8,
            enable_atomics: true,
            enable_readback: false,
        });
        
        let lighting = GpuLighting::new(self.device.clone());
        
        // Test different chunk counts and smoothing passes
        let chunk_counts = [10, 50, 100, 500];
        let smooth_passes = [0, 1, 2, 4];
        
        for &chunk_count in &chunk_counts {
            let chunks: Vec<ChunkPos> = (0..chunk_count)
                .map(|i| ChunkPos {
                    x: (i % 16) as i32,
                    y: 0,
                    z: (i / 16) as i32,
                })
                .collect();
            
            for &passes in &smooth_passes {
                let start = Instant::now();
                
                let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: None,
                });
                
                lighting.calculate_ambient_occlusion(&mut encoder, &world_buffer, &chunks, passes);
                
                self.queue.submit(std::iter::once(encoder.finish()));
                self.device.poll(wgpu::Maintain::Wait);
                
                let elapsed = start.elapsed();
                let ms_per_chunk = elapsed.as_millis() as f64 / chunk_count as f64;
                
                println!("  {} chunks, {} passes: {:.2} ms/chunk",
                         chunk_count, passes, ms_per_chunk);
            }
        }
        println!();
    }
    
    /// Benchmark memory throughput
    fn benchmark_memory_throughput(&self) {
        println!("## Memory Throughput Benchmark");
        
        // Create buffers of different sizes
        let sizes_mb = [1, 10, 100, 500];
        
        for &size_mb in &sizes_mb {
            let size_bytes = size_mb * 1024 * 1024;
            
            // Create source and destination buffers
            let src_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Throughput Test Src"),
                size: size_bytes as u64,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            
            let dst_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Throughput Test Dst"),
                size: size_bytes as u64,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            
            // Measure copy time
            let start = Instant::now();
            
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: None,
            });
            
            encoder.copy_buffer_to_buffer(&src_buffer, 0, &dst_buffer, 0, size_bytes as u64);
            
            self.queue.submit(std::iter::once(encoder.finish()));
            self.device.poll(wgpu::Maintain::Wait);
            
            let elapsed = start.elapsed();
            let throughput_gb_s = size_mb as f64 / 1024.0 / elapsed.as_secs_f64();
            
            println!("  {} MB: {:.2} GB/s", size_mb, throughput_gb_s);
        }
        println!();
    }
}

/// Performance comparison results
pub struct PerformanceComparison {
    pub cpu_baseline: PerformanceMetrics,
    pub gpu_optimized: PerformanceMetrics,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub terrain_gen_chunks_per_sec: f64,
    pub modifications_per_sec: f64,
    pub lighting_chunks_per_sec: f64,
    pub memory_usage_mb: f64,
    pub power_efficiency: f64, // Operations per watt
}

impl PerformanceComparison {
    pub fn calculate_improvements(&self) -> PerformanceImprovements {
        PerformanceImprovements {
            terrain_gen_speedup: self.gpu_optimized.terrain_gen_chunks_per_sec / 
                                self.cpu_baseline.terrain_gen_chunks_per_sec,
            modification_speedup: self.gpu_optimized.modifications_per_sec / 
                                 self.cpu_baseline.modifications_per_sec,
            lighting_speedup: self.gpu_optimized.lighting_chunks_per_sec / 
                             self.cpu_baseline.lighting_chunks_per_sec,
            memory_reduction: 1.0 - (self.gpu_optimized.memory_usage_mb / 
                                    self.cpu_baseline.memory_usage_mb),
            power_efficiency_gain: self.gpu_optimized.power_efficiency / 
                                  self.cpu_baseline.power_efficiency,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceImprovements {
    pub terrain_gen_speedup: f64,
    pub modification_speedup: f64,
    pub lighting_speedup: f64,
    pub memory_reduction: f64,
    pub power_efficiency_gain: f64,
}

impl PerformanceImprovements {
    pub fn print_summary(&self) {
        println!("=== GPU vs CPU Performance Comparison ===");
        println!("Terrain Generation: {:.1}x faster", self.terrain_gen_speedup);
        println!("World Modifications: {:.1}x faster", self.modification_speedup);
        println!("Lighting Calculations: {:.1}x faster", self.lighting_speedup);
        println!("Memory Usage: {:.1}% reduction", self.memory_reduction * 100.0);
        println!("Power Efficiency: {:.1}x better", self.power_efficiency_gain);
        println!();
        
        let avg_speedup = (self.terrain_gen_speedup + self.modification_speedup + 
                          self.lighting_speedup) / 3.0;
        println!("Average Performance Gain: {:.1}x", avg_speedup);
    }
}

/// Expected performance metrics based on GPU architecture
pub fn get_expected_performance() -> PerformanceComparison {
    PerformanceComparison {
        cpu_baseline: PerformanceMetrics {
            terrain_gen_chunks_per_sec: 50.0,
            modifications_per_sec: 10_000.0,
            lighting_chunks_per_sec: 20.0,
            memory_usage_mb: 4096.0,
            power_efficiency: 100.0,
        },
        gpu_optimized: PerformanceMetrics {
            terrain_gen_chunks_per_sec: 5000.0,
            modifications_per_sec: 1_000_000.0,
            lighting_chunks_per_sec: 2000.0,
            memory_usage_mb: 2048.0,
            power_efficiency: 500.0,
        },
    }
}