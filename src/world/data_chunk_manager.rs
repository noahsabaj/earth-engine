/// Data-Oriented Chunk Manager
/// 
/// Sprint 35: Pure data structures for chunk management.
/// No allocations in hot paths, no internal state mutations.

use crate::ChunkPos;
use bytemuck::{Pod, Zeroable};

/// Maximum chunks that can be loaded at once
pub const MAX_LOADED_CHUNKS: usize = 4096;
/// Maximum view distance in chunks
pub const MAX_VIEW_DISTANCE: i32 = 32;
/// Chunk cache size
pub const CHUNK_CACHE_SIZE: usize = 256;

/// Chunk metadata stored in contiguous arrays
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ChunkMetadata {
    pub position: [i32; 3],
    pub flags: u32, // Bit 0: loaded, Bit 1: dirty, Bit 2: cached
    pub last_access_frame: u32,
    pub generation_seed: u32,
    pub neighbor_mask: u8, // Which neighbors are loaded
    _padding: [u8; 3],
}

/// Chunk manager data - all stored contiguously
pub struct ChunkManagerData {
    /// Chunk metadata array (SoA layout)
    pub metadata: Vec<ChunkMetadata>,
    
    /// Chunk position to index mapping (pre-allocated)
    pub position_to_index: rustc_hash::FxHashMap<ChunkPos, usize>,
    
    /// Active chunk count
    pub active_count: usize,
    
    /// Cached chunk count
    pub cache_count: usize,
    
    /// View distance
    pub view_distance: i32,
    
    /// Chunk size
    pub chunk_size: u32,
    
    /// Current frame number
    pub current_frame: u32,
}

impl ChunkManagerData {
    pub fn new(view_distance: i32, chunk_size: u32) -> Self {
        let mut metadata = Vec::with_capacity(MAX_LOADED_CHUNKS + CHUNK_CACHE_SIZE);
        metadata.resize(MAX_LOADED_CHUNKS + CHUNK_CACHE_SIZE, ChunkMetadata {
            position: [0; 3],
            flags: 0,
            last_access_frame: 0,
            generation_seed: 0,
            neighbor_mask: 0,
            _padding: [0; 3],
        });
        
        Self {
            metadata,
            position_to_index: rustc_hash::FxHashMap::with_capacity_and_hasher(
                MAX_LOADED_CHUNKS + CHUNK_CACHE_SIZE,
                Default::default()
            ),
            active_count: 0,
            cache_count: 0,
            view_distance,
            chunk_size,
            current_frame: 0,
        }
    }
}

/// Chunk update operations - pure functions
pub mod operations {
    use super::*;
    use cgmath::Point3;
    
    /// Chunk flags
    pub mod flags {
        pub const LOADED: u32 = 1 << 0;
        pub const DIRTY: u32 = 1 << 1;
        pub const CACHED: u32 = 1 << 2;
        pub const GENERATING: u32 = 1 << 3;
    }
    
    /// Update loaded chunks based on player position
    pub fn update_loaded_chunks(
        data: &mut ChunkManagerData,
        player_pos: Point3<f32>,
        chunks_to_generate: &mut Vec<ChunkPos>,
        chunks_to_unload: &mut Vec<usize>,
    ) {
        // Clear output vectors (they should be pre-allocated)
        chunks_to_generate.clear();
        chunks_to_unload.clear();
        
        // Increment frame counter
        data.current_frame = data.current_frame.wrapping_add(1);
        
        // Calculate player chunk position
        let player_chunk = ChunkPos::new(
            (player_pos.x / data.chunk_size as f32).floor() as i32,
            (player_pos.y / data.chunk_size as f32).floor() as i32,
            (player_pos.z / data.chunk_size as f32).floor() as i32,
        );
        
        // Mark chunks that should be loaded
        let view_dist_sq = data.view_distance * data.view_distance;
        
        // First pass: mark chunks for unloading
        for i in 0..data.active_count {
            let meta = &data.metadata[i];
            if meta.flags & flags::LOADED != 0 {
                let dx = meta.position[0] - player_chunk.x;
                let dy = meta.position[1] - player_chunk.y;
                let dz = meta.position[2] - player_chunk.z;
                let dist_sq = dx * dx + dy * dy + dz * dz;
                
                if dist_sq > view_dist_sq {
                    chunks_to_unload.push(i);
                }
            }
        }
        
        // Second pass: find chunks to load
        for dx in -data.view_distance..=data.view_distance {
            for dy in -data.view_distance..=data.view_distance {
                for dz in -data.view_distance..=data.view_distance {
                    let dist_sq = dx * dx + dy * dy + dz * dz;
                    if dist_sq <= view_dist_sq {
                        let chunk_pos = ChunkPos::new(
                            player_chunk.x + dx,
                            player_chunk.y + dy,
                            player_chunk.z + dz,
                        );
                        
                        // Check if already loaded
                        if !data.position_to_index.contains_key(&chunk_pos) {
                            chunks_to_generate.push(chunk_pos);
                        }
                    }
                }
            }
        }
    }
    
