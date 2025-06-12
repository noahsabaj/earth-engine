use super::entity_data::{EntityData, EntityId};
use super::component_data::{ComponentData, TransformData, PhysicsData, ItemData, RenderableData};
use super::component_data::{COMPONENT_TRANSFORM, COMPONENT_PHYSICS, COMPONENT_ITEM, COMPONENT_RENDERABLE};

/// Data-oriented ECS world
pub struct EcsWorldData {
    /// Entity management
    pub entities: EntityData,
    
    /// Component storage
    pub components: ComponentData,
}

impl EcsWorldData {
    pub fn new() -> Self {
        Self {
            entities: EntityData::new(),
            components: ComponentData::new(),
        }
    }
    
    /// Get component by type (compatibility method)
    pub fn get_component<T>(&self, entity: EntityId) -> Option<T> 
    where 
        T: 'static
    {
        use super::components::{Transform, Physics, Item, Renderable};
        use std::any::TypeId;
        
        let type_id = TypeId::of::<T>();
        
        if type_id == TypeId::of::<Transform>() {
            self.components.get_transform(entity).map(|t| {
                let transform = Transform {
                    position: cgmath::Vector3::new(t.position[0], t.position[1], t.position[2]),
                    rotation: cgmath::Vector3::new(t.rotation[0], t.rotation[1], t.rotation[2]),
                    scale: cgmath::Vector3::new(t.scale[0], t.scale[1], t.scale[2]),
                };
                // SAFETY: We've verified T is Transform
                unsafe { std::mem::transmute_copy(&transform) }
            })
        } else if type_id == TypeId::of::<Physics>() {
            self.components.get_physics(entity).map(|p| {
                let physics = Physics {
                    velocity: cgmath::Vector3::new(p.velocity[0], p.velocity[1], p.velocity[2]),
                    acceleration: cgmath::Vector3::new(p.acceleration[0], p.acceleration[1], p.acceleration[2]),
                    mass: p.mass,
                    grounded: p.flags & 1 != 0,
                };
                // SAFETY: We've verified T is Physics
                unsafe { std::mem::transmute_copy(&physics) }
            })
        } else if type_id == TypeId::of::<Item>() {
            self.components.get_item(entity).map(|i| {
                let item = Item {
                    item_id: i.item_id,
                    count: i.stack_size,
                    lifetime: i.pickup_delay,
                };
                // SAFETY: We've verified T is Item
                unsafe { std::mem::transmute_copy(&item) }
            })
        } else if type_id == TypeId::of::<Renderable>() {
            self.components.get_renderable(entity).map(|r| {
                let renderable = Renderable {
                    mesh_id: r.mesh_id,
                    material_id: r.material_id,
                    visible: r.visible != 0,
                };
                // SAFETY: We've verified T is Renderable
                unsafe { std::mem::transmute_copy(&renderable) }
            })
        } else {
            None
        }
    }
    
    /// Create a new entity
    pub fn create_entity(&mut self) -> EntityId {
        self.entities.create()
    }
    
    /// Destroy an entity and all its components
    pub fn destroy_entity(&mut self, entity: EntityId) -> bool {
        if self.entities.destroy(entity) {
            // Clear all components for this entity
            self.components.clear_entity(entity);
            true
        } else {
            false
        }
    }
    
    /// Check if entity exists and is alive
    pub fn is_entity_alive(&self, entity: EntityId) -> bool {
        self.entities.is_alive(entity)
    }
    
    /// Add transform component to entity
    pub fn add_transform(&mut self, entity: EntityId, position: [f32; 3], rotation: [f32; 3], scale: [f32; 3]) -> bool {
        if !self.is_entity_alive(entity) {
            return false;
        }
        
        let data = TransformData {
            position,
            rotation,
            scale,
        };
        
        if self.components.add_transform(entity, data) {
            self.entities.set_component_bit(entity, COMPONENT_TRANSFORM);
            true
        } else {
            false
        }
    }
    
