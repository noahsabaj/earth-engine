use crate::BlockId;
use std::collections::HashMap;

/// Type of tool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolType {
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Sword,
}

/// Material that tools are made from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolMaterial {
    Wood,
    Stone,
    Iron,
    Gold,
    Diamond,
}

impl ToolMaterial {
    /// Get the durability for this material
    pub fn durability(&self) -> u32 {
        match self {
            ToolMaterial::Wood => 59,
            ToolMaterial::Stone => 131,
            ToolMaterial::Iron => 250,
            ToolMaterial::Gold => 32,
            ToolMaterial::Diamond => 1561,
        }
    }
    
    /// Get the mining level for this material
    pub fn mining_level(&self) -> u32 {
        match self {
            ToolMaterial::Wood => 0,
            ToolMaterial::Stone => 1,
            ToolMaterial::Iron => 2,
            ToolMaterial::Gold => 0, // Gold is weak but fast
            ToolMaterial::Diamond => 3,
        }
    }
    
    /// Get the base mining speed multiplier
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            ToolMaterial::Wood => 2.0,
            ToolMaterial::Stone => 4.0,
            ToolMaterial::Iron => 6.0,
            ToolMaterial::Gold => 12.0, // Gold is very fast
            ToolMaterial::Diamond => 8.0,
        }
    }
    
    /// Get the base damage for weapons
    pub fn base_damage(&self) -> f32 {
        match self {
            ToolMaterial::Wood => 4.0,
            ToolMaterial::Stone => 5.0,
            ToolMaterial::Iron => 6.0,
            ToolMaterial::Gold => 4.0,
            ToolMaterial::Diamond => 7.0,
        }
    }
}

/// Tool durability tracking
#[derive(Debug, Clone)]
pub struct ToolDurability {
    pub current: u32,
    pub max: u32,
}

impl ToolDurability {
    pub fn new(max: u32) -> Self {
        Self { current: max, max }
    }
    
    
    /// Get the percentage of durability remaining (0.0 - 1.0)
    pub fn percentage(&self) -> f32 {
        self.current as f32 / self.max as f32
    }
    
    /// Check if the tool is broken
    pub fn is_broken(&self) -> bool {
        self.current == 0
    }
    
}

/// Represents a tool item
#[derive(Debug, Clone)]
pub struct Tool {
    pub tool_type: ToolType,
    pub material: ToolMaterial,
    pub durability: ToolDurability,
    pub enchantments: HashMap<String, u32>, // For future use
}

impl Tool {
    /// Create a new tool
    pub fn new(tool_type: ToolType, material: ToolMaterial) -> Self {
        let max_durability = material.durability();
        Self {
            tool_type,
            material,
            durability: ToolDurability::new(max_durability),
            enchantments: HashMap::new(),
        }
    }
    
    /// Get the effectiveness of this tool on a block
    pub fn get_effectiveness(&self, block: BlockId, block_properties: &BlockProperties) -> ToolEffectiveness {
        // Check if this is the right tool for the job
        let is_correct_tool = match self.tool_type {
            ToolType::Pickaxe => matches!(block_properties.preferred_tool, Some(ToolType::Pickaxe)),
            ToolType::Axe => matches!(block_properties.preferred_tool, Some(ToolType::Axe)),
            ToolType::Shovel => matches!(block_properties.preferred_tool, Some(ToolType::Shovel)),
            ToolType::Hoe => matches!(block_properties.preferred_tool, Some(ToolType::Hoe)),
            ToolType::Sword => false, // Swords aren't mining tools
        };
        
        // Check if we meet the mining level requirement
        let meets_level = self.material.mining_level() >= block_properties.mining_level;
        
        // Calculate speed multiplier
        let speed = if is_correct_tool && meets_level {
            self.material.speed_multiplier()
        } else if is_correct_tool {
            1.0 // Right tool but wrong level - normal speed
        } else {
            1.0 // Wrong tool - normal speed
        };
        
        // Determine if the block can be harvested
        let can_harvest = meets_level || block_properties.mining_level == 0;
        
        ToolEffectiveness {
            speed_multiplier: speed,
            can_harvest,
            is_correct_tool,
        }
    }
    
    /// Get the damage this tool deals (for weapons)
    pub fn get_damage(&self) -> f32 {
        let base_damage = self.material.base_damage();
        match self.tool_type {
            ToolType::Sword => base_damage,
            ToolType::Axe => base_damage * 0.9, // Axes do slightly less damage than swords
            ToolType::Pickaxe => base_damage * 0.7,
            ToolType::Shovel => base_damage * 0.5,
            ToolType::Hoe => base_damage * 0.3,
        }
    }
}

/// Result of checking tool effectiveness
#[derive(Debug, Clone)]
pub struct ToolEffectiveness {
    /// Speed multiplier for breaking the block
    pub speed_multiplier: f32,
    /// Whether the tool can harvest this block
    pub can_harvest: bool,
    /// Whether this is the correct tool type
    pub is_correct_tool: bool,
}