    /// Load a chunk into the manager
    pub fn load_chunk(
        data: &mut ChunkManagerData,
        pos: ChunkPos,
        generation_seed: u32,
    ) -> Option<usize> {
        // Check if we have space
        if data.active_count >= MAX_LOADED_CHUNKS {
            return None;
        }
        
        // Check cache first
        if let Some(&cache_idx) = data.position_to_index.get(&pos) {
            let meta = &mut data.metadata[cache_idx];
            if meta.flags & flags::CACHED != 0 {
                // Move from cache to active
                meta.flags = flags::LOADED | flags::DIRTY;
                meta.last_access_frame = data.current_frame;
                data.cache_count -= 1;
                return Some(cache_idx);
            }
        }
        
        // Allocate new slot
        let idx = data.active_count;
        data.active_count += 1;
        
        // Initialize metadata
        data.metadata[idx] = ChunkMetadata {
            position: [pos.x, pos.y, pos.z],
            flags: flags::LOADED | flags::DIRTY,
            last_access_frame: data.current_frame,
            generation_seed,
            neighbor_mask: 0,
            _padding: [0; 3],
        };
        
        // Update mapping
        data.position_to_index.insert(pos, idx);
        
        Some(idx)
    }
    
    /// Unload a chunk to cache
    pub fn unload_chunk(data: &mut ChunkManagerData, idx: usize) {
        if idx >= data.active_count {
            return;
        }
        
        let meta = &mut data.metadata[idx];
        let pos = ChunkPos::new(meta.position[0], meta.position[1], meta.position[2]);
        
        // Move to cache if there's space
        if data.cache_count < CHUNK_CACHE_SIZE {
            meta.flags = flags::CACHED;
            data.cache_count += 1;
        } else {
            // Remove from mapping
            data.position_to_index.remove(&pos);
            
            // Swap with last active chunk
            if idx < data.active_count - 1 {
                data.metadata.swap(idx, data.active_count - 1);
                let swapped_pos = ChunkPos::new(
                    data.metadata[idx].position[0],
                    data.metadata[idx].position[1],
                    data.metadata[idx].position[2],
                );
                data.position_to_index.insert(swapped_pos, idx);
            }
            
            data.active_count -= 1;
        }
    }
    
    /// Mark chunk as dirty
    pub fn mark_dirty(data: &mut ChunkManagerData, pos: ChunkPos) {
        if let Some(&idx) = data.position_to_index.get(&pos) {
            data.metadata[idx].flags |= flags::DIRTY;
        }
    }
    
    /// Get all dirty chunks
    pub fn get_dirty_chunks(data: &ChunkManagerData, output: &mut Vec<ChunkPos>) {
        output.clear();
        
        for i in 0..data.active_count {
            let meta = &data.metadata[i];
            if meta.flags & flags::DIRTY != 0 {
                output.push(ChunkPos::new(
                    meta.position[0],
                    meta.position[1],
                    meta.position[2],
                ));
            }
        }
    }
    
    /// Clear dirty flags
    pub fn clear_dirty_flags(data: &mut ChunkManagerData) {
        for i in 0..data.active_count {
            data.metadata[i].flags &= !flags::DIRTY;
        }
    }
    
    /// Update neighbor masks for all chunks
    pub fn update_neighbor_masks(data: &mut ChunkManagerData) {
        // Clear all masks
        for i in 0..data.active_count {
            data.metadata[i].neighbor_mask = 0;
        }
        
        // Update masks based on loaded neighbors
        for i in 0..data.active_count {
            let pos = ChunkPos::new(
                data.metadata[i].position[0],
                data.metadata[i].position[1],
                data.metadata[i].position[2],
            );
            
            // Check each neighbor
            let neighbors = [
                (ChunkPos::new(pos.x - 1, pos.y, pos.z), 0),
                (ChunkPos::new(pos.x + 1, pos.y, pos.z), 1),
                (ChunkPos::new(pos.x, pos.y - 1, pos.z), 2),
                (ChunkPos::new(pos.x, pos.y + 1, pos.z), 3),
                (ChunkPos::new(pos.x, pos.y, pos.z - 1), 4),
                (ChunkPos::new(pos.x, pos.y, pos.z + 1), 5),
            ];
            
            for (neighbor_pos, bit) in neighbors {
                if data.position_to_index.contains_key(&neighbor_pos) {
                    data.metadata[i].neighbor_mask |= 1 << bit;
                }
            }
        }
    }
}

/// Batch operations for efficiency
pub struct ChunkBatchOps {
    /// Pre-allocated vector for chunks to generate
    pub chunks_to_generate: Vec<ChunkPos>,
    /// Pre-allocated vector for chunks to unload
    pub chunks_to_unload: Vec<usize>,
    /// Pre-allocated vector for dirty chunks
    pub dirty_chunks: Vec<ChunkPos>,
}

impl ChunkBatchOps {
    pub fn new() -> Self {
        Self {
            chunks_to_generate: Vec::with_capacity(256),
            chunks_to_unload: Vec::with_capacity(256),
            dirty_chunks: Vec::with_capacity(512),
        }
    }
}

// Note: This is a foundation for data-oriented chunk management.
// The actual chunk data would be stored in a separate buffer system,
// likely integrated with the WorldBuffer from Sprint 21.