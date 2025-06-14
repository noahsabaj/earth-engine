/// Unified Kernel Benchmark
/// 
/// Sprint 34: Performance testing for 1000x target

use std::sync::Arc;
use std::time::Instant;
use wgpu::{Device, Queue};
use crate::memory::{MemoryManager, PerformanceMetrics};
use super::{
    WorldBuffer, WorldBufferDescriptor,
    UnifiedWorldKernel, UnifiedKernelConfig, SystemFlags,
    SparseVoxelOctree, VoxelBvh,
};
use crate::world::ChunkPos;

/// Benchmark results
#[derive(Debug, Clone)]
pub struct UnifiedBenchmarkResults {
    /// Time for traditional multi-pass approach
    pub traditional_time_ms: f64,
    
    /// Time for unified kernel approach
    pub unified_time_ms: f64,
    
    /// Speedup factor
    pub speedup_factor: f64,
    
    /// Individual system times (traditional)
    pub terrain_gen_ms: f64,
    pub lighting_ms: f64,
    pub physics_ms: f64,
    pub modifications_ms: f64,
    
    /// GPU dispatch count
    pub traditional_dispatches: u32,
    pub unified_dispatches: u32,
    
    /// Memory bandwidth
    pub traditional_bandwidth_gb: f64,
    pub unified_bandwidth_gb: f64,
}

/// Unified kernel benchmark
pub struct UnifiedKernelBenchmark {
    device: Arc<Device>,
    queue: Arc<Queue>,
    memory_manager: MemoryManager,
    performance_metrics: PerformanceMetrics,
}

impl UnifiedKernelBenchmark {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let memory_config = crate::memory::MemoryConfig {
            enable_profiling: true,
            ..Default::default()
        };
        
        let memory_manager = MemoryManager::new(device.clone(), memory_config);
        let performance_metrics = PerformanceMetrics::new();
        
