use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashSet;
use parking_lot::RwLock;
use dashmap::DashMap;
use rayon::prelude::*;
use cgmath::Point3;
use crossbeam_channel::{bounded, Sender, Receiver};
use crate::{Chunk, ChunkPos, VoxelPos, BlockId};
use super::generation::WorldGenerator;

/// Statistics for chunk generation performance
#[derive(Debug, Default)]
pub struct GenerationStats {
    pub chunks_generated: usize,
    pub total_generation_time: Duration,
    pub average_chunk_time: Duration,
    pub chunks_per_second: f32,
}

/// Priority-based chunk generation request
#[derive(Debug, Clone)]
struct GenerationRequest {
    chunk_pos: ChunkPos,
    priority: i32, // Lower is higher priority
}

/// Queue metrics for monitoring performance
#[derive(Debug, Clone)]
pub struct QueueMetrics {
    /// Number of chunks waiting to be generated
    pub generation_queue_length: usize,
    /// Number of completed chunks waiting to be processed
    pub completed_queue_length: usize,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Generation queue usage percentage
    pub generation_queue_usage: f32,
    /// Completed queue usage percentage
    pub completed_queue_usage: f32,
    /// Current batch size setting
    pub current_batch_size: usize,
    /// Dynamic batch size based on queue pressure
    pub dynamic_batch_size: usize,
}

/// Enhanced parallel chunk manager with priority generation and metrics
pub struct ParallelChunkManager {
    /// Thread-safe chunk storage
    chunks: Arc<DashMap<ChunkPos, Arc<RwLock<Chunk>>>>,
    /// View distance in chunks
    view_distance: i32,
    /// Size of each chunk
    chunk_size: u32,
    /// World generator
    generator: Arc<dyn WorldGenerator>,
    /// Channel for chunk generation requests
    generation_sender: Sender<GenerationRequest>,
    generation_receiver: Receiver<GenerationRequest>,
    /// Channel for completed chunks
    completed_sender: Sender<(ChunkPos, Chunk, Duration)>,
    completed_receiver: Receiver<(ChunkPos, Chunk, Duration)>,
    /// Generation statistics
    stats: Arc<RwLock<GenerationStats>>,
    /// Maximum chunks to generate per batch
    batch_size: usize,
    /// Chunk cache for unloaded chunks
    chunk_cache: Arc<DashMap<ChunkPos, Arc<RwLock<Chunk>>>>,
    cache_limit: usize,
    /// Counter for total chunks generated
    chunks_generated_counter: Arc<AtomicUsize>,
}

