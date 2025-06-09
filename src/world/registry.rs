use crate::world::{Block, BlockId};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry that stores all block types
pub struct BlockRegistry {
    blocks: HashMap<BlockId, Arc<dyn Block>>,
    name_to_id: HashMap<String, BlockId>,
    next_id: u16,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            name_to_id: HashMap::new(),
            next_id: 1, // 0 is reserved for AIR
        }
    }
    
    /// Register a new block type
    pub fn register<B: Block + 'static>(&mut self, name: &str, block: B) -> BlockId {
        let id = BlockId(self.next_id);
        self.next_id += 1;
        
        self.blocks.insert(id, Arc::new(block));
        self.name_to_id.insert(name.to_string(), id);
        
        id
    }
    
    /// Get a block by ID
    pub fn get_block(&self, id: BlockId) -> Option<Arc<dyn Block>> {
        self.blocks.get(&id).cloned()
    }
    
    /// Get a block ID by name
    pub fn get_id(&self, name: &str) -> Option<BlockId> {
        self.name_to_id.get(name).copied()
    }
}