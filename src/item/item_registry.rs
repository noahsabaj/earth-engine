use crate::item::{ItemType, ItemId, BlockItem, ToolItem, MaterialItem};
use crate::{BlockId, crafting::tool::{Tool, ToolType, ToolMaterial}};
use std::collections::HashMap;

/// Registry for all item types
pub struct ItemRegistry {
    items: HashMap<ItemId, ItemType>,
    block_to_item: HashMap<BlockId, ItemId>,
}

impl ItemRegistry {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            block_to_item: HashMap::new(),
        }
    }
    
    /// Register a new item
    pub fn register(&mut self, id: ItemId, item: ItemType) {
        // If this is a block item, also register the block -> item mapping
        if let ItemType::Block(ref block_item) = item {
            self.block_to_item.insert(block_item.block_id, id);
        }
        
        self.items.insert(id, item);
    }
    
    /// Get an item by ID
    pub fn get_item(&self, id: ItemId) -> Option<&ItemType> {
        self.items.get(&id)
    }
    
    /// Get the item ID for a block
    pub fn get_item_for_block(&self, block_id: BlockId) -> Option<ItemId> {
        self.block_to_item.get(&block_id).copied()
    }
    
    /// Initialize with default items
    pub fn init_default_items(&mut self) {
        // Register block items
        self.register(ItemId::GRASS_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(1) }));
        self.register(ItemId::DIRT_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(2) }));
        self.register(ItemId::STONE_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(3) }));
        self.register(ItemId::WOOD_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(4) }));
        self.register(ItemId::SAND_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(5) }));
        self.register(ItemId::WATER_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(6) }));
        self.register(ItemId::LADDER_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(7) }));
        self.register(ItemId::TORCH_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(8) }));
        self.register(ItemId::COAL_ORE_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(9) }));
        self.register(ItemId::IRON_ORE_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(10) }));
        self.register(ItemId::GOLD_ORE_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(11) }));
        self.register(ItemId::DIAMOND_ORE_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(12) }));
        self.register(ItemId::COBBLESTONE_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(13) }));
        self.register(ItemId::PLANKS_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(14) }));
        self.register(ItemId::CRAFTING_TABLE_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(15) }));
        self.register(ItemId::FURNACE_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(16) }));
        self.register(ItemId::CHEST_BLOCK, ItemType::Block(BlockItem { block_id: BlockId(17) }));
        
        // Register material items
        self.register(ItemId::STICK, ItemType::Material(MaterialItem {
            name: "Stick".to_string(),
            texture_id: 0,
        }));
        
        self.register(ItemId::COAL, ItemType::Material(MaterialItem {
            name: "Coal".to_string(),
            texture_id: 0,
        }));
        
        self.register(ItemId::IRON_INGOT, ItemType::Material(MaterialItem {
            name: "Iron Ingot".to_string(),
            texture_id: 0,
        }));
        
        self.register(ItemId::GOLD_INGOT, ItemType::Material(MaterialItem {
            name: "Gold Ingot".to_string(),
            texture_id: 0,
        }));
        
        self.register(ItemId::DIAMOND, ItemType::Material(MaterialItem {
            name: "Diamond".to_string(),
            texture_id: 0,
        }));
        
        // Register tools
        // Pickaxes
        self.register(ItemId::WOODEN_PICKAXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Pickaxe, ToolMaterial::Wood),
        }));
        self.register(ItemId::STONE_PICKAXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Pickaxe, ToolMaterial::Stone),
        }));
        self.register(ItemId::IRON_PICKAXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Pickaxe, ToolMaterial::Iron),
        }));
        self.register(ItemId::GOLD_PICKAXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Pickaxe, ToolMaterial::Gold),
        }));
        self.register(ItemId::DIAMOND_PICKAXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Pickaxe, ToolMaterial::Diamond),
        }));
        
        // Axes
        self.register(ItemId::WOODEN_AXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Axe, ToolMaterial::Wood),
        }));
        self.register(ItemId::STONE_AXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Axe, ToolMaterial::Stone),
        }));
        self.register(ItemId::IRON_AXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Axe, ToolMaterial::Iron),
        }));
        self.register(ItemId::GOLD_AXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Axe, ToolMaterial::Gold),
        }));
        self.register(ItemId::DIAMOND_AXE, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Axe, ToolMaterial::Diamond),
        }));
        
        // Shovels
        self.register(ItemId::WOODEN_SHOVEL, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Shovel, ToolMaterial::Wood),
        }));
        self.register(ItemId::STONE_SHOVEL, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Shovel, ToolMaterial::Stone),
        }));
        self.register(ItemId::IRON_SHOVEL, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Shovel, ToolMaterial::Iron),
        }));
        self.register(ItemId::GOLD_SHOVEL, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Shovel, ToolMaterial::Gold),
        }));
        self.register(ItemId::DIAMOND_SHOVEL, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Shovel, ToolMaterial::Diamond),
        }));
        
        // Swords
        self.register(ItemId::WOODEN_SWORD, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Sword, ToolMaterial::Wood),
        }));
        self.register(ItemId::STONE_SWORD, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Sword, ToolMaterial::Stone),
        }));
        self.register(ItemId::IRON_SWORD, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Sword, ToolMaterial::Iron),
        }));
        self.register(ItemId::GOLD_SWORD, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Sword, ToolMaterial::Gold),
        }));
        self.register(ItemId::DIAMOND_SWORD, ItemType::Tool(ToolItem {
            tool: Tool::new(ToolType::Sword, ToolMaterial::Diamond),
        }));
    }
}

impl Default for ItemRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.init_default_items();
        registry
    }
}