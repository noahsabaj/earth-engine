use std::sync::Arc;
use parking_lot::RwLock;
use cgmath::Point3;
use rayon::ThreadPoolBuilder;
use crate::{BlockId, VoxelPos, ChunkPos};
use super::{ConcurrentChunkManager, WorldGenerator};

/// Thread-safe world implementation for parallel operations
pub struct ConcurrentWorld {
    chunk_manager: Arc<ConcurrentChunkManager>,
    chunk_size: u32,
    generation_pool: rayon::ThreadPool,
}

impl ConcurrentWorld {
    pub fn new(
        chunk_size: u32, 
        view_distance: i32, 
        generator: Box<dyn WorldGenerator>
    ) -> Self {
        // Create dedicated thread pool for chunk generation
        let generation_pool = ThreadPoolBuilder::new()
            .num_threads(num_cpus::get().saturating_sub(2).max(2))
            .thread_name(|idx| format!("chunk-gen-{}", idx))
            .build()
            .expect("Failed to create generation thread pool");
        
        let chunk_manager = Arc::new(ConcurrentChunkManager::new(
            view_distance,
            chunk_size,
            generator,
        ));
        
        Self {
            chunk_manager,
            chunk_size,
            generation_pool,
        }
    }
    
    /// Update loaded chunks and trigger parallel generation
    pub fn update(&self, player_pos: Point3<f32>) {
        // Update chunk loading state
        self.chunk_manager.update_loaded_chunks(player_pos);
        
        // Process generation queue in parallel
        let manager = Arc::clone(&self.chunk_manager);
        self.generation_pool.spawn(move || {
            manager.process_generation_queue();
        });
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
    
    /// Get number of loaded chunks
    pub fn loaded_chunk_count(&self) -> usize {
        self.chunk_manager.loaded_chunk_count()
    }
    
    /// Get chunk size
    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }
    
    /// Get chunk manager reference
    pub fn chunk_manager(&self) -> &ConcurrentChunkManager {
        &self.chunk_manager
    }
}