    /// Add physics component to entity
    pub fn add_physics(&mut self, entity: EntityId, mass: f32, aabb_min: [f32; 3], aabb_max: [f32; 3]) -> bool {
        if !self.is_entity_alive(entity) {
            return false;
        }
        
        let data = PhysicsData {
            velocity: [0.0, 0.0, 0.0],
            acceleration: [0.0, 0.0, 0.0],
            angular_velocity: [0.0, 0.0, 0.0],
            mass,
            inverse_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            gravity_scale: 1.0,
            drag: 0.1,
            grounded: false,
            aabb_min,
            aabb_max,
        };
        
        if self.components.add_physics(entity, data) {
            self.entities.set_component_bit(entity, COMPONENT_PHYSICS);
            true
        } else {
            false
        }
    }
    
    /// Add item component to entity
    pub fn add_item(&mut self, entity: EntityId, item_id: u32, stack_size: u32) -> bool {
        if !self.is_entity_alive(entity) {
            return false;
        }
        
        let data = ItemData {
            item_id,
            stack_size,
            pickup_delay: 0.5,
            lifetime: 300.0,
        };
        
        if self.components.add_item(entity, data) {
            self.entities.set_component_bit(entity, COMPONENT_ITEM);
            true
        } else {
            false
        }
    }
    
    /// Remove transform component
    pub fn remove_transform(&mut self, entity: EntityId) -> bool {
        if self.components.remove_transform(entity) {
            self.entities.clear_component_bit(entity, COMPONENT_TRANSFORM);
            true
        } else {
            false
        }
    }
    
    /// Remove physics component
    pub fn remove_physics(&mut self, entity: EntityId) -> bool {
        if self.components.remove_physics(entity) {
            self.entities.clear_component_bit(entity, COMPONENT_PHYSICS);
            true
        } else {
            false
        }
    }
    
    /// Remove item component
    pub fn remove_item(&mut self, entity: EntityId) -> bool {
        if self.components.remove_item(entity) {
            self.entities.clear_component_bit(entity, COMPONENT_ITEM);
            true
        } else {
            false
        }
    }
    
    /// Query entities with specific components
    pub fn query_entities(&self, component_mask: u64) -> Vec<EntityId> {
        let mut result = Vec::new();
        
        for i in 0..self.entities.entity_count() {
            let meta = &self.entities.metas[i];
            if meta.alive && (meta.component_mask & component_mask) == component_mask {
                result.push(EntityId {
                    index: i as u32,
                    generation: meta.generation,
                });
            }
        }
        
        result
    }
    
    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.entities.entity_count()
    }
    
    /// Clear all data
    pub fn clear(&mut self) {
        self.entities.clear();
        self.components.clear();
    }
}

// Free functions for component access
pub fn get_transform(world: &EcsWorldData, entity: EntityId) -> Option<&TransformData> {
    world.components.get_transform(entity)
}

pub fn get_transform_mut(world: &mut EcsWorldData, entity: EntityId) -> Option<&mut TransformData> {
    world.components.get_transform_mut(entity)
}

pub fn get_physics(world: &EcsWorldData, entity: EntityId) -> Option<&PhysicsData> {
    world.components.get_physics(entity)
}

pub fn get_physics_mut(world: &mut EcsWorldData, entity: EntityId) -> Option<&mut PhysicsData> {
    world.components.get_physics_mut(entity)
}

pub fn has_transform(world: &EcsWorldData, entity: EntityId) -> bool {
    world.entities.has_component_bit(entity, COMPONENT_TRANSFORM)
}

pub fn has_physics(world: &EcsWorldData, entity: EntityId) -> bool {
    world.entities.has_component_bit(entity, COMPONENT_PHYSICS)
}

pub fn has_item(world: &EcsWorldData, entity: EntityId) -> bool {
    world.entities.has_component_bit(entity, COMPONENT_ITEM)
}

// Helper to create item entities
pub fn spawn_item(
    world: &mut EcsWorldData,
    position: [f32; 3],
    velocity: [f32; 3],
    item_id: u32,
    stack_size: u32,
) -> EntityId {
    let entity = world.create_entity();
    
    // Add transform
    world.add_transform(entity, position, [0.0, 0.0, 0.0], [0.5, 0.5, 0.5]);
    
    // Add physics with velocity
    if let Some(physics) = world.components.get_physics_mut(entity) {
        physics.velocity = velocity;
    }
    
    // Add item component
    world.add_item(entity, item_id, stack_size);
    
    entity
}