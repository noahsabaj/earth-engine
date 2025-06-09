use crate::{BlockId, crafting::tool::Tool};
use serde::{Serialize, Deserialize};

/// Unique identifier for an item type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u32);

/// Different types of items in the game
#[derive(Debug, Clone)]
pub enum ItemType {
    /// A block that can be placed
    Block(BlockItem),
    /// A tool that can be used
    Tool(ToolItem),
    /// A raw material (coal, iron ingot, etc.)
    Material(MaterialItem),
    /// Food item
    Food(FoodItem),
}

/// An item that represents a placeable block
#[derive(Debug, Clone)]
pub struct BlockItem {
    pub block_id: BlockId,
}

/// An item that represents a tool
#[derive(Debug, Clone)]
pub struct ToolItem {
    pub tool: Tool,
}

/// A material item (not placeable)
#[derive(Debug, Clone)]
pub struct MaterialItem {
    pub name: String,
    pub texture_id: u32,
}

/// A food item
#[derive(Debug, Clone)]
pub struct FoodItem {
    pub name: String,
    pub hunger_restored: i32,
    pub saturation: f32,
}

impl ItemType {
    /// Get the display name for this item
    pub fn get_name(&self) -> String {
        match self {
            ItemType::Block(block) => {
                // In a real implementation, would look up block name
                format!("Block {:?}", block.block_id)
            }
            ItemType::Tool(tool) => {
                format!("{:?} {:?}", tool.tool.material, tool.tool.tool_type)
            }
            ItemType::Material(mat) => mat.name.clone(),
            ItemType::Food(food) => food.name.clone(),
        }
    }
    
    /// Get the maximum stack size for this item
    pub fn max_stack_size(&self) -> u32 {
        match self {
            ItemType::Block(_) => 64,
            ItemType::Tool(_) => 1, // Tools don't stack
            ItemType::Material(_) => 64,
            ItemType::Food(_) => 64,
        }
    }
    
    /// Check if this item can be used as a tool
    pub fn as_tool(&self) -> Option<&Tool> {
        match self {
            ItemType::Tool(tool_item) => Some(&tool_item.tool),
            _ => None,
        }
    }
    
    /// Check if this item can be placed as a block
    pub fn as_block(&self) -> Option<BlockId> {
        match self {
            ItemType::Block(block_item) => Some(block_item.block_id),
            _ => None,
        }
    }
}

/// Common item IDs
impl ItemId {
    // Blocks (0-999)
    pub const GRASS_BLOCK: ItemId = ItemId(1);
    pub const DIRT_BLOCK: ItemId = ItemId(2);
    pub const STONE_BLOCK: ItemId = ItemId(3);
    pub const WOOD_BLOCK: ItemId = ItemId(4);
    pub const SAND_BLOCK: ItemId = ItemId(5);
    pub const WATER_BLOCK: ItemId = ItemId(6);
    pub const LADDER_BLOCK: ItemId = ItemId(7);
    pub const TORCH_BLOCK: ItemId = ItemId(8);
    pub const COAL_ORE_BLOCK: ItemId = ItemId(9);
    pub const IRON_ORE_BLOCK: ItemId = ItemId(10);
    pub const GOLD_ORE_BLOCK: ItemId = ItemId(11);
    pub const DIAMOND_ORE_BLOCK: ItemId = ItemId(12);
    pub const COBBLESTONE_BLOCK: ItemId = ItemId(13);
    pub const PLANKS_BLOCK: ItemId = ItemId(14);
    pub const CRAFTING_TABLE_BLOCK: ItemId = ItemId(15);
    pub const FURNACE_BLOCK: ItemId = ItemId(16);
    pub const CHEST_BLOCK: ItemId = ItemId(17);
    
    // Materials (1000-1999)
    pub const STICK: ItemId = ItemId(1000);
    pub const COAL: ItemId = ItemId(1001);
    pub const IRON_INGOT: ItemId = ItemId(1002);
    pub const GOLD_INGOT: ItemId = ItemId(1003);
    pub const DIAMOND: ItemId = ItemId(1004);
    
    // Tools (2000-2999)
    pub const WOODEN_PICKAXE: ItemId = ItemId(2000);
    pub const STONE_PICKAXE: ItemId = ItemId(2001);
    pub const IRON_PICKAXE: ItemId = ItemId(2002);
    pub const GOLD_PICKAXE: ItemId = ItemId(2003);
    pub const DIAMOND_PICKAXE: ItemId = ItemId(2004);
    
    pub const WOODEN_AXE: ItemId = ItemId(2010);
    pub const STONE_AXE: ItemId = ItemId(2011);
    pub const IRON_AXE: ItemId = ItemId(2012);
    pub const GOLD_AXE: ItemId = ItemId(2013);
    pub const DIAMOND_AXE: ItemId = ItemId(2014);
    
    pub const WOODEN_SHOVEL: ItemId = ItemId(2020);
    pub const STONE_SHOVEL: ItemId = ItemId(2021);
    pub const IRON_SHOVEL: ItemId = ItemId(2022);
    pub const GOLD_SHOVEL: ItemId = ItemId(2023);
    pub const DIAMOND_SHOVEL: ItemId = ItemId(2024);
    
    pub const WOODEN_SWORD: ItemId = ItemId(2030);
    pub const STONE_SWORD: ItemId = ItemId(2031);
    pub const IRON_SWORD: ItemId = ItemId(2032);
    pub const GOLD_SWORD: ItemId = ItemId(2033);
    pub const DIAMOND_SWORD: ItemId = ItemId(2034);
    
    // Food (3000-3999)
    pub const APPLE: ItemId = ItemId(3000);
    pub const BREAD: ItemId = ItemId(3001);
    pub const COOKED_BEEF: ItemId = ItemId(3002);
}