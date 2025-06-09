use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;
use rayon::prelude::*;
use cgmath::Point3;
use crate::{Chunk, ChunkPos, VoxelPos, BlockId};
use super::generation::WorldGenerator;
use crossbeam_channel::{unbounded, Sender, Receiver};

/// Thread-safe chunk manager using DashMap for lock-free concurrent access
pub struct ConcurrentChunkManager {
    /// Thread-safe chunk storage
    chunks: Arc<DashMap<ChunkPos, Arc<RwLock<Chunk>>>>,
    /// View distance in chunks
    view_distance: i32,
    /// Size of each chunk
    chunk_size: u32,
    /// World generator
    generator: Arc<dyn WorldGenerator>,
    /// Channel for chunk generation requests
    generation_sender: Sender<ChunkPos>,
    generation_receiver: Receiver<ChunkPos>,
    /// Channel for completed chunks
    completed_sender: Sender<(ChunkPos, Chunk)>,
    completed_receiver: Receiver<(ChunkPos, Chunk)>,
}

impl ConcurrentChunkManager {
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
        }
    }
    
    /// Update loaded chunks based on player position (thread-safe)
    pub fn update_loaded_chunks(&self, player_pos: Point3<f32>) {
        let player_chunk = ChunkPos::new(
            (player_pos.x / self.chunk_size as f32).floor() as i32,
            (player_pos.y / self.chunk_size as f32).floor() as i32,
            (player_pos.z / self.chunk_size as f32).floor() as i32,
        );
        
        // Collect chunks that need to be loaded
        let chunks_to_load: Vec<ChunkPos> = (-self.view_distance..=self.view_distance)
            .into_par_iter()
            .flat_map(|dx| {
                (-self.view_distance..=self.view_distance)
                    .into_par_iter()
                    .flat_map(move |dy| {
                        (-self.view_distance..=self.view_distance)
                            .into_par_iter()
                            .filter_map(move |dz| {
                                let distance_sq = dx * dx + dy * dy + dz * dz;
                                if distance_sq <= self.view_distance * self.view_distance {
                                    let chunk_pos = ChunkPos::new(
                                        player_chunk.x + dx,
                                        player_chunk.y + dy,
                                        player_chunk.z + dz,
                                    );
                                    
                                    // Only load if not already present
                                    if !self.chunks.contains_key(&chunk_pos) {
                                        Some(chunk_pos)
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
        
        // Queue chunk generation requests
        for chunk_pos in chunks_to_load {
            let _ = self.generation_sender.send(chunk_pos);
        }
        
        // Process completed chunks
        while let Ok((pos, chunk)) = self.completed_receiver.try_recv() {
            self.chunks.insert(pos, Arc::new(RwLock::new(chunk)));
        }
        
        // TODO: Implement chunk unloading based on distance
    }
    
    /// Generate chunks in parallel using thread pool
    pub fn process_generation_queue(&self) {
        let mut pending_chunks = Vec::new();
        
        // Collect pending generation requests
        while let Ok(chunk_pos) = self.generation_receiver.try_recv() {
            pending_chunks.push(chunk_pos);
        }
        
        // Generate chunks in parallel
        let chunk_size = self.chunk_size;
        let generator = Arc::clone(&self.generator);
        let sender = self.completed_sender.clone();
        
        pending_chunks.par_iter().for_each(|&chunk_pos| {
            let chunk = generator.generate_chunk(chunk_pos, chunk_size);
            let _ = sender.send((chunk_pos, chunk));
        });
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
    
    /// Get chunks for iteration (returns cloned Arc references)
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
}