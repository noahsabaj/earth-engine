use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;
use cgmath::Point3;
use crate::{Chunk, ChunkPos, VoxelPos, BlockId};
use crate::lighting::SkylightCalculator;
use super::generation::WorldGenerator;
use super::frame_budget::ChunkLoadThrottler;

/// Chunk loading request with priority
#[derive(Debug, Clone)]
struct ChunkLoadRequest {
    position: ChunkPos,
    priority: i32, // Lower value = higher priority (distance squared)
}

/// Statistics about chunk loading
#[derive(Debug, Clone)]
pub struct ChunkLoadingStats {
    pub loaded_chunks: usize,
    pub cached_chunks: usize,
    pub pending_chunks: usize,
    pub chunks_in_generation: usize,
}

pub struct ChunkManager {
    loaded_chunks: HashMap<ChunkPos, Chunk>,
    view_distance: i32,
    chunk_size: u32,
    generator: Box<dyn WorldGenerator>,
    // Track which chunks need meshing
    dirty_chunks: HashSet<ChunkPos>,
    // Cache for recently unloaded chunks
    chunk_cache: HashMap<ChunkPos, Chunk>,
    cache_size: usize,
    // Chunk loading throttling
    load_queue: VecDeque<ChunkLoadRequest>,
    max_chunks_per_frame: usize,
    // Track chunks being generated to avoid duplicates
    chunks_in_generation: HashSet<ChunkPos>,
    // Frame budget management
    throttler: ChunkLoadThrottler,
}

impl ChunkManager {
    pub fn new(view_distance: i32, chunk_size: u32, generator: Box<dyn WorldGenerator>) -> Self {
        Self {
            loaded_chunks: HashMap::new(),
            view_distance,
            chunk_size,
            generator,
            dirty_chunks: HashSet::new(),
            chunk_cache: HashMap::new(),
            cache_size: 64, // Cache up to 64 chunks
            load_queue: VecDeque::new(),
            max_chunks_per_frame: 5, // Load at most 5 chunks per frame
            chunks_in_generation: HashSet::new(),
            throttler: ChunkLoadThrottler::new(),
        }
    }
    
    /// Set the maximum number of chunks to load per frame
    pub fn set_max_chunks_per_frame(&mut self, max: usize) {
        self.max_chunks_per_frame = max.max(1); // Ensure at least 1
        self.throttler.set_chunks_per_frame(max);
    }
    
    /// Enable or disable adaptive chunk loading
    pub fn set_adaptive_loading(&mut self, enabled: bool) {
        self.throttler.set_adaptive_mode(enabled);
    }
    
    /// Get loading statistics
    pub fn get_loading_stats(&self) -> ChunkLoadingStats {
        ChunkLoadingStats {
            loaded_chunks: self.loaded_chunks.len(),
            cached_chunks: self.chunk_cache.len(),
            pending_chunks: self.load_queue.len(),
            chunks_in_generation: self.chunks_in_generation.len(),
        }
    }
    
    pub fn update_loaded_chunks(&mut self, player_pos: Point3<f32>) {
        // Start frame budget tracking
        self.throttler.start_frame();
        
        // Convert player position to chunk coordinates
        let player_chunk = ChunkPos::new(
            (player_pos.x / self.chunk_size as f32).floor() as i32,
            (player_pos.y / self.chunk_size as f32).floor() as i32,
            (player_pos.z / self.chunk_size as f32).floor() as i32,
        );
        
        // Log the first few updates for debugging
        static UPDATE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = UPDATE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count < 5 {
            log::info!("[ChunkManager::update_loaded_chunks] Update #{} - player chunk: {:?}, loaded: {}, in queue: {}", 
                     count + 1, player_chunk, self.loaded_chunks.len(), self.load_queue.len());
        }
        
        // First, unload chunks that are too far
        self.unload_distant_chunks(player_chunk);
        
        // Queue new chunks that need to be loaded
        self.queue_chunks_for_loading(player_chunk);
        
        // Process the load queue with throttling
        self.process_load_queue();
    }
    
    /// Unload chunks that are outside the view distance
    fn unload_distant_chunks(&mut self, player_chunk: ChunkPos) {
        let unload_distance_sq = (self.view_distance + 2) * (self.view_distance + 2); // Small buffer
        let mut chunks_to_unload = Vec::new();
        
        for &chunk_pos in self.loaded_chunks.keys() {
            let distance_sq = chunk_pos.distance_squared_to(player_chunk);
            if distance_sq > unload_distance_sq {
                chunks_to_unload.push(chunk_pos);
            }
        }
        
        // Unload chunks and add to cache
        for chunk_pos in chunks_to_unload {
            if let Some(chunk) = self.loaded_chunks.remove(&chunk_pos) {
                // Remove from generation tracking
                self.chunks_in_generation.remove(&chunk_pos);
                
                // Add to cache
                self.chunk_cache.insert(chunk_pos, chunk);
                
                // Trim cache if too large using LRU-like behavior
                if self.chunk_cache.len() > self.cache_size {
                    // Find the furthest cached chunk from player
                    if let Some(furthest_pos) = self.chunk_cache
                        .keys()
                        .max_by_key(|pos| pos.distance_squared_to(player_chunk))
                        .cloned()
                    {
                        self.chunk_cache.remove(&furthest_pos);
                    }
                }
            }
        }
    }
    
