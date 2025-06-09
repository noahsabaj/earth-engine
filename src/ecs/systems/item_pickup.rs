use crate::ecs::{System, EcsWorld, Entity};
use crate::ecs::components::{Transform, ItemComponent};
use crate::inventory::{PlayerInventory, ItemStack};
use glam::Vec3;

/// Distance at which items can be picked up
const PICKUP_DISTANCE: f32 = 1.5;

/// System that handles item pickup by the player
pub struct ItemPickupSystem {
    player_entity: Entity,
}

impl ItemPickupSystem {
    pub fn new(player_entity: Entity) -> Self {
        Self { player_entity }
    }
    
    /// Check if player can pick up an item
    fn can_pickup(&self, player_pos: Vec3, item_pos: Vec3, item: &ItemComponent) -> bool {
        // Check pickup delay
        if item.pickup_delay > 0.0 {
            return false;
        }
        
        // Check distance
        let distance = player_pos.distance(item_pos);
        distance <= PICKUP_DISTANCE
    }
}

impl System for ItemPickupSystem {
    fn update(&mut self, world: &mut EcsWorld, delta_time: f32) {
        // Get player position
        let player_transform = match world.get_component::<Transform>(self.player_entity) {
            Some(transform) => transform.clone(),
            None => return,
        };
        
        // Get player inventory (for now, we'll assume it's stored elsewhere)
        // In a real implementation, this would be a component or resource
        let mut items_to_pickup = Vec::new();
        
        // Find all item entities within pickup range
        let entities: Vec<Entity> = world.get_entities_with_component::<ItemComponent>();
        
        for entity in entities {
            let item_transform = match world.get_component::<Transform>(entity) {
                Some(transform) => transform,
                None => continue,
            };
            
            let item_component = match world.get_component::<ItemComponent>(entity) {
                Some(item) => item,
                None => continue,
            };
            
            let player_pos = Vec3::new(player_transform.position.x, player_transform.position.y, player_transform.position.z);
            let item_pos = Vec3::new(item_transform.position.x, item_transform.position.y, item_transform.position.z);
            if self.can_pickup(player_pos, item_pos, item_component) {
                items_to_pickup.push((entity, item_component.item_id, item_component.stack_size));
            }
        }
        
        // Process pickups (done separately to avoid borrowing issues)
        for (entity, item_id, count) in items_to_pickup {
            // In a real implementation, we'd add to player inventory here
            // For now, just remove the entity
            world.remove_entity(entity);
            
            // Log pickup for debugging
            println!("Picked up {:?} x{}", item_id, count);
        }
        
        // Update pickup delays for remaining items
        let entities: Vec<Entity> = world.get_entities_with_component::<ItemComponent>();
        for entity in entities {
            if let Some(item) = world.get_component_mut::<ItemComponent>(entity) {
                if item.pickup_delay > 0.0 {
                    item.pickup_delay = (item.pickup_delay - delta_time).max(0.0);
                }
            }
        }
    }
}

/// System that manages player inventory and item pickup
pub struct InventorySystem {
    player_entity: Entity,
    player_inventory: PlayerInventory,
}

impl InventorySystem {
    pub fn new(player_entity: Entity) -> Self {
        Self {
            player_entity,
            player_inventory: PlayerInventory::new(),
        }
    }
    
    pub fn get_inventory(&self) -> &PlayerInventory {
        &self.player_inventory
    }
    
    pub fn get_inventory_mut(&mut self) -> &mut PlayerInventory {
        &mut self.player_inventory
    }
    
    /// Try to pick up an item
    pub fn try_pickup(&mut self, item_id: crate::item::ItemId, count: u32) -> bool {
        let item_stack = ItemStack::new(item_id, count);
        match self.player_inventory.add_item(item_stack) {
            Some(remaining) => {
                // Some items couldn't be picked up
                if remaining.count < count {
                    // Partial pickup
                    println!("Picked up {:?} x{}, {} left", item_id, count - remaining.count, remaining.count);
                    true
                } else {
                    // No room
                    println!("No room in inventory for {:?} x{}", item_id, count);
                    false
                }
            }
            None => {
                // All items picked up
                println!("Picked up {:?} x{}", item_id, count);
                true
            }
        }
    }
}