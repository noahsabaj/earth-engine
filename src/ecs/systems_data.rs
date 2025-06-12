use super::world_data::EcsWorldData;
use super::entity_data::EntityId;
use super::component_data::{COMPONENT_TRANSFORM, COMPONENT_PHYSICS, COMPONENT_ITEM};

/// Gravity constant for items
const ITEM_GRAVITY: f32 = 20.0;
/// Drag coefficient for items
const ITEM_DRAG: f32 = 0.98;
/// Size of item collision box
const ITEM_SIZE: f32 = 0.25;

/// Update physics for all entities with physics components
pub fn update_physics_system(world: &mut EcsWorldData, delta_time: f32) {
    // Process all physics entities
    for i in 0..world.components.physics_count {
        let entity = world.components.physics_entities[i];
        
        if !world.is_entity_alive(entity) {
            continue;
        }
        
        let physics = &mut world.components.physics[i];
        
        // Apply gravity if enabled
        if physics.gravity_scale > 0.0 {
            physics.velocity[1] -= ITEM_GRAVITY * physics.gravity_scale * delta_time;
        }
        
        // Apply drag
        physics.velocity[0] *= physics.drag;
        physics.velocity[1] *= physics.drag;
        physics.velocity[2] *= physics.drag;
        
        // Stop very slow movement
        let vel_magnitude = (physics.velocity[0] * physics.velocity[0] + 
                           physics.velocity[1] * physics.velocity[1] + 
                           physics.velocity[2] * physics.velocity[2]).sqrt();
        
        if vel_magnitude < 0.01 {
            physics.velocity = [0.0, 0.0, 0.0];
        }
        
        // Apply velocity to acceleration
        physics.velocity[0] += physics.acceleration[0] * delta_time;
        physics.velocity[1] += physics.acceleration[1] * delta_time;
        physics.velocity[2] += physics.acceleration[2] * delta_time;
    }
}

/// Update transforms based on physics
pub fn update_transform_from_physics(world: &mut EcsWorldData, delta_time: f32) {
    // Process entities that have both transform and physics
    let mask = (1u64 << COMPONENT_TRANSFORM) | (1u64 << COMPONENT_PHYSICS);
    
    for i in 0..world.entities.entity_count() {
        let meta = &world.entities.metas[i];
        if !meta.alive || (meta.component_mask & mask) != mask {
            continue;
        }
        
        let entity = EntityId {
            index: i as u32,
            generation: meta.generation,
        };
        
        // Get physics velocity
        let velocity = match world.components.get_physics(entity) {
            Some(physics) => physics.velocity,
            None => continue,
        };
        
        // Update transform position
        if let Some(transform) = world.components.get_transform_mut(entity) {
            transform.position[0] += velocity[0] * delta_time;
            transform.position[1] += velocity[1] * delta_time;
            transform.position[2] += velocity[2] * delta_time;
            
            // Simple ground collision (y = 0)
            if transform.position[1] < ITEM_SIZE / 2.0 {
                transform.position[1] = ITEM_SIZE / 2.0;
                
                // Stop downward velocity
                if let Some(physics) = world.components.get_physics_mut(entity) {
                    if physics.velocity[1] < 0.0 {
                        physics.velocity[1] = 0.0;
                        physics.grounded = true;
                    }
                }
            }
        }
    }
}

/// Update item lifetimes and remove expired items
pub fn update_item_lifetimes(world: &mut EcsWorldData, delta_time: f32) -> Vec<EntityId> {
    let mut expired_items = Vec::new();
    
    // Update lifetimes
    for i in 0..world.components.items_count {
        let entity = world.components.items_entities[i];
        
        if !world.is_entity_alive(entity) {
            continue;
        }
        
        let item = &mut world.components.items[i];
        item.lifetime -= delta_time;
        
        if item.pickup_delay > 0.0 {
            item.pickup_delay -= delta_time;
        }
        
        if item.lifetime <= 0.0 {
            expired_items.push(entity);
        }
    }
    
    // Remove expired items
    for entity in &expired_items {
        world.destroy_entity(*entity);
    }
    
    expired_items
}

/// Complete item physics system update
pub fn update_item_physics_system(world: &mut EcsWorldData, delta_time: f32) {
    // Update physics simulation
    update_physics_system(world, delta_time);
    
    // Update transforms from physics
    update_transform_from_physics(world, delta_time);
    
    // Update item lifetimes and remove expired
    update_item_lifetimes(world, delta_time);
}

/// Check for item pickups near a position
pub fn check_item_pickups(
    world: &EcsWorldData,
    position: [f32; 3],
    pickup_radius: f32,
) -> Vec<(EntityId, u32, u32)> { // Returns (entity, item_id, stack_size)
    let mut pickups = Vec::new();
    let radius_sq = pickup_radius * pickup_radius;
    
    // Check all items
    for i in 0..world.components.items_count {
        let entity = world.components.items_entities[i];
        
        if !world.is_entity_alive(entity) {
            continue;
        }
        
        let item = &world.components.items[i];
        
        // Check if pickup delay has expired
        if item.pickup_delay > 0.0 {
            continue;
        }
        
        // Check distance
        if let Some(transform) = world.components.get_transform(entity) {
            let dx = transform.position[0] - position[0];
            let dy = transform.position[1] - position[1];
            let dz = transform.position[2] - position[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            
            if dist_sq <= radius_sq {
                pickups.push((entity, item.item_id, item.stack_size));
            }
        }
    }
    
    pickups
}

/// Spawn a dropped item in the world
pub fn spawn_dropped_item(
    world: &mut EcsWorldData,
    position: [f32; 3],
    velocity: [f32; 3],
    item_id: u32,
    stack_size: u32,
) -> EntityId {
    let entity = world.create_entity();
    
    // Add transform (items are half size)
    world.add_transform(entity, position, [0.0, 0.0, 0.0], [0.5, 0.5, 0.5]);
    
    // Add physics
    let half_size = ITEM_SIZE / 2.0;
    world.add_physics(entity, 1.0, [-half_size, -half_size, -half_size], [half_size, half_size, half_size]);
    
    // Set initial velocity
    if let Some(physics) = world.components.get_physics_mut(entity) {
        physics.velocity = velocity;
        physics.drag = ITEM_DRAG;
    }
    
    // Add item component
    world.add_item(entity, item_id, stack_size);
    
    entity
}

/// Apply impulse to physics entity
pub fn apply_impulse(world: &mut EcsWorldData, entity: EntityId, impulse: [f32; 3]) {
    if let Some(physics) = world.components.get_physics_mut(entity) {
        if physics.inverse_mass > 0.0 {
            physics.velocity[0] += impulse[0] * physics.inverse_mass;
            physics.velocity[1] += impulse[1] * physics.inverse_mass;
            physics.velocity[2] += impulse[2] * physics.inverse_mass;
        }
    }
}

/// Set velocity directly
pub fn set_velocity(world: &mut EcsWorldData, entity: EntityId, velocity: [f32; 3]) {
    if let Some(physics) = world.components.get_physics_mut(entity) {
        physics.velocity = velocity;
    }
}

/// Get entity position
pub fn get_position(world: &EcsWorldData, entity: EntityId) -> Option<[f32; 3]> {
    world.components.get_transform(entity).map(|t| t.position)
}

/// Get entity velocity
pub fn get_velocity(world: &EcsWorldData, entity: EntityId) -> Option<[f32; 3]> {
    world.components.get_physics(entity).map(|p| p.velocity)
}