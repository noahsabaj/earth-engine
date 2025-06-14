use super::world_data::EcsWorldData;
use super::entity_data::EntityId;
use super::component_data::{COMPONENT_TRANSFORM, COMPONENT_PHYSICS};
use crate::physics_data::{PhysicsData, PhysicsIntegrator, WorldInterface};

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

/// New integrated physics system using physics_data
pub struct IntegratedPhysicsSystem {
    physics_data: PhysicsData,
    integrator: PhysicsIntegrator,
    entity_mapping: rustc_hash::FxHashMap<EntityId, crate::physics_data::EntityId>,
}

impl IntegratedPhysicsSystem {
    pub fn new() -> Self {
        Self {
            physics_data: PhysicsData::new(4096), // Support up to 4096 physics entities
            integrator: PhysicsIntegrator::new(4096),
            entity_mapping: rustc_hash::FxHashMap::default(),
        }
    }
    
    /// Add ECS entity to physics system
    pub fn add_physics_entity(
        &mut self,
        ecs_entity: EntityId,
        position: [f32; 3],
        velocity: [f32; 3],
        mass: f32,
        half_extents: [f32; 3],
    ) -> Result<(), &'static str> {
        let physics_entity = self.physics_data.add_entity(position, velocity, mass, half_extents);
        self.entity_mapping.insert(ecs_entity, physics_entity);
        Ok(())
    }
    
    /// Remove ECS entity from physics system
    pub fn remove_physics_entity(&mut self, ecs_entity: EntityId) {
        if let Some(physics_entity) = self.entity_mapping.remove(&ecs_entity) {
            self.physics_data.remove_entity(physics_entity);
        }
    }
    
    /// Update physics system with world collision
    pub fn update_with_world<W: WorldInterface>(&mut self, world: &W, delta_time: f32) {
        self.integrator.integrate_with_world(&mut self.physics_data, world, delta_time);
    }
    
    /// Get position for rendering with interpolation
    pub fn get_interpolated_position(&self, ecs_entity: EntityId) -> Option<[f32; 3]> {
        let physics_entity = self.entity_mapping.get(&ecs_entity)?;
        self.integrator.get_interpolated_position(*physics_entity, &self.physics_data)
    }
    
    /// Set velocity for an entity
    pub fn set_entity_velocity(&mut self, ecs_entity: EntityId, velocity: [f32; 3]) {
        if let Some(&physics_entity) = self.entity_mapping.get(&ecs_entity) {
            PhysicsIntegrator::set_velocity(&mut self.physics_data, physics_entity, velocity);
        }
    }
    
    /// Apply impulse to an entity
    pub fn apply_entity_impulse(&mut self, ecs_entity: EntityId, impulse: [f32; 3]) {
        if let Some(&physics_entity) = self.entity_mapping.get(&ecs_entity) {
            let impulses = vec![(physics_entity, impulse)];
            PhysicsIntegrator::apply_impulses(&mut self.physics_data, &impulses);
        }
    }
    
    /// Get position from physics system
    pub fn get_entity_position(&self, ecs_entity: EntityId) -> Option<[f32; 3]> {
        let physics_entity = self.entity_mapping.get(&ecs_entity)?;
        let idx = physics_entity.index();
        self.physics_data.positions.get(idx).copied()
    }
    
    /// Get velocity from physics system
    pub fn get_entity_velocity(&self, ecs_entity: EntityId) -> Option<[f32; 3]> {
        let physics_entity = self.entity_mapping.get(&ecs_entity)?;
        let idx = physics_entity.index();
        self.physics_data.velocities.get(idx).copied()
    }
}

/// Update ECS world with integrated physics system
pub fn update_integrated_physics_system<W: WorldInterface>(
    ecs_world: &mut EcsWorldData,
    physics_system: &mut IntegratedPhysicsSystem,
    world: &W,
    delta_time: f32,
) {
    // Update physics simulation
    physics_system.update_with_world(world, delta_time);
    
    // Sync physics results back to ECS transform components
    sync_physics_to_transforms(ecs_world, physics_system);
    
    // Handle item lifetimes
    let expired_items = update_item_lifetimes(ecs_world, delta_time);
    
    // Remove expired items from physics system
    for entity in expired_items {
        physics_system.remove_physics_entity(entity);
    }
}

/// Sync physics positions back to ECS transform components
fn sync_physics_to_transforms(
    ecs_world: &mut EcsWorldData,
    physics_system: &IntegratedPhysicsSystem,
) {
    // Process entities that have both transform and physics
    let mask = (1u64 << COMPONENT_TRANSFORM) | (1u64 << COMPONENT_PHYSICS);
    
    for i in 0..ecs_world.entities.entity_count() {
        let meta = &ecs_world.entities.metas[i];
        if !meta.alive || (meta.component_mask & mask) != mask {
            continue;
        }
        
        let entity = EntityId {
            index: i as u32,
            generation: meta.generation,
        };
        
        // Get interpolated position from physics system
        if let Some(position) = physics_system.get_interpolated_position(entity) {
            // Update ECS transform
            if let Some(transform) = ecs_world.components.get_transform_mut(entity) {
                transform.position = position;
            }
        }
    }
}

/// Input processing with proper timing for physics integration
pub fn process_movement_input<W: WorldInterface>(
    ecs_world: &mut EcsWorldData,
    physics_system: &mut IntegratedPhysicsSystem,
    world: &W,
    input_state: &crate::input::InputState,
    player_entity: EntityId,
    delta_time: f32,
) {
    use crate::input::KeyCode;
    
    // Calculate movement direction from input
    let mut movement = [0.0f32, 0.0f32, 0.0f32];
    let movement_speed = 4.0f32; // 4 m/s
    
    if input_state.is_key_pressed(KeyCode::KeyW) {
        movement[2] -= 1.0;
    }
    if input_state.is_key_pressed(KeyCode::KeyS) {
        movement[2] += 1.0;
    }
    if input_state.is_key_pressed(KeyCode::KeyA) {
        movement[0] -= 1.0;
    }
    if input_state.is_key_pressed(KeyCode::KeyD) {
        movement[0] += 1.0;
    }
    
    // Normalize horizontal movement
    let horizontal_magnitude = (movement[0] * movement[0] + movement[2] * movement[2]).sqrt();
    if horizontal_magnitude > 0.0f32 {
        movement[0] = (movement[0] / horizontal_magnitude) * movement_speed;
        movement[2] = (movement[2] / horizontal_magnitude) * movement_speed;
    }
    
    // Handle jumping
    if input_state.is_key_pressed(KeyCode::Space) {
        // Check if player is grounded (this would need to be tracked)
        movement[1] = 8.0f32; // Jump velocity
    }
    
    // Apply movement to physics system
    physics_system.set_entity_velocity(player_entity, movement);
}