use std::collections::HashMap;
use std::any::TypeId;
use super::{Entity, EntityManager, Component, ComponentStorage, AnyComponentStorage};

/// The ECS world that manages entities and components
pub struct EcsWorld {
    entity_manager: EntityManager,
    component_storages: HashMap<TypeId, Box<dyn AnyComponentStorage>>,
}

impl EcsWorld {
    pub fn new() -> Self {
        Self {
            entity_manager: EntityManager::new(),
            component_storages: HashMap::new(),
        }
    }
    
    /// Create a new entity
    pub fn create_entity(&mut self) -> Entity {
        self.entity_manager.create()
    }
    
    /// Destroy an entity and all its components
    pub fn destroy_entity(&mut self, entity: Entity) -> bool {
        if self.entity_manager.destroy(entity) {
            // Remove all components for this entity
            for storage in self.component_storages.values_mut() {
                storage.clear_entity(entity);
            }
            true
        } else {
            false
        }
    }
    
    /// Check if entity exists
    pub fn entity_exists(&self, entity: Entity) -> bool {
        self.entity_manager.exists(entity)
    }
    
    /// Register a component type (must be called before using the component)
    pub fn register_component<T: Component + 'static>(&mut self) {
        let type_id = TypeId::of::<T>();
        if !self.component_storages.contains_key(&type_id) {
            self.component_storages.insert(
                type_id,
                Box::new(ComponentStorage::<T>::new()),
            );
        }
    }
    
    /// Add a component to an entity
    pub fn add_component<T: Component + 'static>(&mut self, entity: Entity, component: T) -> Result<(), &'static str> {
        if !self.entity_manager.exists(entity) {
            return Err("Entity does not exist");
        }
        
        let type_id = TypeId::of::<T>();
        let storage = self.component_storages
            .get_mut(&type_id)
            .ok_or("Component type not registered")?;
            
        if let Some(storage) = storage.as_any_mut().downcast_mut::<ComponentStorage<T>>() {
            storage.insert(entity, component);
            Ok(())
        } else {
            Err("Failed to downcast component storage")
        }
    }
    
    /// Remove a component from an entity
    pub fn remove_component<T: Component + 'static>(&mut self, entity: Entity) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let storage = self.component_storages.get_mut(&type_id)?;
        
        if let Some(storage) = storage.as_any_mut().downcast_mut::<ComponentStorage<T>>() {
            storage.remove(entity)
        } else {
            None
        }
    }
    
    /// Get a component for an entity
    pub fn get_component<T: Component + 'static>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let storage = self.component_storages.get(&type_id)?;
        
        if let Some(storage) = storage.as_any().downcast_ref::<ComponentStorage<T>>() {
            storage.get(entity)
        } else {
            None
        }
    }
    
    /// Get a mutable component for an entity
    pub fn get_component_mut<T: Component + 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        let storage = self.component_storages.get_mut(&type_id)?;
        
        if let Some(storage) = storage.as_any_mut().downcast_mut::<ComponentStorage<T>>() {
            storage.get_mut(entity)
        } else {
            None
        }
    }
    
    /// Check if entity has a component
    pub fn has_component<T: Component + 'static>(&self, entity: Entity) -> bool {
        self.get_component::<T>(entity).is_some()
    }
    
    /// Get component storage for iteration
    pub fn get_storage<T: Component + 'static>(&self) -> Option<&ComponentStorage<T>> {
        let type_id = TypeId::of::<T>();
        let storage = self.component_storages.get(&type_id)?;
        storage.as_any().downcast_ref::<ComponentStorage<T>>()
    }
    
    /// Get mutable component storage for iteration
    pub fn get_storage_mut<T: Component + 'static>(&mut self) -> Option<&mut ComponentStorage<T>> {
        let type_id = TypeId::of::<T>();
        let storage = self.component_storages.get_mut(&type_id)?;
        storage.as_any_mut().downcast_mut::<ComponentStorage<T>>()
    }
    
    /// Create an entity with components
    pub fn spawn(&mut self) -> EntityBuilder {
        let entity = self.create_entity();
        EntityBuilder {
            world: self,
            entity,
        }
    }
    
    /// Get all active entities
    pub fn entities(&self) -> &std::collections::HashSet<Entity> {
        self.entity_manager.active_entities()
    }
    
    /// Remove an entity (alias for destroy_entity)
    pub fn remove_entity(&mut self, entity: Entity) -> bool {
        self.destroy_entity(entity)
    }
    
    /// Get all entities that have a specific component
    pub fn get_entities_with_component<T: Component + 'static>(&self) -> Vec<Entity> {
        let type_id = TypeId::of::<T>();
        if let Some(storage) = self.component_storages.get(&type_id) {
            if let Some(storage) = storage.as_any().downcast_ref::<ComponentStorage<T>>() {
                storage.entities().cloned().collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }
}

/// Builder for creating entities with components
pub struct EntityBuilder<'a> {
    world: &'a mut EcsWorld,
    entity: Entity,
}

impl<'a> EntityBuilder<'a> {
    /// Add a component to the entity being built
    pub fn with<T: Component + 'static>(self, component: T) -> Self {
        self.world.add_component(self.entity, component).ok();
        self
    }
    
    /// Finish building and return the entity
    pub fn build(self) -> Entity {
        self.entity
    }
}