use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashSet;
use parking_lot::RwLock;
use cgmath::Point3;
use crate::{BlockId, VoxelPos, ChunkPos, Chunk};
use crate::world::WorldInterface;
use crate::thread_pool::{ThreadPoolManager, PoolCategory};
use super::{ParallelChunkManager, WorldGenerator};

/// Handle for tracking spawn area generation progress
#[derive(Clone)]
pub struct SpawnGenerationHandle {
    pub total_chunks: usize,
    pub chunks_generated: Arc<AtomicUsize>,
    pub is_complete: Arc<AtomicBool>,
    pub start_time: Instant,
}

impl SpawnGenerationHandle {
    /// Check if generation is complete
    pub fn is_complete(&self) -> bool {
        self.is_complete.load(Ordering::Relaxed)
    }
    
    /// Get number of chunks generated so far
    pub fn chunks_generated(&self) -> usize {
        self.chunks_generated.load(Ordering::Relaxed)
    }
    
    /// Get progress as a percentage
    pub fn progress_percent(&self) -> f32 {
        if self.total_chunks == 0 {
            100.0
        } else {
            (self.chunks_generated() as f32 / self.total_chunks as f32) * 100.0
        }
    }
    
    /// Get elapsed time since generation started
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

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

impl ParallelWorldConfig {
    /// Create config from engine config to ensure consistency
    pub fn from_engine_config(engine_config: &crate::EngineConfig) -> Self {
        let cpu_count = num_cpus::get();
        Self {
            generation_threads: cpu_count.saturating_sub(2).max(2),
            mesh_threads: cpu_count.saturating_sub(2).max(2),
            chunks_per_frame: cpu_count * 2,
            view_distance: engine_config.render_distance as i32,
            chunk_size: engine_config.chunk_size,
        }
    }
}

/// High-performance parallel world implementation
pub struct ParallelWorld {
    /// Parallel chunk manager
    chunk_manager: Arc<ParallelChunkManager>,
    /// Configuration
    config: ParallelWorldConfig,
    /// Performance metrics
    last_update_time: Arc<RwLock<Instant>>,
    frame_times: Arc<RwLock<Vec<Duration>>>,
}

impl ParallelWorld {
    pub fn new(generator: Box<dyn WorldGenerator>, config: ParallelWorldConfig) -> Self {
        let chunk_manager = Arc::new(ParallelChunkManager::new(
            config.view_distance,
            config.chunk_size,
            generator,
        ));
        
        Self {
            chunk_manager,
            config,
            last_update_time: Arc::new(RwLock::new(Instant::now())),
            frame_times: Arc::new(RwLock::new(Vec::with_capacity(120))),
        }
    }
    
