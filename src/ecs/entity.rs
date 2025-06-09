use std::collections::{HashSet, VecDeque};

/// Unique identifier for an entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(pub u32);

impl Entity {
    pub const INVALID: Entity = Entity(u32::MAX);
}

/// Manages entity creation and destruction
pub struct EntityManager {
    next_id: u32,
    active_entities: HashSet<Entity>,
    recycled_ids: VecDeque<u32>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            active_entities: HashSet::new(),
            recycled_ids: VecDeque::new(),
        }
    }
    
    /// Create a new entity
    pub fn create(&mut self) -> Entity {
        let id = if let Some(recycled_id) = self.recycled_ids.pop_front() {
            recycled_id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        };
        
        let entity = Entity(id);
        self.active_entities.insert(entity);
        entity
    }
    
    /// Destroy an entity
    pub fn destroy(&mut self, entity: Entity) -> bool {
        if self.active_entities.remove(&entity) {
            self.recycled_ids.push_back(entity.0);
            true
        } else {
            false
        }
    }
    
    /// Check if an entity exists
    pub fn exists(&self, entity: Entity) -> bool {
        self.active_entities.contains(&entity)
    }
    
    /// Get all active entities
    pub fn active_entities(&self) -> &HashSet<Entity> {
        &self.active_entities
    }
    
    /// Get the number of active entities
    pub fn count(&self) -> usize {
        self.active_entities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_entity_creation() {
        let mut manager = EntityManager::new();
        let e1 = manager.create();
        let e2 = manager.create();
        
        assert_ne!(e1, e2);
        assert!(manager.exists(e1));
        assert!(manager.exists(e2));
        assert_eq!(manager.count(), 2);
    }
    
    #[test]
    fn test_entity_destruction() {
        let mut manager = EntityManager::new();
        let entity = manager.create();
        
        assert!(manager.destroy(entity));
        assert!(!manager.exists(entity));
        assert_eq!(manager.count(), 0);
    }
    
    #[test]
    fn test_id_recycling() {
        let mut manager = EntityManager::new();
        let e1 = manager.create();
        let id1 = e1.0;
        
        manager.destroy(e1);
        let e2 = manager.create();
        
        assert_eq!(e2.0, id1); // ID should be recycled
    }
}