    /// Queue chunks that need to be loaded based on player position
    fn queue_chunks_for_loading(&mut self, player_chunk: ChunkPos) {
        // Clear and rebuild the queue with current priorities
        self.load_queue.clear();
        let mut new_requests = Vec::new();
        
        // Log the first few calls
        static QUEUE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = QUEUE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let log_this = count < 5;
        
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
                        
                        // Only queue if not already loaded or being generated
                        if !self.loaded_chunks.contains_key(&chunk_pos) 
                            && !self.chunks_in_generation.contains(&chunk_pos) {
                            new_requests.push(ChunkLoadRequest {
                                position: chunk_pos,
                                priority: distance_sq,
                            });
                        }
                    }
                }
            }
        }
        
        // Sort by priority (closest chunks first)
        new_requests.sort_by_key(|req| req.priority);
        
        if log_this {
            log::info!("[ChunkManager::queue_chunks_for_loading] Queued {} chunks for loading", new_requests.len());
        }
        
        // Add to queue
        self.load_queue.extend(new_requests);
    }
    
    /// Process the load queue with frame-based throttling
    fn process_load_queue(&mut self) {
        let mut chunks_loaded = 0;
        let chunks_per_frame = self.throttler.get_chunks_per_frame();
        
        while chunks_loaded < chunks_per_frame && !self.load_queue.is_empty() && self.throttler.can_load_chunk() {
            if let Some(request) = self.load_queue.pop_front() {
                let chunk_pos = request.position;
                
                // Skip if already loaded (can happen due to queue updates)
                if self.loaded_chunks.contains_key(&chunk_pos) {
                    continue;
                }
                
                // Mark as being generated
                self.chunks_in_generation.insert(chunk_pos);
                
                let load_start = Instant::now();
                
                // Check cache first
                let chunk = if let Some(cached_chunk) = self.chunk_cache.remove(&chunk_pos) {
                    cached_chunk
                } else {
                    // Generate new chunk
                    self.generator.generate_chunk(chunk_pos, self.chunk_size)
                };
                
                let load_duration = load_start.elapsed();
                self.throttler.record_chunk_load(load_duration);
                
                // Remove from generation tracking
                self.chunks_in_generation.remove(&chunk_pos);
                
                // Add to loaded chunks
                self.loaded_chunks.insert(chunk_pos, chunk);
                self.dirty_chunks.insert(chunk_pos);
                
                chunks_loaded += 1;
            }
        }
    }
    
    /// Get the number of chunks waiting to be loaded
    pub fn get_pending_chunk_count(&self) -> usize {
        self.load_queue.len()
    }
    
    /// Check if chunk loading is in progress
    pub fn is_loading(&self) -> bool {
        !self.load_queue.is_empty() || !self.chunks_in_generation.is_empty()
    }
    
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.loaded_chunks.get(&pos)
    }
    
    pub fn get_chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        if let Some(chunk) = self.loaded_chunks.get_mut(&pos) {
            self.dirty_chunks.insert(pos);
            Some(chunk)
        } else {
            None
        }
    }
    
    pub fn get_block(&self, pos: VoxelPos) -> BlockId {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.loaded_chunks.get(&chunk_pos) {
            chunk.get_block(local_pos.0, local_pos.1, local_pos.2)
        } else {
            BlockId::AIR
        }
    }
    
    pub fn set_block(&mut self, pos: VoxelPos, block: BlockId) {
        let chunk_pos = pos.to_chunk_pos(self.chunk_size);
        let local_pos = pos.to_local_pos(self.chunk_size);
        
        if let Some(chunk) = self.get_chunk_mut(chunk_pos) {
            chunk.set_block(local_pos.0, local_pos.1, local_pos.2, block);
            
            // Mark neighboring chunks as dirty if on edge
            if local_pos.0 == 0 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x - 1, chunk_pos.y, chunk_pos.z));
            }
            if local_pos.0 == self.chunk_size - 1 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x + 1, chunk_pos.y, chunk_pos.z));
            }
            if local_pos.1 == 0 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y - 1, chunk_pos.z));
            }
            if local_pos.1 == self.chunk_size - 1 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y + 1, chunk_pos.z));
            }
            if local_pos.2 == 0 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y, chunk_pos.z - 1));
            }
            if local_pos.2 == self.chunk_size - 1 {
                self.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y, chunk_pos.z + 1));
            }
        }
    }
    
    pub fn get_loaded_chunks(&self) -> &HashMap<ChunkPos, Chunk> {
        &self.loaded_chunks
    }
    
    pub fn take_dirty_chunks(&mut self) -> HashSet<ChunkPos> {
        std::mem::take(&mut self.dirty_chunks)
    }
    
    pub fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        self.generator.get_surface_height(world_x, world_z)
    }
}