        Self {
            device,
            queue,
            memory_manager,
            performance_metrics,
        }
    }
    
    /// Run the benchmark
    pub async fn run_benchmark(
        &mut self,
        world_size: u32,
        active_chunks: u32,
        iterations: u32,
    ) -> UnifiedBenchmarkResults {
        println!("=== Unified Kernel Benchmark ===");
        println!("World size: {}³ chunks", world_size);
        println!("Active chunks: {}", active_chunks);
        println!("Iterations: {}", iterations);
        println!();
        
        // Create world buffer  
        let world_buffer = WorldBuffer::new(
            self.device.clone(),
            &WorldBufferDescriptor {
                view_distance: world_size.min(16), // Use world_size as view_distance but cap at 16
                enable_atomics: true,
                enable_readback: false,
            },
        );
        
        // Generate active chunk positions
        let chunk_positions: Vec<ChunkPos> = (0..active_chunks)
            .map(|i| ChunkPos {
                x: (i % world_size) as i32,
                y: ((i / world_size) % world_size) as i32,
                z: (i / (world_size * world_size)) as i32,
            })
            .collect();
        
        // Benchmark traditional approach
        let traditional_results = self.benchmark_traditional(
            &world_buffer,
            &chunk_positions,
            iterations,
        ).await;
        
        // Benchmark unified kernel
        let unified_results = self.benchmark_unified(
            &world_buffer,
            &chunk_positions,
            iterations,
        ).await;
        
        // Calculate results
        let speedup_factor = traditional_results.total_ms / unified_results.total_ms;
        
        UnifiedBenchmarkResults {
            traditional_time_ms: traditional_results.total_ms,
            unified_time_ms: unified_results.total_ms,
            speedup_factor,
            terrain_gen_ms: traditional_results.terrain_ms,
            lighting_ms: traditional_results.lighting_ms,
            physics_ms: traditional_results.physics_ms,
            modifications_ms: traditional_results.modifications_ms,
            traditional_dispatches: traditional_results.dispatches,
            unified_dispatches: unified_results.dispatches,
            traditional_bandwidth_gb: traditional_results.bandwidth_gb,
            unified_bandwidth_gb: unified_results.bandwidth_gb,
        }
    }
    
    /// Benchmark traditional multi-pass approach
    async fn benchmark_traditional(
        &mut self,
        world_buffer: &WorldBuffer,
        chunk_positions: &[ChunkPos],
        iterations: u32,
    ) -> TraditionalResults {
        println!("Benchmarking traditional approach...");
        
        let mut total_time = 0.0;
        let mut terrain_time = 0.0;
        let mut lighting_time = 0.0;
        let mut physics_time = 0.0;
        let mut modifications_time = 0.0;
        let mut total_dispatches = 0u32;
        
        // Create individual systems
        let terrain_gen = super::TerrainGenerator::new(self.device.clone());
        let lighting = super::GpuLighting::new(self.device.clone());
        let modifier = super::ChunkModifier::new(self.device.clone());
        
        for i in 0..iterations {
            let frame_start = Instant::now();
            
            // Terrain generation pass
            let terrain_start = Instant::now();
            {
                let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Terrain Generation"),
                });
                
                for chunk_pos in chunk_positions {
                    terrain_gen.generate_chunk(
                        &mut encoder,
                        world_buffer,
                        *chunk_pos,
                    );
                    total_dispatches += 1;
                }
                
                self.queue.submit(std::iter::once(encoder.finish()));
            }
            terrain_time += terrain_start.elapsed().as_secs_f64() * 1000.0;
            
            // Lighting pass
            let lighting_start = Instant::now();
            {
                let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Lighting"),
                });
                
                lighting.batch_update_lighting(
                    &mut encoder,
                    world_buffer,
                    chunk_positions,
                );
                total_dispatches += 1;
                
                self.queue.submit(std::iter::once(encoder.finish()));
            }
            lighting_time += lighting_start.elapsed().as_secs_f64() * 1000.0;
            
            // Physics pass (simulated)
            let physics_start = Instant::now();
            {
                let encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Physics"),
                });
                
                // Simulate physics workload
                total_dispatches += chunk_positions.len() as u32 / 64;
                
                self.queue.submit(std::iter::once(encoder.finish()));
            }
            physics_time += physics_start.elapsed().as_secs_f64() * 1000.0;
            
            // Modifications pass
            let modifications_start = Instant::now();
            {
                let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Modifications"),
                });
                
                // Simulate some modifications
                let commands = vec![
                    super::ModificationCommand::set_block(100, 50, 100, 1),
                    super::ModificationCommand::set_block(101, 50, 100, 1),
                ];
                
                modifier.apply_modifications(
                    &mut encoder,
                    &self.queue,
                    world_buffer,
                    &commands,
                );
                total_dispatches += 1;
                
                self.queue.submit(std::iter::once(encoder.finish()));
            }
            modifications_time += modifications_start.elapsed().as_secs_f64() * 1000.0;
            
            // Wait for GPU
            self.device.poll(wgpu::Maintain::Wait);
            
            total_time += frame_start.elapsed().as_secs_f64() * 1000.0;
        }
        
        // Calculate averages
        let avg_iterations = iterations as f64;
        
        TraditionalResults {
            total_ms: total_time / avg_iterations,
            terrain_ms: terrain_time / avg_iterations,
            lighting_ms: lighting_time / avg_iterations,
            physics_ms: physics_time / avg_iterations,
            modifications_ms: modifications_time / avg_iterations,
            dispatches: total_dispatches / iterations,
            bandwidth_gb: self.estimate_bandwidth_gb(chunk_positions.len(), 4), // 4 passes
        }
    }
    
    /// Benchmark unified kernel approach
    async fn benchmark_unified(
        &mut self,
        world_buffer: &WorldBuffer,
        chunk_positions: &[ChunkPos],
        iterations: u32,
    ) -> UnifiedResults {
        println!("Benchmarking unified kernel...");
        
        // Create unified kernel
        let unified_kernel = UnifiedWorldKernel::new(
            self.device.clone(),
            world_buffer,
            &mut self.memory_manager,
        );
        
        // Create acceleration structures
        let mut octree = SparseVoxelOctree::new(
            self.device.clone(),
            &mut self.memory_manager,
            world_buffer.view_distance(),
        );
        octree.build_from_world(&self.queue, world_buffer, chunk_positions);
        
        let mut bvh = VoxelBvh::new(
            self.device.clone(),
            &mut self.memory_manager,
            chunk_positions.len() as u32,
        );
        bvh.build_from_chunks(&self.queue, chunk_positions, 32.0);
        
        // Build work graph
        unified_kernel.build_work_graph(&self.queue, chunk_positions);
        
        let mut total_time = 0.0;
        let mut total_dispatches = 0u32;
        
        for i in 0..iterations {
            let frame_start = Instant::now();
            
            let config = UnifiedKernelConfig {
                frame_number: i,
                delta_time_ms: 16, // 60 FPS
                world_size: world_buffer.view_distance(),
                active_chunks: chunk_positions.len() as u32,
                physics_substeps: 2,
                lighting_iterations: 3,
                system_flags: SystemFlags::ALL,
                random_seed: i * 12345,
            };
            
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Unified World Update"),
            });
            
            // Single dispatch updates everything!
            unified_kernel.update_world(
                &self.queue,
                &mut encoder,
                config,
                chunk_positions.len() as u32,
            );
            total_dispatches += 1;
            
            self.queue.submit(std::iter::once(encoder.finish()));
            self.device.poll(wgpu::Maintain::Wait);
            
            total_time += frame_start.elapsed().as_secs_f64() * 1000.0;
        }
        
        UnifiedResults {
            total_ms: total_time / iterations as f64,
            dispatches: total_dispatches / iterations,
            bandwidth_gb: self.estimate_bandwidth_gb(chunk_positions.len(), 1), // Single pass!
        }
    }
    
    /// Estimate memory bandwidth usage
    fn estimate_bandwidth_gb(&self, chunk_count: usize, passes: u32) -> f64 {
        let bytes_per_chunk = 32 * 32 * 32 * 4; // CHUNK_SIZE³ * sizeof(u32)
        let total_bytes = chunk_count * bytes_per_chunk * passes as usize;
        total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
    
    /// Print benchmark results
    pub fn print_results(&self, results: &UnifiedBenchmarkResults) {
        println!("\n=== Benchmark Results ===");
        println!();
        println!("Traditional Multi-Pass Approach:");
        println!("  Total time: {:.2} ms", results.traditional_time_ms);
        println!("  - Terrain generation: {:.2} ms", results.terrain_gen_ms);
        println!("  - Lighting: {:.2} ms", results.lighting_ms);
        println!("  - Physics: {:.2} ms", results.physics_ms);
        println!("  - Modifications: {:.2} ms", results.modifications_ms);
        println!("  Dispatches: {}", results.traditional_dispatches);
        println!("  Bandwidth: {:.2} GB", results.traditional_bandwidth_gb);
        println!();
        println!("Unified Kernel Approach:");
        println!("  Total time: {:.2} ms", results.unified_time_ms);
        println!("  Dispatches: {}", results.unified_dispatches);
        println!("  Bandwidth: {:.2} GB", results.unified_bandwidth_gb);
        println!();
        println!("Performance Improvement: {:.1}x", results.speedup_factor);
        
        if results.speedup_factor >= 1000.0 {
            println!("\n🎉 ACHIEVED 1000x PERFORMANCE TARGET! 🎉");
        } else if results.speedup_factor >= 100.0 {
            println!("\n✨ Excellent performance: {}x improvement", results.speedup_factor as u32);
        } else {
            println!("\n📈 Good performance: {:.1}x improvement", results.speedup_factor);
        }
    }
}

#[derive(Debug)]
struct TraditionalResults {
    total_ms: f64,
    terrain_ms: f64,
    lighting_ms: f64,
    physics_ms: f64,
    modifications_ms: f64,
    dispatches: u32,
    bandwidth_gb: f64,
}

#[derive(Debug)]
struct UnifiedResults {
    total_ms: f64,
    dispatches: u32,
    bandwidth_gb: f64,
}