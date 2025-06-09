use crate::ecs::{EcsWorld, Entity};
use crate::ecs::systems::item_physics::create_item_entity;
use crate::inventory::PlayerInventory;
use glam::Vec3;
use rand::Rng;

/// Default drop velocity forward
const DROP_FORWARD_VELOCITY: f32 = 4.0;
/// Random velocity range for drops
const DROP_RANDOM_VELOCITY: f32 = 1.0;

/// Handles item dropping from inventory
pub struct ItemDropHandler;

impl ItemDropHandler {
    /// Drop a single item from the selected hotbar slot
    pub fn drop_selected_item(
        inventory: &mut PlayerInventory,
        world: &mut EcsWorld,
        player_position: Vec3,
        player_forward: Vec3,
    ) -> Option<Entity> {
        // Remove one item from selected slot
        if let Some(item_stack) = inventory.remove_selected(1) {
            // Calculate drop position (slightly in front of player)
            let drop_position = player_position + player_forward * 0.5 + Vec3::Y * 0.5;
            
            // Calculate drop velocity
            let mut rng = rand::thread_rng();
            let random_offset = Vec3::new(
                rng.gen_range(-DROP_RANDOM_VELOCITY..DROP_RANDOM_VELOCITY),
                rng.gen_range(0.0..DROP_RANDOM_VELOCITY),
                rng.gen_range(-DROP_RANDOM_VELOCITY..DROP_RANDOM_VELOCITY),
            );
            let drop_velocity = player_forward * DROP_FORWARD_VELOCITY + random_offset;
            
            // Create item entity
            let entity = create_item_entity(
                world,
                drop_position,
                drop_velocity,
                item_stack.item_id,
                item_stack.count,
            );
            
            Some(entity)
        } else {
            None
        }
    }
    
    /// Drop the entire selected stack
    pub fn drop_selected_stack(
        inventory: &mut PlayerInventory,
        world: &mut EcsWorld,
        player_position: Vec3,
        player_forward: Vec3,
    ) -> Option<Entity> {
        // Get the full stack count
        let stack_count = inventory.get_selected_item().map(|item| item.count)?;
        
        // Remove the entire stack
        if let Some(item_stack) = inventory.remove_selected(stack_count) {
            // Calculate drop position
            let drop_position = player_position + player_forward * 0.5 + Vec3::Y * 0.5;
            
            // Calculate drop velocity
            let mut rng = rand::thread_rng();
            let random_offset = Vec3::new(
                rng.gen_range(-DROP_RANDOM_VELOCITY..DROP_RANDOM_VELOCITY),
                rng.gen_range(0.0..DROP_RANDOM_VELOCITY),
                rng.gen_range(-DROP_RANDOM_VELOCITY..DROP_RANDOM_VELOCITY),
            );
            let drop_velocity = player_forward * DROP_FORWARD_VELOCITY + random_offset;
            
            // Create item entity
            let entity = create_item_entity(
                world,
                drop_position,
                drop_velocity,
                item_stack.item_id,
                item_stack.count,
            );
            
            Some(entity)
        } else {
            None
        }
    }
    
    /// Drop items from a specific inventory slot
    pub fn drop_from_slot(
        inventory: &mut PlayerInventory,
        world: &mut EcsWorld,
        slot_index: usize,
        count: u32,
        player_position: Vec3,
        player_forward: Vec3,
    ) -> Option<Entity> {
        // Get the slot
        let slot = inventory.get_slot_mut(slot_index)?;
        let item = slot.get_item_mut()?;
        
        // Split the stack
        let dropped = item.split(count)?;
        slot.cleanup();
        
        // Calculate drop position
        let drop_position = player_position + player_forward * 0.5 + Vec3::Y * 0.5;
        
        // Calculate drop velocity
        let mut rng = rand::thread_rng();
        let random_offset = Vec3::new(
            rng.gen_range(-DROP_RANDOM_VELOCITY..DROP_RANDOM_VELOCITY),
            rng.gen_range(0.0..DROP_RANDOM_VELOCITY),
            rng.gen_range(-DROP_RANDOM_VELOCITY..DROP_RANDOM_VELOCITY),
        );
        let drop_velocity = player_forward * DROP_FORWARD_VELOCITY + random_offset;
        
        // Create item entity
        let entity = create_item_entity(
            world,
            drop_position,
            drop_velocity,
            dropped.item_id,
            dropped.count,
        );
        
        Some(entity)
    }
}