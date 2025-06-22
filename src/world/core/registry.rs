use super::{Block, BlockId};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry that stores all block types
pub struct BlockRegistry {
    blocks: HashMap<BlockId, Arc<dyn Block>>,
    name_to_id: HashMap<String, BlockId>,
    next_engine_id: u16,
    next_game_id: u16,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            name_to_id: HashMap::new(),
            next_engine_id: 1, // 0 is reserved for AIR, engine blocks use 1-99
            next_game_id: 100, // Game blocks start at 100
        }
    }

    /// Register a new block type
    pub fn register<B: Block + 'static>(&mut self, name: &str, block: B) -> BlockId {
        // Debug logging to track ID assignment
        log::info!("BlockRegistry::register called for '{}'", name);
        log::info!("  - Current next_engine_id: {}, next_game_id: {}", self.next_engine_id, self.next_game_id);
        
        // Determine if this is an engine block or game block based on name prefix
        let is_engine_block = name.starts_with("engine:") || !name.contains(':');
        log::info!("  - Checking '{}': starts_with('engine:')={}, contains(':')={}, is_engine_block={}", 
                  name, 
                  name.starts_with("engine:"), 
                  name.contains(':'),
                  is_engine_block);
        
        let id = if is_engine_block {
            // Engine blocks use IDs 1-99
            if self.next_engine_id >= 100 {
                panic!("Too many engine blocks registered (max 99)");
            }
            let id = BlockId(self.next_engine_id);
            self.next_engine_id += 1;
            log::info!("  - Assigned ENGINE block ID {} to '{}'", id.0, name);
            id
        } else {
            // Game blocks (with mod prefix like "danger_money:") use IDs 100+
            let id = BlockId(self.next_game_id);
            self.next_game_id += 1;
            log::info!("  - Assigned GAME block ID {} to '{}'", id.0, name);
            id
        };

        self.blocks.insert(id, Arc::new(block));
        self.name_to_id.insert(name.to_string(), id);

        log::info!("Registered block '{}' with ID {} (engine: {}, game: {})", 
                  name, id.0, self.next_engine_id, self.next_game_id);
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
