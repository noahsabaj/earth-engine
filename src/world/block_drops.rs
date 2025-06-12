use crate::world::{VoxelPos, BlockId, World};
use crate::item::ItemRegistry;
use crate::ecs::{EcsWorldData, spawn_dropped_item};
use crate::crafting::tool::{Tool, get_block_properties, BlockProperties};
use glam::Vec3;
use rand::Rng;

/// Handles dropping items when blocks are broken
pub struct BlockDropHandler;

impl BlockDropHandler {
    /// Handle a block being broken
    pub fn handle_block_break(
        world: &World,
        ecs_world: &mut EcsWorldData,
        item_registry: &ItemRegistry,
        pos: VoxelPos,
        block_id: BlockId,
        tool_used: Option<&Tool>,
    ) {
        // Get block properties
        let properties = get_block_properties(block_id);
        
        // Determine what items to drop
        let drops = Self::calculate_drops(&properties, tool_used);
        
        // Create item entities for each drop
        let world_pos = Vec3::new(
            pos.x as f32 + 0.5,
            pos.y as f32 + 0.5,
            pos.z as f32 + 0.5,
        );
        
        for drop in drops {
            // Convert block ID to item ID
            let item_id = if let Some(id) = item_registry.get_item_for_block(drop.block_id) {
                id
            } else {
                // If no item mapping exists, skip this drop
                continue;
            };
            
            // Create random velocity
            let mut rng = rand::thread_rng();
            let velocity = Vec3::new(
                rng.gen_range(-2.0..2.0),
                rng.gen_range(2.0..5.0),
                rng.gen_range(-2.0..2.0),
            );
            
            // Create the item entity
            spawn_dropped_item(
                ecs_world,
                [world_pos.x, world_pos.y, world_pos.z],
                [velocity.x, velocity.y, velocity.z],
                item_id.0, // Assuming ItemId wraps u32
                drop.count,
            );
        }
    }
    
    /// Calculate what items should drop based on block properties and tool
    fn calculate_drops(properties: &BlockProperties, tool_used: Option<&Tool>) -> Vec<BlockDrop> {
        let mut drops = Vec::new();
        
        for drop_def in &properties.drops {
            // Check if tool is required
            if drop_def.requires_tool {
                match tool_used {
                    Some(tool) => {
                        let effectiveness = tool.get_effectiveness(BlockId(0), properties); // BlockId doesn't matter here
                        if !effectiveness.can_harvest {
                            continue; // Tool can't harvest this
                        }
                    }
                    None => continue, // No tool but tool required
                }
            }
            
            // Calculate drop count (random between min and max)
            let mut rng = rand::thread_rng();
            let count = if drop_def.min_count == drop_def.max_count {
                drop_def.min_count
            } else {
                rng.gen_range(drop_def.min_count..=drop_def.max_count)
            };
            
            if count > 0 {
                drops.push(BlockDrop {
                    block_id: drop_def.block_id,
                    count,
                });
            }
        }
        
        drops
    }
    
    /// Calculate mining time for a block
    pub fn calculate_mining_time(
        block_id: BlockId,
        tool_used: Option<&Tool>,
    ) -> f32 {
        let properties = get_block_properties(block_id);
        
        // Base time is the block's hardness
        let base_time = properties.hardness;
        
        // Apply tool effectiveness
        if let Some(tool) = tool_used {
            let effectiveness = tool.get_effectiveness(block_id, &properties);
            base_time / effectiveness.speed_multiplier
        } else {
            // Mining by hand
            if properties.breakable_by_hand {
                base_time * 5.0 // 5x slower by hand
            } else {
                f32::INFINITY // Can't break by hand
            }
        }
    }
}

/// Represents an actual item drop
#[derive(Debug, Clone)]
struct BlockDrop {
    pub block_id: BlockId,
    pub count: u32,
}

/// Mining progress tracker
#[derive(Debug, Clone)]
pub struct MiningProgress {
    pub block_pos: VoxelPos,
    pub progress: f32, // 0.0 to 1.0
    pub total_time: f32,
    pub elapsed_time: f32,
}

impl MiningProgress {
    pub fn new(block_pos: VoxelPos, total_time: f32) -> Self {
        Self {
            block_pos,
            progress: 0.0,
            total_time,
            elapsed_time: 0.0,
        }
    }
    
    /// Update mining progress
    pub fn update(&mut self, delta_time: f32) -> bool {
        self.elapsed_time += delta_time;
        self.progress = (self.elapsed_time / self.total_time).min(1.0);
        self.progress >= 1.0
    }
    
    /// Reset progress
    pub fn reset(&mut self) {
        self.progress = 0.0;
        self.elapsed_time = 0.0;
    }
}