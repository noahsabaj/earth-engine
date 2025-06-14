/// Data-Oriented Chunk Management Functions
/// 
/// Pure functions for transforming chunk data. No methods, no self, just data transformations.
/// Follows DOP principles from Sprint 37.

use std::collections::{HashSet, VecDeque};
use std::time::Instant;
use cgmath::Point3;
use crate::{Chunk, ChunkPos, VoxelPos, BlockId};
use crate::utils::chunk_spatial_hash::{
    ChunkSpatialHash, ChunkDistanceHash,
    spatial_hash_remove, spatial_hash_get_mut, 
    distance_hash_insert, distance_hash_update_center, distance_hash_get_mut
};
use super::generation::WorldGenerator;
use super::frame_budget::ChunkLoadThrottler;

/// Chunk loading request with priority
#[derive(Debug, Clone)]
pub struct ChunkLoadRequest {
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

/// Chunk management data structure (no methods)
/// Pure data - manipulated by free functions only
pub struct ChunkManagerData {
    pub loaded_chunks: ChunkDistanceHash<Chunk>,
    pub view_distance: i32,
    pub chunk_size: u32,
    pub generator: Box<dyn WorldGenerator>,
    /// Track which chunks need meshing
    pub dirty_chunks: HashSet<ChunkPos>,
    /// Cache for recently unloaded chunks
    pub chunk_cache: ChunkSpatialHash<Chunk>,
    pub cache_size: usize,
    /// Chunk loading throttling
    pub load_queue: VecDeque<ChunkLoadRequest>,
    pub max_chunks_per_frame: usize,
    /// Track chunks being generated to avoid duplicates
    pub chunks_in_generation: HashSet<ChunkPos>,
    /// Frame budget management
    pub throttler: ChunkLoadThrottler,
}

impl std::fmt::Debug for ChunkManagerData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkManagerData")
            .field("loaded_chunks", &self.loaded_chunks)
            .field("view_distance", &self.view_distance)
            .field("chunk_size", &self.chunk_size)
            .field("generator", &"<WorldGenerator>")
            .field("dirty_chunks", &self.dirty_chunks)
            .field("chunk_cache", &self.chunk_cache)
            .field("cache_size", &self.cache_size)
            .field("load_queue", &self.load_queue)
            .field("max_chunks_per_frame", &self.max_chunks_per_frame)
            .field("chunks_in_generation", &self.chunks_in_generation)
            .field("throttler", &self.throttler)
            .finish()
    }
}

impl ChunkManagerData {
    /// Create a new chunk manager with the specified chunk size
    pub fn new(chunk_size: u32) -> Self {
        use crate::world::generation::DefaultWorldGenerator;
        
        Self {
            loaded_chunks: ChunkDistanceHash::new(8), // Pass view distance
            view_distance: 8, // Default view distance
            chunk_size,
            generator: Box::new(crate::world::generation::DefaultWorldGenerator::new(
                12345, // seed
                crate::world::block::BlockId::GRASS, // grass_id
                crate::world::block::BlockId::DIRT, // dirt_id
                crate::world::block::BlockId::STONE, // stone_id
                crate::world::block::BlockId::WATER, // water_id
                crate::world::block::BlockId::SAND // sand_id
            )), // Use correct generator with all parameters
            dirty_chunks: HashSet::new(),
            chunk_cache: ChunkSpatialHash::new(),
            cache_size: 100, // Default cache size
            load_queue: VecDeque::new(),
            max_chunks_per_frame: 4, // Default load limit per frame
            chunks_in_generation: HashSet::new(),
            throttler: ChunkLoadThrottler::new(), // No arguments
        }
    }

}

/// Chunk management configuration
#[derive(Debug, Copy, Clone)]
pub struct ChunkManagerConfig {
    pub view_distance: i32,
    pub chunk_size: u32,
    pub cache_size: usize,
    pub max_chunks_per_frame: usize,
}

