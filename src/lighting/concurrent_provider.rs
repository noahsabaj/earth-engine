use std::sync::Arc;
use parking_lot::RwLock;
use crate::{
    world::{VoxelPos, BlockId, ChunkPos, ConcurrentChunkManager, ParallelChunkManager},
    lighting::parallel_propagator::BlockProvider,
};

/// Thread-safe block provider for concurrent chunk manager
pub struct ConcurrentBlockProvider {
    chunk_manager: Arc<ConcurrentChunkManager>,
    chunk_size: u32,
}

impl ConcurrentBlockProvider {
    pub fn new(chunk_manager: Arc<ConcurrentChunkManager>, chunk_size: u32) -> Self {
        Self {
            chunk_manager,
            chunk_size,
        }
    }
}

impl BlockProvider for ConcurrentBlockProvider {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        self.chunk_manager.get_block(pos)
    }
    
    fn is_transparent(&self, pos: VoxelPos) -> bool {
        let block = self.get_block(pos);
        // TODO: This should check block registry for transparency
        // For now, only air is transparent
        block == BlockId::AIR
    }
}

/// Thread-safe block provider for parallel chunk manager
pub struct ParallelBlockProvider {
    chunk_manager: Arc<ParallelChunkManager>,
}

impl ParallelBlockProvider {
    pub fn new(chunk_manager: Arc<ParallelChunkManager>) -> Self {
        Self { chunk_manager }
    }
}

impl BlockProvider for ParallelBlockProvider {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        self.chunk_manager.get_block(pos)
    }
    
    fn is_transparent(&self, pos: VoxelPos) -> bool {
        let block = self.get_block(pos);
        // TODO: This should check block registry for transparency
        // For now, only air is transparent
        block == BlockId::AIR
    }
}

/// Test block provider for benchmarking
pub struct TestBlockProvider {
    chunks: Arc<RwLock<std::collections::HashMap<ChunkPos, Vec<BlockId>>>>,
    chunk_size: u32,
}

impl TestBlockProvider {
    pub fn new(chunk_size: u32) -> Self {
        Self {
            chunks: Arc::new(RwLock::new(std::collections::HashMap::new())),
            chunk_size,
        }
    }
    
    pub fn set_chunk(&self, chunk_pos: ChunkPos, blocks: Vec<BlockId>) {
        self.chunks.write().insert(chunk_pos, blocks);
    }
    
    fn world_to_chunk_pos(&self, pos: VoxelPos) -> ChunkPos {
        ChunkPos::new(
            pos.x.div_euclid(self.chunk_size as i32),
            pos.y.div_euclid(self.chunk_size as i32),
            pos.z.div_euclid(self.chunk_size as i32),
        )
    }
    
    fn world_to_local_index(&self, pos: VoxelPos) -> usize {
        let x = pos.x.rem_euclid(self.chunk_size as i32) as u32;
        let y = pos.y.rem_euclid(self.chunk_size as i32) as u32;
        let z = pos.z.rem_euclid(self.chunk_size as i32) as u32;
        (y * self.chunk_size * self.chunk_size + z * self.chunk_size + x) as usize
    }
}

impl BlockProvider for TestBlockProvider {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        let chunk_pos = self.world_to_chunk_pos(pos);
        let local_idx = self.world_to_local_index(pos);
        
        self.chunks.read()
            .get(&chunk_pos)
            .and_then(|blocks| blocks.get(local_idx))
            .copied()
            .unwrap_or(BlockId::AIR)
    }
    
    fn is_transparent(&self, pos: VoxelPos) -> bool {
        self.get_block(pos) == BlockId::AIR
    }
}