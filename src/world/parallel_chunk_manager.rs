use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashSet;
use parking_lot::RwLock;
use dashmap::DashMap;
use rayon::prelude::*;
use cgmath::Point3;
use crossbeam_channel::{unbounded, Sender, Receiver};
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
}

impl ParallelChunkManager {
    pub fn new(view_distance: i32, chunk_size: u32, generator: Box<dyn WorldGenerator>) -> Self {
        let (gen_sender, gen_receiver) = unbounded();
        let (comp_sender, comp_receiver) = unbounded();
        
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
    
    /// Get the number of chunks waiting in the generation queue
    pub fn get_queue_length(&self) -> usize {
        self.generation_receiver.len()
    }
    
    /// Update loaded chunks with priority-based generation
    pub fn update_loaded_chunks(&self, player_pos: Point3<f32>) {
        let player_chunk = ChunkPos::new(
            (player_pos.x / self.chunk_size as f32).floor() as i32,
            (player_pos.y / self.chunk_size as f32).floor() as i32,
            (player_pos.z / self.chunk_size as f32).floor() as i32,
        );
        
        // First, unload distant chunks
        self.unload_distant_chunks(player_chunk);
        
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
        
        // Only queue a limited number of requests to avoid overwhelming the system
        let max_new_requests = self.batch_size * 2; // Allow queuing up to 2x batch size
        for request in generation_requests.into_iter().take(max_new_requests) {
            let _ = self.generation_sender.send(request);
        }
        
        // Process completed chunks with a limit to avoid frame stalls
        let max_completions = self.batch_size; // Process at most batch_size completions per frame
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
    }
    
    /// Process chunk generation queue in parallel with batching
    pub fn process_generation_queue(&self) {
        let mut pending_requests = Vec::new();
        
        // Collect up to batch_size requests
        for _ in 0..self.batch_size {
            if let Ok(request) = self.generation_receiver.try_recv() {
                pending_requests.push(request);
            } else {
                break;
            }
        }
        
        if pending_requests.is_empty() {
            return;
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
            let chunk = generator.generate_chunk(request.chunk_pos, chunk_size);
            
            let generation_time = start_time.elapsed();
            let _ = sender.send((request.chunk_pos, chunk, generation_time));
        });
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
        
        // Send all requests
        for request in requests {
            let _ = self.generation_sender.send(request);
        }
        
        // Process immediately
        self.process_generation_queue();
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