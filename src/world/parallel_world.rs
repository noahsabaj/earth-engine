use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use cgmath::Point3;
use rayon::{ThreadPool, ThreadPoolBuilder};
use crate::{BlockId, VoxelPos, ChunkPos};
use super::{ParallelChunkManager, WorldGenerator, GenerationStats};

/// Configuration for parallel world processing
#[derive(Debug, Clone)]
pub struct ParallelWorldConfig {
    /// Number of threads for chunk generation
    pub generation_threads: usize,
    /// Number of threads for mesh building
    pub mesh_threads: usize,
    /// Maximum chunks to generate per frame
    pub chunks_per_frame: usize,
    /// View distance in chunks
    pub view_distance: i32,
    /// Chunk size
    pub chunk_size: u32,
}

impl Default for ParallelWorldConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            generation_threads: cpu_count.saturating_sub(2).max(2),
            mesh_threads: cpu_count.saturating_sub(2).max(2),
            chunks_per_frame: cpu_count * 2,
            view_distance: 8,
            chunk_size: 32,
        }
    }
}

/// High-performance parallel world implementation
pub struct ParallelWorld {
    /// Parallel chunk manager
    chunk_manager: Arc<ParallelChunkManager>,
    /// Generation thread pool
    generation_pool: ThreadPool,
    /// Configuration
    config: ParallelWorldConfig,
    /// Performance metrics
    last_update_time: Arc<RwLock<Instant>>,
    frame_times: Arc<RwLock<Vec<Duration>>>,
}

impl ParallelWorld {
    pub fn new(generator: Box<dyn WorldGenerator>, config: ParallelWorldConfig) -> Self {
        // Create generation thread pool
        let generation_pool = ThreadPoolBuilder::new()
            .num_threads(config.generation_threads)
            .thread_name(|idx| format!("world-gen-{}", idx))
            .build()
            .expect("Failed to create generation thread pool");
        
        let chunk_manager = Arc::new(ParallelChunkManager::new(
            config.view_distance,
            config.chunk_size,
            generator,
        ));
        
        Self {
            chunk_manager,
            generation_pool,
            config,
            last_update_time: Arc::new(RwLock::new(Instant::now())),
            frame_times: Arc::new(RwLock::new(Vec::with_capacity(120))),
        }
    }
    
    /// Update world state and trigger parallel chunk generation
    pub fn update(&self, player_pos: Point3<f32>) {
        let start_time = Instant::now();
        
        // Update chunk loading state
        self.chunk_manager.update_loaded_chunks(player_pos);
        
        // Process generation queue in parallel
        let manager = Arc::clone(&self.chunk_manager);
        let max_chunks = self.config.chunks_per_frame;
        
        self.generation_pool.spawn(move || {
            // Process up to max_chunks per frame
            for _ in 0..max_chunks {
                manager.process_generation_queue();
            }
        });
        
        // Track frame time
        let frame_time = start_time.elapsed();
        let mut frame_times = self.frame_times.write();
        frame_times.push(frame_time);
        
        // Keep only last 120 frames (2 seconds at 60 FPS)
        if frame_times.len() > 120 {
            frame_times.remove(0);
        }
        
        *self.last_update_time.write() = start_time;
    }
    
    /// Pregenerate spawn area for smooth start
    pub fn pregenerate_spawn_area(&self, spawn_pos: Point3<f32>, radius: i32) {
        let spawn_chunk = ChunkPos::new(
            (spawn_pos.x / self.config.chunk_size as f32).floor() as i32,
            (spawn_pos.y / self.config.chunk_size as f32).floor() as i32,
            (spawn_pos.z / self.config.chunk_size as f32).floor() as i32,
        );
        
        println!("Pregenerating {} chunks around spawn...", 
            (2 * radius + 1).pow(3));
        
        let start_time = Instant::now();
        self.chunk_manager.pregenerate_chunks(spawn_chunk, radius);
        
        // Wait for generation to complete
        while self.chunk_manager.loaded_chunk_count() < (2 * radius + 1).pow(3) as usize {
            self.generation_pool.spawn({
                let manager = Arc::clone(&self.chunk_manager);
                move || {
                    manager.process_generation_queue();
                }
            });
            std::thread::sleep(Duration::from_millis(10));
        }
        
        let elapsed = start_time.elapsed();
        println!("Pregeneration complete in {:.2}s", elapsed.as_secs_f32());
    }
    
    /// Get world performance metrics
    pub fn get_performance_metrics(&self) -> WorldPerformanceMetrics {
        let gen_stats = self.chunk_manager.get_stats();
        let frame_times = self.frame_times.read();
        
        let avg_frame_time = if !frame_times.is_empty() {
            let sum: Duration = frame_times.iter().sum();
            sum / frame_times.len() as u32
        } else {
            Duration::from_millis(16)
        };
        
        WorldPerformanceMetrics {
            loaded_chunks: self.chunk_manager.loaded_chunk_count(),
            cached_chunks: self.chunk_manager.cached_chunk_count(),
            chunks_generated: gen_stats.chunks_generated,
            average_chunk_time: gen_stats.average_chunk_time,
            chunks_per_second: gen_stats.chunks_per_second,
            average_frame_time: avg_frame_time,
            fps: if avg_frame_time.as_secs_f32() > 0.0 {
                1.0 / avg_frame_time.as_secs_f32()
            } else {
                0.0
            },
        }
    }
    
    /// Reset performance statistics
    pub fn reset_stats(&self) {
        self.chunk_manager.reset_stats();
        self.frame_times.write().clear();
    }
    
    /// Get block at position (thread-safe)
    pub fn get_block(&self, pos: VoxelPos) -> BlockId {
        self.chunk_manager.get_block(pos)
    }
    
    /// Set block at position (thread-safe)
    pub fn set_block(&self, pos: VoxelPos, block: BlockId) {
        self.chunk_manager.set_block(pos, block);
    }
    
    /// Check if chunk is loaded
    pub fn is_chunk_loaded(&self, pos: ChunkPos) -> bool {
        self.chunk_manager.is_chunk_loaded(pos)
    }
    
    /// Get chunk manager reference
    pub fn chunk_manager(&self) -> &ParallelChunkManager {
        &self.chunk_manager
    }
    
    /// Get chunk manager Arc for sharing
    pub fn chunk_manager_arc(&self) -> Arc<ParallelChunkManager> {
        Arc::clone(&self.chunk_manager)
    }
    
    /// Get configuration
    pub fn config(&self) -> &ParallelWorldConfig {
        &self.config
    }
}

/// Performance metrics for the parallel world
#[derive(Debug, Clone)]
pub struct WorldPerformanceMetrics {
    pub loaded_chunks: usize,
    pub cached_chunks: usize,
    pub chunks_generated: usize,
    pub average_chunk_time: Duration,
    pub chunks_per_second: f32,
    pub average_frame_time: Duration,
    pub fps: f32,
}