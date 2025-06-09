use std::collections::HashMap;
use std::any::Any;
use super::Entity;

/// Trait that all components must implement
pub trait Component: Any + Send + Sync {
    /// Get the component as Any for downcasting
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// Blanket implementation for all types that satisfy the bounds
impl<T: Any + Send + Sync> Component for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Storage for a specific component type
pub struct ComponentStorage<T: Component> {
    components: HashMap<Entity, T>,
}

impl<T: Component> ComponentStorage<T> {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
    
    /// Add a component to an entity
    pub fn insert(&mut self, entity: Entity, component: T) -> Option<T> {
        self.components.insert(entity, component)
    }
    
    /// Remove a component from an entity
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        self.components.remove(&entity)
    }
    
    /// Get a component for an entity
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.components.get(&entity)
    }
    
    /// Get a mutable component for an entity
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_mut(&entity)
    }
    
    /// Check if entity has this component
    pub fn contains(&self, entity: Entity) -> bool {
        self.components.contains_key(&entity)
    }
    
    /// Iterate over all entities with this component
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.components.iter().map(|(&e, c)| (e, c))
    }
    
    /// Iterate mutably over all entities with this component
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.components.iter_mut().map(|(&e, c)| (e, c))
    }
    
    /// Clear all components when entity is destroyed
    pub fn clear_entity(&mut self, entity: Entity) {
        self.components.remove(&entity);
    }
    
    /// Get all entities that have this component
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.components.keys()
    }
}

/// Type-erased component storage
pub trait AnyComponentStorage: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear_entity(&mut self, entity: Entity);
}

impl<T: Component + 'static> AnyComponentStorage for ComponentStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    
    fn clear_entity(&mut self, entity: Entity) {
        self.clear_entity(entity);
    }
}