
use std::sync::Arc;
use parking_lot::RwLock;
use crate::{
    world::{VoxelPos, BlockId, ChunkPos, management::UnifiedWorldManager},
    lighting::parallel_propagator::BlockProvider,
};

/// Thread-safe block provider for unified world manager
pub struct UnifiedBlockProvider {
    world_manager: Arc<RwLock<UnifiedWorldManager>>,
    chunk_size: u32,
}

impl UnifiedBlockProvider {
    pub fn new(world_manager: Arc<RwLock<UnifiedWorldManager>>, chunk_size: u32) -> Self {
        Self {
            world_manager,
            chunk_size,
        }
    }
}

impl BlockProvider for UnifiedBlockProvider {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        self.world_manager.read().get_block(pos)
    }
    
    fn is_transparent(&self, pos: VoxelPos) -> bool {
        let block = self.get_block(pos);
        // TODO: This should check block registry for transparency
        // For now, only air is transparent
        block == BlockId::AIR
    }
}

/// Deprecated: Use UnifiedBlockProvider instead
pub type ConcurrentBlockProvider = UnifiedBlockProvider;

/// Thread-safe block provider for parallel chunk manager (deprecated)
pub struct ParallelBlockProvider {
    world_manager: Arc<RwLock<UnifiedWorldManager>>,
}

impl ParallelBlockProvider {
    pub fn new(world_manager: Arc<RwLock<UnifiedWorldManager>>) -> Self {
        Self { world_manager }
    }
}

impl BlockProvider for ParallelBlockProvider {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        self.world_manager.read().get_block(pos)
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