    /// Update world state and trigger parallel chunk generation
    pub fn update(&self, player_pos: Point3<f32>) {
        let start_time = Instant::now();
        
        // Log the first few updates for debugging
        static UPDATE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = UPDATE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count < 5 {
            log::info!("[ParallelWorld::update] Update #{} at player pos: {:?}", count + 1, player_pos);
        }
        
        // Update chunk loading state
        self.chunk_manager.update_loaded_chunks(player_pos);
        
        // Check queue depths for backpressure
        let completed_queue_depth = self.chunk_manager.get_completed_queue_depth();
        let generation_queue_depth = self.chunk_manager.get_generation_queue_depth();
        
        // Calculate dynamic generation rate based on queue pressure
        let queue_pressure = completed_queue_depth as f32 / self.chunk_manager.get_max_queue_size() as f32;
        let chunks_to_process = if queue_pressure > 0.8 {
            // High pressure - slow down generation
            1
        } else if queue_pressure > 0.5 {
            // Medium pressure - reduce generation rate
            self.config.chunks_per_frame / 2
        } else {
            // Low pressure - normal generation rate
            self.config.chunks_per_frame
        };
        
        // Only spawn generation if there's capacity
        if completed_queue_depth < self.chunk_manager.get_max_queue_size() * 3 / 4 {
            let manager = Arc::clone(&self.chunk_manager);
            
            ThreadPoolManager::global().spawn(PoolCategory::WorldGeneration, move || {
                // Process only the calculated number of chunks based on queue pressure
                for _ in 0..chunks_to_process {
                    // Check queue depth before each generation
                    if manager.get_completed_queue_depth() >= manager.get_max_queue_size() * 3 / 4 {
                        // Stop generating if queue is getting full
                        break;
                    }
                    manager.process_generation_queue();
                }
            });
        } else if count % 60 == 0 {
            // Log warning every 60 frames if generation is stalled
            log::warn!("[ParallelWorld::update] Generation stalled - completed queue at {}/{} capacity", 
                      completed_queue_depth, self.chunk_manager.get_max_queue_size());
        }
        
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
    /// This function is non-blocking and uses progressive generation to avoid freezes
    pub fn pregenerate_spawn_area(&self, spawn_pos: Point3<f32>, radius: i32) -> Result<SpawnGenerationHandle, String> {
        log::info!("[ParallelWorld] Starting spawn area pregeneration at {:?}, radius {}", spawn_pos, radius);
        
        // Limit radius to prevent excessive memory usage
        let effective_radius = radius.min(4); // Max 4 chunks in each direction
        if radius > effective_radius {
            log::warn!("[ParallelWorld] Spawn radius reduced from {} to {} to prevent memory issues", radius, effective_radius);
        }
        
        let spawn_chunk = ChunkPos::new(
            (spawn_pos.x / self.config.chunk_size as f32).floor() as i32,
            (spawn_pos.y / self.config.chunk_size as f32).floor() as i32,
            (spawn_pos.z / self.config.chunk_size as f32).floor() as i32,
        );
        
        // Calculate actual number of chunks in a sphere (more accurate)
        let mut chunk_positions = Vec::new();
        for dx in -effective_radius..=effective_radius {
            for dy in -effective_radius..=effective_radius {
                for dz in -effective_radius..=effective_radius {
                    let distance_sq = dx * dx + dy * dy + dz * dz;
                    // Use floating point for more accurate sphere calculation
                    let radius_sq = effective_radius as f32 * effective_radius as f32;
                    if (distance_sq as f32) <= radius_sq {
                        chunk_positions.push(ChunkPos::new(
                            spawn_chunk.x + dx,
                            spawn_chunk.y + dy,
                            spawn_chunk.z + dz,
                        ));
                    }
                }
            }
        }
        
        let expected_chunks = chunk_positions.len();
        log::info!("[ParallelWorld] Pregenerating {} chunks around spawn (radius: {})", expected_chunks, effective_radius);
        
        let start_time = Instant::now();
        let initial_count = self.chunk_manager.loaded_chunk_count();
        
        // Create handle for tracking progress
        let handle = SpawnGenerationHandle {
            total_chunks: expected_chunks,
            chunks_generated: Arc::new(AtomicUsize::new(0)),
            is_complete: Arc::new(AtomicBool::new(false)),
            start_time,
        };
        
        // Start non-blocking generation
        let manager = Arc::clone(&self.chunk_manager);
        let chunks_generated = Arc::clone(&handle.chunks_generated);
        let is_complete = Arc::clone(&handle.is_complete);
        
        // Use the thread pool to process chunks in batches
        ThreadPoolManager::global().spawn(PoolCategory::WorldGeneration, move || {
            log::debug!("[ParallelWorld] Starting progressive spawn chunk generation");
            
            // Process chunks in priority order (closest to spawn first)
            let mut sorted_positions = chunk_positions;
            sorted_positions.sort_by_key(|pos| {
                let dx = pos.x - spawn_chunk.x;
                let dy = pos.y - spawn_chunk.y;
                let dz = pos.z - spawn_chunk.z;
                dx * dx + dy * dy + dz * dz
            });
            
            // Generate chunks in small batches to avoid overwhelming the system
            const BATCH_SIZE: usize = 8;
            for batch in sorted_positions.chunks(BATCH_SIZE) {
                // Check if any chunks in this batch need generation
                let chunks_to_generate: Vec<_> = batch
                    .iter()
                    .filter(|pos| !manager.is_chunk_loaded(**pos))
                    .cloned()
                    .collect();
                
                if !chunks_to_generate.is_empty() {
                    // Queue generation requests with priority
                    for (idx, chunk_pos) in chunks_to_generate.iter().enumerate() {
                        manager.queue_chunk_generation(*chunk_pos, idx as i32);
                    }
                    
                    // Process the generation queue
                    for _ in 0..chunks_to_generate.len() {
                        manager.process_generation_queue();
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
                
                // Update progress
                let current_generated = manager.loaded_chunk_count().saturating_sub(initial_count);
                chunks_generated.store(current_generated, Ordering::Relaxed);
                
                // Log progress
                if current_generated % 10 == 0 || current_generated == expected_chunks {
                    log::debug!("[ParallelWorld] Spawn generation progress: {}/{} chunks", 
                              current_generated, expected_chunks);
                }
                
                // Yield to other threads
                std::thread::yield_now();
            }
            
            // Mark as complete
            is_complete.store(true, Ordering::Relaxed);
            let final_count = manager.loaded_chunk_count().saturating_sub(initial_count);
            let elapsed = start_time.elapsed();
            
            log::info!("[ParallelWorld] Spawn pregeneration complete: {} chunks in {:.2}s", 
                      final_count, elapsed.as_secs_f32());
        });
        
        Ok(handle)
    }
    
    /// Pregenerate spawn area synchronously (blocking)
    /// Use this only during initial world setup
    pub fn pregenerate_spawn_area_blocking(&self, spawn_pos: Point3<f32>, radius: i32) -> Result<(), String> {
        let handle = self.pregenerate_spawn_area(spawn_pos, radius)?;
        
        // Wait for completion with timeout
        let timeout = Duration::from_secs(10); // Reduced timeout for smaller radius
        let start = Instant::now();
        let mut last_log = Instant::now();
        
        while !handle.is_complete() {
            if start.elapsed() > timeout {
                return Err(format!("Spawn generation timed out after {} seconds", timeout.as_secs()));
            }
            
            // Help process the queue
            self.chunk_manager.process_generation_queue();
            
            // Log progress periodically
            if last_log.elapsed() > Duration::from_secs(1) {
                log::info!("[ParallelWorld] Spawn generation progress: {:.1}% ({}/{} chunks)", 
                          handle.progress_percent(), 
                          handle.chunks_generated(), 
                          handle.total_chunks);
                last_log = Instant::now();
            }
            
            std::thread::sleep(Duration::from_millis(10));
        }
        
        log::info!("[ParallelWorld] Spawn generation completed in {:.2}s", handle.elapsed().as_secs_f32());
        Ok(())
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
    
    /// Get access to GPU WorldBuffer if using GPU-based generator
    /// Returns None if using CPU-based generator
    pub fn get_world_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::world_gpu::WorldBuffer>>> {
        self.chunk_manager.get_world_buffer()
    }
    
    /// Get loaded chunk positions for cleanup purposes
    pub fn get_loaded_chunk_positions(&self) -> Vec<ChunkPos> {
        self.chunk_manager.get_loaded_chunk_positions()
    }
    
    /// Iterator over loaded chunks for mesh generation
    /// Returns chunk position and thread-safe chunk reference
    pub fn iter_loaded_chunks(&self) -> impl Iterator<Item = (ChunkPos, Arc<RwLock<Chunk>>)> {
        self.chunk_manager.chunks_iter()
    }
    
    /// Get chunk data for mesh building (thread-safe)
    /// Returns None if chunk is not loaded
    pub fn get_chunk_for_meshing(&self, pos: ChunkPos) -> Option<Arc<RwLock<Chunk>>> {
        self.chunk_manager.get_chunk(pos)
    }
    
    /// Get all neighbor chunks for a given chunk position
    /// Used for mesh generation to handle block faces at chunk boundaries
    pub fn get_neighbor_chunks(&self, center: ChunkPos) -> [Option<Arc<RwLock<Chunk>>>; 6] {
        [
            self.chunk_manager.get_chunk(ChunkPos::new(center.x - 1, center.y, center.z)), // Left
            self.chunk_manager.get_chunk(ChunkPos::new(center.x + 1, center.y, center.z)), // Right
            self.chunk_manager.get_chunk(ChunkPos::new(center.x, center.y - 1, center.z)), // Bottom
            self.chunk_manager.get_chunk(ChunkPos::new(center.x, center.y + 1, center.z)), // Top
            self.chunk_manager.get_chunk(ChunkPos::new(center.x, center.y, center.z - 1)), // Back
            self.chunk_manager.get_chunk(ChunkPos::new(center.x, center.y, center.z + 1)), // Front
        ]
    }
    
    /// Check if a chunk and all its neighbors are loaded
    /// Used to determine if a chunk is ready for mesh generation
    pub fn is_chunk_ready_for_meshing(&self, pos: ChunkPos) -> bool {
        // Check if the chunk itself is loaded
        if !self.chunk_manager.is_chunk_loaded(pos) {
            return false;
        }
        
        // Check all 6 neighbors
        let neighbors = [
            ChunkPos::new(pos.x - 1, pos.y, pos.z),
            ChunkPos::new(pos.x + 1, pos.y, pos.z),
            ChunkPos::new(pos.x, pos.y - 1, pos.z),
            ChunkPos::new(pos.x, pos.y + 1, pos.z),
            ChunkPos::new(pos.x, pos.y, pos.z - 1),
            ChunkPos::new(pos.x, pos.y, pos.z + 1),
        ];
        
        // All neighbors must be loaded for proper face culling
        neighbors.iter().all(|&neighbor_pos| self.chunk_manager.is_chunk_loaded(neighbor_pos))
    }
    
    /// Get multiple chunks at once for batch processing
    /// Returns a vector of (position, chunk) pairs for chunks that are loaded
    pub fn get_chunks_batch(&self, positions: &[ChunkPos]) -> Vec<(ChunkPos, Arc<RwLock<Chunk>>)> {
        positions.iter()
            .filter_map(|&pos| {
                self.chunk_manager.get_chunk(pos)
                    .map(|chunk| (pos, chunk))
            })
            .collect()
    }
    
    /// Get all chunks that need meshing (dirty and have all neighbors loaded)
    /// This is optimized for the mesh generation pipeline
    pub fn get_chunks_needing_mesh(&self) -> Vec<(ChunkPos, Arc<RwLock<Chunk>>)> {
        let dirty_chunks = self.chunk_manager.take_dirty_chunks();
        
        dirty_chunks.into_iter()
            .filter(|&pos| self.is_chunk_ready_for_meshing(pos))
            .filter_map(|pos| {
                self.chunk_manager.get_chunk(pos)
                    .map(|chunk| (pos, chunk))
            })
            .collect()
    }
    
    /// Get block at position from a specific chunk (for mesh building)
    /// This is more efficient than going through world coordinates when you already have the chunk
    pub fn get_block_in_chunk(&self, chunk_pos: ChunkPos, local_x: u32, local_y: u32, local_z: u32) -> BlockId {
        if let Some(chunk_lock) = self.chunk_manager.get_chunk(chunk_pos) {
            let chunk = chunk_lock.read();
            chunk.get_block(local_x, local_y, local_z)
        } else {
            BlockId::AIR
        }
    }
    
    /// Check if a block face is visible (for mesh optimization)
    /// A face is visible if the adjacent block is transparent
    pub fn is_face_visible(&self, pos: VoxelPos, face_offset: (i32, i32, i32)) -> bool {
        let adjacent_pos = VoxelPos::new(
            pos.x + face_offset.0,
            pos.y + face_offset.1,
            pos.z + face_offset.2,
        );
        
        let adjacent_block = self.get_block(adjacent_pos);
        // A face is visible if the adjacent block is air or transparent
        adjacent_block == BlockId::AIR || self.is_block_transparent(adjacent_pos)
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
        self.chunk_manager.get_block(pos)
    }
    
    fn set_block(&mut self, pos: VoxelPos, block: BlockId) {
        self.chunk_manager.set_block(pos, block);
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
    
    fn ensure_camera_chunk_loaded(&mut self, _camera_pos: Point3<f32>) -> bool {
        // ParallelWorld uses priority-based chunk generation which handles
        // camera position prioritization in its own system
        true
    }
}