impl ParallelChunkManager {
    pub fn new(view_distance: i32, chunk_size: u32, generator: Box<dyn WorldGenerator>) -> Self {
        // Use bounded channels to prevent memory issues
        // Queue size is based on view distance - allow up to 2x the visible chunks
        let max_queue_size = ((view_distance * 2 + 1).pow(3) * 2).max(1000) as usize;
        let (gen_sender, gen_receiver) = bounded(max_queue_size);
        let (comp_sender, comp_receiver) = bounded(max_queue_size);
        
        Self {
            chunks: Arc::new(DashMap::new()),
            view_distance,
            chunk_size,
            generator: Arc::from(generator),
            generation_sender: gen_sender,
            generation_receiver: gen_receiver,
            completed_sender: comp_sender,
            completed_receiver: comp_receiver,
            stats: Arc::new(RwLock::new(GenerationStats::default())),
            batch_size: num_cpus::get().min(8), // Process at most 8 chunks per batch
            chunk_cache: Arc::new(DashMap::new()),
            cache_limit: 128, // Cache up to 128 chunks
            chunks_generated_counter: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    /// Set the batch size for chunk generation
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size.max(1).min(32); // Clamp between 1 and 32
    }
    
    /// Get current batch size
    pub fn get_batch_size(&self) -> usize {
        self.batch_size
    }
    
    /// Set view distance
    pub fn set_view_distance(&self, view_distance: i32) {
        // For now, we can't change the view distance of an immutable reference
        // This would need to be changed to &mut self or use interior mutability
        log::warn!("set_view_distance called but ParallelChunkManager view distance is immutable");
    }
    
    /// Get the number of chunks waiting in the generation queue
    pub fn get_queue_length(&self) -> usize {
        self.generation_receiver.len()
    }
    
    /// Get the number of chunks waiting in the generation queue
    pub fn get_generation_queue_depth(&self) -> usize {
        self.generation_receiver.len()
    }
    
    /// Get the number of completed chunks waiting to be processed
    pub fn get_completed_queue_depth(&self) -> usize {
        self.completed_receiver.len()
    }
    
    /// Get the maximum queue size
    pub fn get_max_queue_size(&self) -> usize {
        // Return the capacity used when creating the bounded channels
        let view_distance = self.view_distance;
        ((view_distance * 2 + 1).pow(3) * 2).max(1000) as usize
    }
    
    /// Get the queue fullness as a percentage (0-100)
    pub fn get_queue_fullness_percent(&self) -> f32 {
        let queue_length = self.generation_receiver.len();
        let max_queue_size = self.generation_receiver.capacity().unwrap_or(1000);
        (queue_length as f32 / max_queue_size as f32) * 100.0
    }
    
    /// Calculate dynamic batch size based on queue pressure
    fn calculate_dynamic_batch_size(&self) -> usize {
        let queue_depth = self.get_queue_length();
        let completed_depth = self.get_completed_queue_depth();
        let base_batch = self.batch_size;
        
        // Adjust batch size based on queue pressure
        if queue_depth > base_batch * 4 {
            // Queue is backing up, generate more chunks
            (base_batch * 2).min(32)
        } else if completed_depth > base_batch * 2 {
            // Completed queue is backing up, slow down generation
            (base_batch / 2).max(1)
        } else {
            // Normal operation
            base_batch
        }
    }
    
    /// Update loaded chunks with priority-based generation
    pub fn update_loaded_chunks(&self, player_pos: Point3<f32>) {
        let player_chunk = ChunkPos::new(
            (player_pos.x / self.chunk_size as f32).floor() as i32,
            (player_pos.y / self.chunk_size as f32).floor() as i32,
            (player_pos.z / self.chunk_size as f32).floor() as i32,
        );
        
        // Log the first few updates
        static UPDATE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = UPDATE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count < 5 {
            log::info!("[ParallelChunkManager::update_loaded_chunks] Update #{} - player chunk: {:?}, loaded: {}", 
                     count + 1, player_chunk, self.chunks.len());
        }
        
        // First, unload distant chunks
        self.unload_distant_chunks(player_chunk);
        
        // Log queue metrics periodically
        if count % 30 == 0 && count > 0 {
            let gen_depth = self.get_generation_queue_depth();
            let comp_depth = self.get_completed_queue_depth();
            let max_size = self.get_max_queue_size();
            log::info!("[ParallelChunkManager] Queue metrics - Gen: {} ({:.1}%), Comp: {} ({:.1}%)",
                     gen_depth, (gen_depth as f32 / max_size as f32) * 100.0,
                     comp_depth, (comp_depth as f32 / max_size as f32) * 100.0);
        }
        
        // Collect chunks that need to be loaded with priority
        let mut generation_requests = Vec::new();
        
        for dx in -self.view_distance..=self.view_distance {
            for dy in -self.view_distance..=self.view_distance {
                for dz in -self.view_distance..=self.view_distance {
                    let distance_sq = dx * dx + dy * dy + dz * dz;
                    if distance_sq <= self.view_distance * self.view_distance {
                        let chunk_pos = ChunkPos::new(
                            player_chunk.x + dx,
                            player_chunk.y + dy,
                            player_chunk.z + dz,
                        );
                        
                        // Check if chunk needs loading
                        if !self.chunks.contains_key(&chunk_pos) {
                            // Check cache first
                            if let Some((_, cached)) = self.chunk_cache.remove(&chunk_pos) {
                                self.chunks.insert(chunk_pos, cached);
                            } else {
                                // Add to generation queue with priority
                                generation_requests.push(GenerationRequest {
                                    chunk_pos,
                                    priority: distance_sq, // Closer chunks have lower priority value
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by priority (closer chunks first)
        generation_requests.sort_by_key(|req| req.priority);
        
        if count < 5 && !generation_requests.is_empty() {
            log::info!("[ParallelChunkManager::update_loaded_chunks] {} chunks need generation", generation_requests.len());
        }
        
        // Only queue a limited number of requests to avoid overwhelming the system
        let max_new_requests = self.batch_size * 2; // Allow queuing up to 2x batch size
        let mut queued_count = 0;
        let mut skipped_count = 0;
        
        for request in generation_requests.into_iter().take(max_new_requests) {
            // Use try_send to avoid blocking if channel is full
            match self.generation_sender.try_send(request) {
                Ok(_) => { queued_count += 1; },
                Err(e) => {
                    skipped_count += 1;
                    let queue_fullness = self.get_queue_fullness_percent();
                    if queue_fullness > 90.0 {
                        log::error!("[ParallelChunkManager] CRITICAL: Generation queue {:.1}% full! Skipping chunk at {:?}. Consider increasing queue size or processing rate.", 
                                  queue_fullness, e.into_inner().chunk_pos);
                    } else if queue_fullness > 70.0 {
                        log::warn!("[ParallelChunkManager] Generation queue {:.1}% full, skipping chunk at {:?}", 
                                 queue_fullness, e.into_inner().chunk_pos);
                    } else {
                        log::debug!("[ParallelChunkManager] Generation queue full, skipping chunk: {:?}", e);
                    }
                    break;
                }
            }
        }
        
        if skipped_count > 0 {
            log::warn!("[ParallelChunkManager] Skipped {} chunks due to full queue (queued {} successfully)", 
                     skipped_count, queued_count);
        }
        
        if count < 5 && queued_count > 0 {
            log::info!("[ParallelChunkManager::update_loaded_chunks] Queued {} chunks for generation", queued_count);
        }
        
        // Process completed chunks more aggressively to match generation rate
        // Calculate how many to consume based on queue depth
        let completed_queue_depth = self.get_completed_queue_depth();
        let max_completions = if completed_queue_depth > self.batch_size * 4 {
            // Queue is backing up, consume more aggressively
            (self.batch_size * 3).min(completed_queue_depth)
        } else {
            self.batch_size
        };
        
        let mut processed = 0;
        
        while processed < max_completions {
            match self.completed_receiver.try_recv() {
                Ok((pos, chunk, gen_time)) => {
                    self.chunks.insert(pos, Arc::new(RwLock::new(chunk)));
                    
                    // Update statistics
                    let mut stats = self.stats.write();
                    stats.chunks_generated += 1;
                    stats.total_generation_time += gen_time;
                    stats.average_chunk_time = stats.total_generation_time / stats.chunks_generated as u32;
                    let total_secs = stats.total_generation_time.as_secs_f32();
                    if total_secs > 0.0 {
                        stats.chunks_per_second = stats.chunks_generated as f32 / total_secs;
                    }
                    
                    processed += 1;
                }
                Err(_) => break, // No more chunks ready
            }
        }
        
        if count < 5 && processed > 0 {
            log::info!("[ParallelChunkManager::update_loaded_chunks] Processed {} completed chunks, total loaded: {}", 
                     processed, self.chunks.len());
        }
    }
    
    /// Process chunk generation queue in parallel with batching
    pub fn process_generation_queue(&self) {
        // Check if we should process based on completed queue pressure
        let completed_queue_depth = self.get_completed_queue_depth();
        let max_queue_size = self.get_max_queue_size();
        
        // Don't generate if completed queue is too full
        if completed_queue_depth >= max_queue_size * 3 / 4 {
            static STALL_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let stall_count = STALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if stall_count % 100 == 0 {
                log::warn!("[ParallelChunkManager] Generation stalled - completed queue at {}/{} capacity", 
                         completed_queue_depth, max_queue_size);
            }
            return;
        }
        
        let mut pending_requests = Vec::new();
        
        // Calculate dynamic batch size based on queue pressure
        let dynamic_batch_size = self.calculate_dynamic_batch_size();
        
        // Collect up to dynamic_batch_size requests
        for _ in 0..dynamic_batch_size {
            if let Ok(request) = self.generation_receiver.try_recv() {
                pending_requests.push(request);
            } else {
                break;
            }
        }
        
        if pending_requests.is_empty() {
            return;
        }
        
        // Log the first few generation batches or emergency situations
        static GEN_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = GEN_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let is_emergency = dynamic_batch_size > self.batch_size * 2;
        if count < 5 || is_emergency {
            log::info!("[ParallelChunkManager::process_generation_queue] Processing {} chunks (batch #{}, dynamic_batch: {}){}",
                     pending_requests.len(), count + 1, dynamic_batch_size,
                     if is_emergency { " [EMERGENCY DRAIN]" } else { "" });
        }
        
        // Sort by priority again in case new requests came in
        pending_requests.sort_by_key(|req| req.priority);
        
        // Generate chunks in parallel
        let chunk_size = self.chunk_size;
        let generator = Arc::clone(&self.generator);
        let sender = self.completed_sender.clone();
        
        pending_requests.par_iter().for_each(|request| {
            let start_time = Instant::now();
            
            // Generate chunk
            let mut chunk = generator.generate_chunk(request.chunk_pos, chunk_size);
            
            // IMPORTANT: Mark chunk as dirty after generation so it gets meshed
            chunk.mark_dirty();
            
            // Log chunk generation details
            let non_air_blocks = chunk.blocks().iter().filter(|&&b| b != crate::BlockId::AIR).count();
            if count < 5 {
                log::info!("[ParallelChunkManager] Generated chunk at {:?} with {} non-air blocks, dirty: {}", 
                         request.chunk_pos, non_air_blocks, chunk.is_dirty());
            }
            
            let generation_time = start_time.elapsed();
            // Use send instead of try_send to ensure no chunks are dropped
            // The bounded channel will block if full, providing natural backpressure
            match sender.send((request.chunk_pos, chunk, generation_time)) {
                Ok(_) => {},
                Err(e) => {
                    log::error!("[ParallelChunkManager] Failed to send completed chunk: {:?}", e);
                }
            }
        });
        
        // Log when emergency drain completes
        if is_emergency {
            log::info!("[ParallelChunkManager] Emergency drain completed. Processed {} chunks.", pending_requests.len());
        }
    }
    
    /// Unload chunks that are too far from the player
    fn unload_distant_chunks(&self, player_chunk: ChunkPos) {
        let unload_distance = self.view_distance + 2; // Keep a small buffer
        let unload_distance_sq = unload_distance * unload_distance;
        
        // Find chunks to unload
        let chunks_to_unload: Vec<ChunkPos> = self.chunks
            .iter()
            .filter_map(|entry| {
                let chunk_pos = *entry.key();
                let distance_sq = chunk_pos.distance_squared_to(player_chunk);
                if distance_sq > unload_distance_sq {
                    Some(chunk_pos)
                } else {
                    None
                }
            })
            .collect();
        
        // Move chunks to cache
        for chunk_pos in chunks_to_unload {
            if let Some((_, chunk)) = self.chunks.remove(&chunk_pos) {
                self.chunk_cache.insert(chunk_pos, chunk);
                
                // Evict old cached chunks if over limit
                if self.chunk_cache.len() > self.cache_limit {
                    // Remove the furthest chunk from player
                    if let Some(furthest) = self.chunk_cache
                        .iter()
                        .map(|entry| (*entry.key(), entry.key().distance_squared_to(player_chunk)))
                        .max_by_key(|(_, dist)| *dist)
                        .map(|(pos, _)| pos)
                    {
                        self.chunk_cache.remove(&furthest);
                    }
                }
            }
        }
    }
    
    /// Get generation statistics
    pub fn get_stats(&self) -> GenerationStats {
        self.stats.read().clone()
    }
    
    /// Clear generation statistics
    pub fn reset_stats(&self) {
        *self.stats.write() = GenerationStats::default();
    }
    
    /// Log comprehensive queue statistics
    pub fn log_queue_stats(&self) {
        let metrics = self.get_queue_metrics();
        let stats = self.get_stats();
        
        log::info!("[ParallelChunkManager] === Queue Statistics ===");
        log::info!("  Generation Queue: {} / {} ({:.1}% full)", 
                 metrics.generation_queue_length, metrics.max_queue_size, metrics.generation_queue_usage);
        log::info!("  Completed Queue: {} / {} ({:.1}% full)", 
                 metrics.completed_queue_length, metrics.max_queue_size, metrics.completed_queue_usage);
        log::info!("  Batch Size: {} (dynamic: {})", metrics.current_batch_size, metrics.dynamic_batch_size);
        log::info!("  Generation Performance: {:.2} chunks/s (avg {:.2}ms per chunk)",
                 stats.chunks_per_second, stats.average_chunk_time.as_millis());
        log::info!("  Total Generated: {} chunks", stats.chunks_generated);
        log::info!("========================");
    }
    
    /// Get immutable chunk reference (thread-safe)
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<Arc<RwLock<Chunk>>> {
        self.chunks.get(&pos).map(|entry| Arc::clone(&entry))
    }
    
    /// Get block at position (thread-safe)
    pub fn get_block(&self, pos: VoxelPos) -> BlockId {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk_lock) = self.get_chunk(chunk_pos) {
            let chunk = chunk_lock.read();
            chunk.get_block(local_pos.0, local_pos.1, local_pos.2)
        } else {
            BlockId::AIR
        }
    }
    
    /// Set block at position (thread-safe)
    pub fn set_block(&self, pos: VoxelPos, block: BlockId) {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk_lock) = self.get_chunk(chunk_pos) {
            let mut chunk = chunk_lock.write();
            chunk.set_block(local_pos.0, local_pos.1, local_pos.2, block);
            chunk.mark_dirty();
        }
    }
    
    /// Get chunks for iteration
    pub fn chunks_iter(&self) -> impl Iterator<Item = (ChunkPos, Arc<RwLock<Chunk>>)> {
        self.chunks
            .iter()
            .map(|entry| (*entry.key(), Arc::clone(entry.value())))
            .collect::<Vec<_>>()
            .into_iter()
    }
    
    /// Check if chunk is loaded
    pub fn is_chunk_loaded(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }
    
    /// Get number of loaded chunks
    pub fn loaded_chunk_count(&self) -> usize {
        self.chunks.len()
    }
    
    /// Get number of cached chunks
    pub fn cached_chunk_count(&self) -> usize {
        self.chunk_cache.len()
    }
    
    /// Get loaded chunk positions
    pub fn get_loaded_chunk_positions(&self) -> Vec<ChunkPos> {
        self.chunks
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }
    
    /// Take dirty chunks that need remeshing
    pub fn take_dirty_chunks(&self) -> HashSet<ChunkPos> {
        let mut dirty_chunks = HashSet::new();
        
        for entry in self.chunks.iter() {
            let chunk_pos = *entry.key();
            let chunk_lock = entry.value();
            let mut chunk = chunk_lock.write();
            
            if chunk.is_dirty() {
                chunk.clear_dirty();
                dirty_chunks.insert(chunk_pos);
            }
        }
        
        dirty_chunks
    }
    
    /// Get surface height from the world generator
    pub fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        self.generator.get_surface_height(world_x, world_z)
    }
    
    /// Get access to GPU WorldBuffer if using GPU-based generator
    /// Returns None if using CPU-based generator
    pub fn get_world_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::world_gpu::WorldBuffer>>> {
        self.generator.get_world_buffer()
    }
    
    /// Get current queue metrics
    pub fn get_queue_metrics(&self) -> QueueMetrics {
        let generation_queue_length = self.get_generation_queue_depth();
        let completed_queue_length = self.get_completed_queue_depth();
        let max_queue_size = self.get_max_queue_size();
        
        QueueMetrics {
            generation_queue_length,
            completed_queue_length,
            max_queue_size,
            generation_queue_usage: (generation_queue_length as f32 / max_queue_size as f32) * 100.0,
            completed_queue_usage: (completed_queue_length as f32 / max_queue_size as f32) * 100.0,
            current_batch_size: self.batch_size,
            dynamic_batch_size: self.calculate_dynamic_batch_size(),
        }
    }
    
    /// Force generate specific chunks (useful for spawn area)
    pub fn pregenerate_chunks(&self, center: ChunkPos, radius: i32) {
        let requests: Vec<GenerationRequest> = (-radius..=radius)
            .flat_map(|dx| {
                (-radius..=radius).flat_map(move |dy| {
                    (-radius..=radius).filter_map(move |dz| {
                        let distance_sq = dx * dx + dy * dy + dz * dz;
                        if distance_sq <= radius * radius {
                            let chunk_pos = ChunkPos::new(
                                center.x + dx,
                                center.y + dy,
                                center.z + dz,
                            );
                            
                            if !self.is_chunk_loaded(chunk_pos) {
                                Some(GenerationRequest {
                                    chunk_pos,
                                    priority: distance_sq,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                })
            })
            .collect();
        
        // Generate all chunks directly in parallel for faster startup
        let chunk_size = self.chunk_size;
        let generator = Arc::clone(&self.generator);
        let sender = self.completed_sender.clone();
        let gen_counter = Arc::clone(&self.chunks_generated_counter);
        
        requests.par_iter().for_each(|request| {
            let start_time = Instant::now();
            let chunk = generator.generate_chunk(request.chunk_pos, chunk_size);
            let generation_time = start_time.elapsed();
            // Use send to ensure chunks aren't dropped during pregeneration
            match sender.send((request.chunk_pos, chunk, generation_time)) {
                Ok(_) => {
                    // Increment generation counter
                    gen_counter.fetch_add(1, Ordering::Relaxed);
                },
                Err(e) => {
                    log::error!("[ParallelChunkManager] Failed to send pregenerated chunk: {:?}", e);
                }
            }
        });
    }
    
    /// Queue a single chunk for generation with explicit priority
    pub fn queue_chunk_generation(&self, chunk_pos: ChunkPos, priority: i32) {
        if !self.is_chunk_loaded(chunk_pos) {
            let request = GenerationRequest {
                chunk_pos,
                priority,
            };
            // Try to send, but don't block if channel is full
            let _ = self.generation_sender.try_send(request);
        }
    }
    
    /// Pregenerate a batch of chunks
    pub fn pregenerate_chunks_batch(&self, chunks: &[ChunkPos]) {
        let chunk_size = self.chunk_size;
        let generator = Arc::clone(&self.generator);
        let sender = self.completed_sender.clone();
        let gen_counter = Arc::clone(&self.chunks_generated_counter);
        
        // Generate chunks in parallel
        chunks.par_iter().for_each(|&chunk_pos| {
            if !self.is_chunk_loaded(chunk_pos) {
                let start_time = Instant::now();
                let chunk = generator.generate_chunk(chunk_pos, chunk_size);
                let generation_time = start_time.elapsed();
                // Use send to ensure chunks aren't dropped
                match sender.send((chunk_pos, chunk, generation_time)) {
                    Ok(_) => {
                        gen_counter.fetch_add(1, Ordering::Relaxed);
                    },
                    Err(e) => {
                        log::error!("[ParallelChunkManager] Failed to send batch chunk: {:?}", e);
                    }
                }
            }
        });
    }
    
}

impl Clone for GenerationStats {
    fn clone(&self) -> Self {
        Self {
            chunks_generated: self.chunks_generated,
            total_generation_time: self.total_generation_time,
            average_chunk_time: self.average_chunk_time,
            chunks_per_second: self.chunks_per_second,
        }
    }
}