/// Properties related to mining a block
#[derive(Debug, Clone)]
pub struct BlockProperties {
    /// Hardness of the block (time to break in seconds with no tool)
    pub hardness: f32,
    /// Minimum mining level required to harvest
    pub mining_level: u32,
    /// Preferred tool type for this block
    pub preferred_tool: Option<ToolType>,
    /// Whether the block can be broken by hand
    pub breakable_by_hand: bool,
    /// What the block drops when broken
    pub drops: Vec<ItemDrop>,
}

/// Represents what a block drops when broken
#[derive(Debug, Clone)]
pub struct ItemDrop {
    pub block_id: BlockId,
    pub min_count: u32,
    pub max_count: u32,
    pub requires_tool: bool,
}

impl ItemDrop {
    pub fn simple(block_id: BlockId) -> Self {
        Self {
            block_id,
            min_count: 1,
            max_count: 1,
            requires_tool: false,
        }
    }
    
    pub fn with_count(block_id: BlockId, min: u32, max: u32) -> Self {
        Self {
            block_id,
            min_count: min,
            max_count: max,
            requires_tool: false,
        }
    }
    
    pub fn tool_required(mut self) -> Self {
        self.requires_tool = true;
        self
    }
}

/// Get default block properties for a block type
pub fn get_block_properties(block: BlockId) -> BlockProperties {
    match block {
        BlockId::Air => BlockProperties {
            hardness: 0.0,
            mining_level: 0,
            preferred_tool: None,
            breakable_by_hand: true,
            drops: vec![],
        },
        BlockId::Grass | BlockId::Dirt => BlockProperties {
            hardness: 0.5,
            mining_level: 0,
            preferred_tool: Some(ToolType::Shovel),
            breakable_by_hand: true,
            drops: vec![ItemDrop::simple(BlockId::Dirt)],
        },
        BlockId::Stone => BlockProperties {
            hardness: 1.5,
            mining_level: 0,
            preferred_tool: Some(ToolType::Pickaxe),
            breakable_by_hand: true,
            drops: vec![ItemDrop::simple(BlockId::Stone).tool_required()], // Would drop cobblestone
        },
        BlockId::Wood => BlockProperties {
            hardness: 2.0,
            mining_level: 0,
            preferred_tool: Some(ToolType::Axe),
            breakable_by_hand: true,
            drops: vec![ItemDrop::simple(BlockId::Wood)],
        },
        BlockId::Sand => BlockProperties {
            hardness: 0.5,
            mining_level: 0,
            preferred_tool: Some(ToolType::Shovel),
            breakable_by_hand: true,
            drops: vec![ItemDrop::simple(BlockId::Sand)],
        },
        BlockId::Water | BlockId::Lava => BlockProperties {
            hardness: 100.0, // Can't break fluids
            mining_level: 0,
            preferred_tool: None,
            breakable_by_hand: false,
            drops: vec![],
        },
        BlockId::LADDER => BlockProperties {
            hardness: 0.4,
            mining_level: 0,
            preferred_tool: Some(ToolType::Axe),
            breakable_by_hand: true,
            drops: vec![ItemDrop::simple(BlockId::LADDER)],
        },
        BlockId::TORCH => BlockProperties {
            hardness: 0.0,
            mining_level: 0,
            preferred_tool: None,
            breakable_by_hand: true,
            drops: vec![ItemDrop::simple(BlockId::TORCH)],
        },
        BlockId::COAL_ORE => BlockProperties {
            hardness: 3.0,
            mining_level: 0,
            preferred_tool: Some(ToolType::Pickaxe),
            breakable_by_hand: false,
            drops: vec![ItemDrop::with_count(BlockId::COAL_ORE, 1, 1).tool_required()], // Would drop coal item
        },
        BlockId::IRON_ORE => BlockProperties {
            hardness: 3.0,
            mining_level: 1, // Requires stone pickaxe
            preferred_tool: Some(ToolType::Pickaxe),
            breakable_by_hand: false,
            drops: vec![ItemDrop::simple(BlockId::IRON_ORE).tool_required()],
        },
        BlockId::GOLD_ORE => BlockProperties {
            hardness: 3.0,
            mining_level: 2, // Requires iron pickaxe
            preferred_tool: Some(ToolType::Pickaxe),
            breakable_by_hand: false,
            drops: vec![ItemDrop::simple(BlockId::GOLD_ORE).tool_required()],
        },
        BlockId::DIAMOND_ORE => BlockProperties {
            hardness: 3.0,
            mining_level: 2, // Requires iron pickaxe
            preferred_tool: Some(ToolType::Pickaxe),
            breakable_by_hand: false,
            drops: vec![ItemDrop::with_count(BlockId::DIAMOND_ORE, 1, 1).tool_required()], // Would drop diamond item
        },
        _ => BlockProperties {
            hardness: 1.0,
            mining_level: 0,
            preferred_tool: None,
            breakable_by_hand: true,
            drops: vec![ItemDrop::simple(block)],
        },
    }
}

/// Use the tool, reducing durability
/// Function - transforms tool durability data
pub fn use_tool_durability(durability: &mut ToolDurability) -> bool {
    if durability.current > 0 {
        durability.current -= 1;
        true
    } else {
        false // Tool is broken
    }
}

/// Repair the tool by a certain amount
/// Function - transforms tool durability data by repairing
pub fn repair_tool_durability(durability: &mut ToolDurability, amount: u32) {
    durability.current = (durability.current + amount).min(durability.max);
}