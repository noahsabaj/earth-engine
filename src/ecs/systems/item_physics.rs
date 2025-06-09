use crate::ecs::{System, EcsWorld, Entity};
use crate::ecs::components::{Transform, ItemComponent, Physics};
use crate::physics::AABB;
use glam::Vec3;
use cgmath::{Point3, Vector3, InnerSpace};

/// Gravity constant for items
const ITEM_GRAVITY: f32 = 20.0;
/// Drag coefficient for items
const ITEM_DRAG: f32 = 0.98;
/// Size of item collision box
const ITEM_SIZE: f32 = 0.25;

/// System that updates physics for item entities
pub struct ItemPhysicsSystem;

impl ItemPhysicsSystem {
    pub fn new() -> Self {
        Self
    }
}

impl System for ItemPhysicsSystem {
    fn update(&mut self, world: &mut EcsWorld, delta_time: f32) {
        // Get all entities with both ItemComponent and Physics
        let entities: Vec<Entity> = world.get_entities_with_component::<ItemComponent>();
        
        for entity in entities {
            // Update physics
            if let Some(physics) = world.get_component_mut::<Physics>(entity) {
                // Apply gravity
                physics.velocity.y -= ITEM_GRAVITY * delta_time;
                
                // Apply drag
                physics.velocity.x *= ITEM_DRAG;
                physics.velocity.y *= ITEM_DRAG;
                physics.velocity.z *= ITEM_DRAG;
                
                // Stop very slow movement
                if physics.velocity.magnitude() < 0.01 {
                    physics.velocity = Vector3::new(0.0, 0.0, 0.0);
                }
            }
            
            // Update transform based on physics
            let velocity = match world.get_component::<Physics>(entity) {
                Some(physics) => physics.velocity,
                None => continue,
            };
            
            if let Some(transform) = world.get_component_mut::<Transform>(entity) {
                transform.position.x += velocity.x * delta_time;
                transform.position.y += velocity.y * delta_time;
                transform.position.z += velocity.z * delta_time;
                
                // Simple ground collision (y = 0)
                if transform.position.y < ITEM_SIZE / 2.0 {
                    transform.position.y = ITEM_SIZE / 2.0;
                    
                    // Stop downward velocity
                    if let Some(physics) = world.get_component_mut::<Physics>(entity) {
                        if physics.velocity.y < 0.0 {
                            physics.velocity.y = 0.0;
                        }
                    }
                }
            }
            
            // Update item lifetime
            if let Some(item) = world.get_component_mut::<ItemComponent>(entity) {
                item.lifetime -= delta_time;
                
                // Mark for removal if lifetime expired
                if item.lifetime <= 0.0 {
                    // In a real implementation, we'd queue this for removal
                    // to avoid borrowing issues
                }
            }
        }
        
        // Remove expired items (done separately to avoid borrowing issues)
        let mut expired_items = Vec::new();
        let entities_check: Vec<Entity> = world.get_entities_with_component::<ItemComponent>();
        for entity in entities_check {
            if let Some(item) = world.get_component::<ItemComponent>(entity) {
                if item.lifetime <= 0.0 {
                    expired_items.push(entity);
                }
            }
        }
        
        for entity in expired_items {
            world.remove_entity(entity);
        }
    }
}

/// Create a dropped item entity
pub fn create_item_entity(
    world: &mut EcsWorld,
    position: Vec3,
    velocity: Vec3,
    item_id: crate::item::ItemId,
    stack_size: u32,
) -> Entity {
    let entity = world.create_entity();
    
    // Add transform
    world.add_component(entity, Transform {
        position: cgmath::Point3::new(position.x, position.y, position.z),
        rotation: cgmath::Vector3::new(0.0, 0.0, 0.0),
        scale: cgmath::Vector3::new(0.5, 0.5, 0.5), // Items are half size
    });
    
    // Add physics
    world.add_component(entity, Physics {
        velocity: cgmath::Vector3::new(velocity.x, velocity.y, velocity.z),
        acceleration: cgmath::Vector3::new(0.0, 0.0, 0.0),
        mass: 1.0,
        gravity_scale: 1.0,
        drag: 0.98,
        angular_velocity: cgmath::Vector3::new(0.0, 0.0, 0.0),
        bounding_box: AABB {
            min: cgmath::Point3::new(-ITEM_SIZE/2.0, -ITEM_SIZE/2.0, -ITEM_SIZE/2.0),
            max: cgmath::Point3::new(ITEM_SIZE/2.0, ITEM_SIZE/2.0, ITEM_SIZE/2.0),
        },
        grounded: false,
    });
    
    // Add item component
    world.add_component(entity, ItemComponent {
        item_id,
        stack_size,
        pickup_delay: 1.0, // 1 second delay before pickup
        lifetime: 300.0, // 5 minutes lifetime
    });
    
    entity
}