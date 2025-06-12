use std::sync::Arc;
use parking_lot::{RwLock, Mutex};
use crate::{ChunkPos, Chunk, World};
use super::async_chunk_renderer::ChunkManager;

/// Adapter to make World work with AsyncChunkRenderer
pub struct WorldAdapter<'a> {
    world: &'a mut World,
    // Cache of chunk wrappers with interior mutability
    chunk_cache: Arc<Mutex<std::collections::HashMap<ChunkPos, Arc<RwLock<Chunk>>>>>,
}

impl<'a> WorldAdapter<'a> {
    pub fn new(world: &'a mut World) -> Self {
        Self {
            world,
            chunk_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
    
    /// Get dirty chunks and wrap them in Arc<RwLock<Chunk>>
    pub fn get_dirty_chunks(&mut self) -> Vec<(ChunkPos, Arc<RwLock<Chunk>>)> {
        let dirty_chunks = self.world.take_dirty_chunks();
        let mut result = Vec::new();
        
        for chunk_pos in dirty_chunks {
            if let Some(chunk) = self.world.get_chunk(chunk_pos) {
                // Clone the chunk and wrap it
                let chunk_clone = chunk.clone();
                let wrapped = Arc::new(RwLock::new(chunk_clone));
                self.chunk_cache.lock().insert(chunk_pos, Arc::clone(&wrapped));
                result.push((chunk_pos, wrapped));
            }
        }
        
        result
    }
    
    /// Update the world with modified chunks from the cache
    pub fn sync_chunks_back(&mut self) {
        let cache = self.chunk_cache.lock();
        for (pos, wrapped_chunk) in cache.iter() {
            let chunk = wrapped_chunk.read();
            if chunk.is_dirty() {
                // Update the chunk in the world
                self.world.set_chunk(*pos, (*chunk).clone());
            }
        }
    }
}

impl<'a> ChunkManager for WorldAdapter<'a> {
    fn chunks_iter(&self) -> Box<dyn Iterator<Item = (ChunkPos, Arc<RwLock<Chunk>>)> + '_> {
        // We need to collect to avoid lifetime issues with the lock
        let cache = self.chunk_cache.lock();
        let items: Vec<_> = cache.iter().map(|(pos, chunk)| (*pos, Arc::clone(chunk))).collect();
        Box::new(items.into_iter())
    }
    
    fn get_chunk(&self, pos: ChunkPos) -> Option<Arc<RwLock<Chunk>>> {
        // First check cache
        {
            let cache = self.chunk_cache.lock();
            if let Some(wrapped) = cache.get(&pos) {
                return Some(Arc::clone(wrapped));
            }
        }
        
        // Otherwise get from world and wrap
        if let Some(chunk) = self.world.get_chunk(pos) {
            let wrapped = Arc::new(RwLock::new(chunk.clone()));
            self.chunk_cache.lock().insert(pos, Arc::clone(&wrapped));
            Some(wrapped)
        } else {
            None
        }
    }
}