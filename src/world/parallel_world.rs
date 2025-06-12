use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashSet;
use parking_lot::RwLock;
use cgmath::Point3;
use rayon::{ThreadPool, ThreadPoolBuilder};
use crate::{BlockId, VoxelPos, ChunkPos};
use crate::world::WorldInterface;
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
    
    /// Get loaded chunk positions for cleanup purposes
    pub fn get_loaded_chunk_positions(&self) -> Vec<ChunkPos> {
        self.chunk_manager.get_loaded_chunk_positions()
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

// Implement WorldInterface for ParallelWorld
impl WorldInterface for ParallelWorld {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        self.get_block(pos)
    }
    
    fn set_block(&mut self, pos: VoxelPos, block: BlockId) {
        self.set_block(pos, block);
    }
    
    fn update_loaded_chunks(&mut self, player_pos: Point3<f32>) {
        self.update(player_pos);
    }
    
    fn chunk_size(&self) -> u32 {
        self.config.chunk_size
    }
    
    fn is_block_in_bounds(&self, _pos: VoxelPos) -> bool {
        true // Infinite world
    }
    
    fn get_sky_light(&self, pos: VoxelPos) -> u8 {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size());
        let local_pos = pos.to_local_pos(self.chunk_size());
        
        if let Some(chunk_lock) = self.chunk_manager.get_chunk(chunk_pos) {
            let chunk = chunk_lock.read();
            chunk.get_sky_light(local_pos.0, local_pos.1, local_pos.2)
        } else {
            0
        }
    }
    
    fn set_sky_light(&mut self, pos: VoxelPos, level: u8) {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size());
        let local_pos = pos.to_local_pos(self.chunk_size());
        
        if let Some(chunk_lock) = self.chunk_manager.get_chunk(chunk_pos) {
            let mut chunk = chunk_lock.write();
            chunk.set_sky_light(local_pos.0, local_pos.1, local_pos.2, level);
        }
    }
    
    fn get_block_light(&self, pos: VoxelPos) -> u8 {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size());
        let local_pos = pos.to_local_pos(self.chunk_size());
        
        if let Some(chunk_lock) = self.chunk_manager.get_chunk(chunk_pos) {
            let chunk = chunk_lock.read();
            chunk.get_block_light(local_pos.0, local_pos.1, local_pos.2)
        } else {
            0
        }
    }
    
    fn set_block_light(&mut self, pos: VoxelPos, level: u8) {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size());
        let local_pos = pos.to_local_pos(self.chunk_size());
        
        if let Some(chunk_lock) = self.chunk_manager.get_chunk(chunk_pos) {
            let mut chunk = chunk_lock.write();
            chunk.set_block_light(local_pos.0, local_pos.1, local_pos.2, level);
        }
    }
    
    fn is_chunk_loaded(&self, pos: ChunkPos) -> bool {
        self.is_chunk_loaded(pos)
    }
    
    fn take_dirty_chunks(&mut self) -> HashSet<ChunkPos> {
        // ParallelWorld manages dirty chunks internally through the chunk manager
        self.chunk_manager.take_dirty_chunks()
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        // Delegate to the world generator
        self.chunk_manager.get_surface_height(world_x, world_z)
    }
    
    fn is_block_transparent(&self, pos: VoxelPos) -> bool {
        let block_id = self.get_block(pos);
        // For now, only air and water are transparent
        block_id == BlockId::AIR || block_id == BlockId(6) // Water
    }
}