impl Default for ChunkManagerConfig {
    fn default() -> Self {
        Self {
            view_distance: 8,
            chunk_size: 32,
            cache_size: 64,
            max_chunks_per_frame: 5,
        }
    }
}

/// Create new chunk manager data
/// Pure function - returns data structure, no behavior
pub fn create_chunk_manager_data(
    config: ChunkManagerConfig,
    generator: Box<dyn WorldGenerator>,
) -> ChunkManagerData {
    ChunkManagerData {
        loaded_chunks: ChunkDistanceHash::new(config.view_distance),
        view_distance: config.view_distance,
        chunk_size: config.chunk_size,
        generator,
        dirty_chunks: HashSet::new(),
        chunk_cache: ChunkSpatialHash::new(),
        cache_size: config.cache_size,
        load_queue: VecDeque::new(),
        max_chunks_per_frame: config.max_chunks_per_frame,
        chunks_in_generation: HashSet::new(),
        throttler: ChunkLoadThrottler::new(),
    }
}
    
/// Set the maximum number of chunks to load per frame
/// Pure function - transforms chunk manager data
pub fn set_max_chunks_per_frame(data: &mut ChunkManagerData, max: usize) {
    data.max_chunks_per_frame = max.max(1); // Ensure at least 1
    data.throttler.set_chunks_per_frame(max);
}

/// Enable or disable adaptive chunk loading
/// Pure function - transforms throttler configuration
pub fn set_adaptive_loading(data: &mut ChunkManagerData, enabled: bool) {
    data.throttler.set_adaptive_mode(enabled);
}

/// Get loading statistics
/// Pure function - reads data, no mutation
pub fn get_loading_stats(data: &ChunkManagerData) -> ChunkLoadingStats {
    ChunkLoadingStats {
        loaded_chunks: data.loaded_chunks.len(),
        cached_chunks: data.chunk_cache.len(),
        pending_chunks: data.load_queue.len(),
        chunks_in_generation: data.chunks_in_generation.len(),
    }
}
    
/// Update loaded chunks based on player position
/// Pure function - transforms chunk manager data based on player position
pub fn update_loaded_chunks(data: &mut ChunkManagerData, player_pos: Point3<f32>) {
    // Start frame budget tracking
    data.throttler.start_frame();
    
    // Convert player position to chunk coordinates
    let player_chunk = ChunkPos::new(
        (player_pos.x / data.chunk_size as f32).floor() as i32,
        (player_pos.y / data.chunk_size as f32).floor() as i32,
        (player_pos.z / data.chunk_size as f32).floor() as i32,
    );
    
    // Log the first few updates for debugging
    static UPDATE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let count = UPDATE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if count < 5 {
        log::info!("[chunk_manager::update_loaded_chunks] Update #{} - player chunk: {:?}, loaded: {}, in queue: {}", 
                 count + 1, player_chunk, data.loaded_chunks.len(), data.load_queue.len());
    }
    
    // Update the center position for distance-based storage
    distance_hash_update_center(&mut data.loaded_chunks, player_chunk);
    
    // First, unload chunks that are too far
    unload_distant_chunks(data, player_chunk);
    
    // Queue new chunks that need to be loaded
    queue_chunks_for_loading(data, player_chunk);
    
    // Process the load queue with throttling
    process_load_queue(data);
}
    
/// Unload chunks that are outside the view distance
/// Pure function - transforms cache data based on distance
fn unload_distant_chunks(data: &mut ChunkManagerData, player_chunk: ChunkPos) {
    // ChunkDistanceHash already handles distance culling in update_center
    // We just need to manage the cache for unloaded chunks
    
    // Trim cache if too large using LRU-like behavior
    if data.chunk_cache.len() > data.cache_size {
        let mut furthest_pos: Option<ChunkPos> = None;
        let mut max_distance = 0;
        
        for (pos, _) in data.chunk_cache.iter() {
            let distance = pos.distance_squared_to(player_chunk);
            if distance > max_distance {
                max_distance = distance;
                furthest_pos = Some(pos);
            }
        }
        
        if let Some(pos) = furthest_pos {
            spatial_hash_remove(&mut data.chunk_cache, pos);
        }
    }
}
    
