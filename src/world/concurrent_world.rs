use std::sync::Arc;
use cgmath::Point3;
use crate::{BlockId, VoxelPos, ChunkPos};
use crate::thread_pool::{ThreadPoolManager, PoolCategory};
use super::{ConcurrentChunkManager, WorldGenerator};

/// Thread-safe world implementation for parallel operations
pub struct ConcurrentWorld {
    chunk_manager: Arc<ConcurrentChunkManager>,
    chunk_size: u32,
    thread_pool_manager: Arc<ThreadPoolManager>,
}

impl ConcurrentWorld {
    pub fn new(
        chunk_size: u32,
        view_distance: i32,
        generator: Box<dyn WorldGenerator>
    ) -> Self {
        // Get thread pool manager
        let thread_pool_manager = ThreadPoolManager::global();

        let chunk_manager = Arc::new(ConcurrentChunkManager::new(
            view_distance,
            chunk_size,
            generator,
        ));

        Self {
            chunk_manager,
            chunk_size,
            thread_pool_manager,
        }
    }

    /// Update loaded chunks and trigger parallel generation
    pub fn update(&self, player_pos: Point3<f32>) {
        // Update chunk loading state
        self.chunk_manager.update_loaded_chunks(player_pos);

        // Process generation queue in parallel
        let manager = Arc::clone(&self.chunk_manager);
        self.thread_pool_manager.spawn(PoolCategory::WorldGeneration, move || {
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