/// Queue chunks that need to be loaded based on player position
/// Pure function - transforms load queue based on spatial requirements
fn queue_chunks_for_loading(data: &mut ChunkManagerData, player_chunk: ChunkPos) {
    // Clear and rebuild the queue with current priorities
    data.load_queue.clear();
    let mut new_requests = Vec::new();
    
    // Log the first few calls
    static QUEUE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let count = QUEUE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let log_this = count < 5;
    
    for dx in -data.view_distance..=data.view_distance {
        for dy in -data.view_distance..=data.view_distance {
            for dz in -data.view_distance..=data.view_distance {
                let distance_sq = dx * dx + dy * dy + dz * dz;
                if distance_sq <= data.view_distance * data.view_distance {
                    let chunk_pos = ChunkPos::new(
                        player_chunk.x + dx,
                        player_chunk.y + dy,
                        player_chunk.z + dz,
                    );
                    
                    // Only queue if not already loaded or being generated
                    if data.loaded_chunks.get(chunk_pos).is_none()
                        && !data.chunks_in_generation.contains(&chunk_pos) {
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
        log::info!("[chunk_manager::queue_chunks_for_loading] Queued {} chunks for loading", new_requests.len());
    }
    
    // Add to queue
    data.load_queue.extend(new_requests);
}
    
/// Process the load queue with frame-based throttling
/// Pure function - transforms loaded chunks based on queue and throttling
fn process_load_queue(data: &mut ChunkManagerData) {
    let mut chunks_loaded = 0;
    let chunks_per_frame = data.throttler.get_chunks_per_frame();
    
    while chunks_loaded < chunks_per_frame && !data.load_queue.is_empty() && data.throttler.can_load_chunk() {
        if let Some(request) = data.load_queue.pop_front() {
            let chunk_pos = request.position;
            
            // Skip if already loaded (can happen due to queue updates)
            if data.loaded_chunks.get(chunk_pos).is_some() {
                continue;
            }
            
            // Mark as being generated
            data.chunks_in_generation.insert(chunk_pos);
            
            let load_start = Instant::now();
            
            // Check cache first
            let chunk = if let Some(cached_chunk) = spatial_hash_remove(&mut data.chunk_cache, chunk_pos) {
                cached_chunk
            } else {
                // Generate new chunk
                data.generator.generate_chunk(chunk_pos, data.chunk_size)
            };
            
            let load_duration = load_start.elapsed();
            data.throttler.record_chunk_load(load_duration);
            
            // Remove from generation tracking
            data.chunks_in_generation.remove(&chunk_pos);
            
            // Add to loaded chunks
            distance_hash_insert(&mut data.loaded_chunks, chunk_pos, chunk);
            data.dirty_chunks.insert(chunk_pos);
            
            chunks_loaded += 1;
        }
    }
}
    
/// Get the number of chunks waiting to be loaded
/// Pure function - reads queue length
pub fn get_pending_chunk_count(data: &ChunkManagerData) -> usize {
    data.load_queue.len()
}

/// Check if chunk loading is in progress
/// Pure function - reads loading state
pub fn is_loading(data: &ChunkManagerData) -> bool {
    !data.load_queue.is_empty() || !data.chunks_in_generation.is_empty()
}

/// Add chunk to manager data
/// Function - transforms chunk data and marks as dirty
pub fn add_chunk_to_manager(data: &mut ChunkManagerData, pos: ChunkPos, chunk: Chunk) {
    distance_hash_insert(&mut data.loaded_chunks, pos, chunk);
    data.dirty_chunks.insert(pos);
}

/// Check if manager has chunk at position
/// Pure function - reads chunk existence
pub fn manager_has_chunk(data: &ChunkManagerData, pos: ChunkPos) -> bool {
    data.loaded_chunks.get(pos).is_some()
}

/// Get chunk at position
/// Pure function - reads chunk data
pub fn get_chunk(data: &ChunkManagerData, pos: ChunkPos) -> Option<&Chunk> {
    data.loaded_chunks.get(pos)
}

/// Get mutable chunk at position and mark as dirty
/// Function - transforms chunk data and dirty set
pub fn get_chunk_mut(data: &mut ChunkManagerData, pos: ChunkPos) -> Option<&mut Chunk> {
    if let Some(chunk) = distance_hash_get_mut(&mut data.loaded_chunks, pos) {
        data.dirty_chunks.insert(pos);
        Some(chunk)
    } else {
        None
    }
}

/// Get block at world position
/// Pure function - reads block data from chunks
pub fn get_block(data: &ChunkManagerData, pos: VoxelPos) -> BlockId {
    let chunk_pos = pos.to_chunk_pos(data.chunk_size);
    let local_pos = pos.to_local_pos(data.chunk_size);
    
    if let Some(chunk) = data.loaded_chunks.get(chunk_pos) {
        chunk.get_block(local_pos.0, local_pos.1, local_pos.2)
    } else {
        BlockId::AIR
    }
}

/// Set block at world position
/// Function - transforms chunk data and marks neighbors dirty
pub fn set_block(data: &mut ChunkManagerData, pos: VoxelPos, block: BlockId) {
    let chunk_pos = pos.to_chunk_pos(data.chunk_size);
    let local_pos = pos.to_local_pos(data.chunk_size);
    
    if let Some(chunk) = get_chunk_mut(data, chunk_pos) {
        chunk.set_block(local_pos.0, local_pos.1, local_pos.2, block);
        
        // Mark neighboring chunks as dirty if on edge
        if local_pos.0 == 0 {
            data.dirty_chunks.insert(ChunkPos::new(chunk_pos.x - 1, chunk_pos.y, chunk_pos.z));
        }
        if local_pos.0 == data.chunk_size - 1 {
            data.dirty_chunks.insert(ChunkPos::new(chunk_pos.x + 1, chunk_pos.y, chunk_pos.z));
        }
        if local_pos.1 == 0 {
            data.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y - 1, chunk_pos.z));
        }
        if local_pos.1 == data.chunk_size - 1 {
            data.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y + 1, chunk_pos.z));
        }
        if local_pos.2 == 0 {
            data.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y, chunk_pos.z - 1));
        }
        if local_pos.2 == data.chunk_size - 1 {
            data.dirty_chunks.insert(ChunkPos::new(chunk_pos.x, chunk_pos.y, chunk_pos.z + 1));
        }
    }
}

/// Get iterator over loaded chunks
/// Pure function - returns iterator over chunk data
pub fn get_loaded_chunks(data: &ChunkManagerData) -> impl Iterator<Item = (ChunkPos, &Chunk)> {
    data.loaded_chunks.iter()
}

/// Take all dirty chunks and clear the set
/// Function - transforms dirty chunks set
pub fn take_dirty_chunks(data: &mut ChunkManagerData) -> HashSet<ChunkPos> {
    std::mem::take(&mut data.dirty_chunks)
}

/// Get surface height at world coordinates
/// Pure function - delegates to generator
pub fn get_surface_height(data: &ChunkManagerData, world_x: f64, world_z: f64) -> i32 {
    data.generator.get_surface_height(world_x, world_z)
}

// ===== COMPATIBILITY LAYER =====
// Temporary aliases for code that hasn't been converted yet

/// Compatibility alias - will be removed in future sprints
#[deprecated(note = "Use ChunkManagerData and pure functions instead")]
pub type ChunkManager = ChunkManagerData;