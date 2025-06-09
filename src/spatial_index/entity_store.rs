use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Types of entities that can be spatially indexed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Mob,
    Item,
    Projectile,
    Vehicle,
    Structure,
    Particle,
    Custom(u32),
}

impl EntityType {
    /// Convert the entity type to a unique discriminant for hashing
    pub fn to_discriminant(&self) -> u64 {
        match self {
            EntityType::Player => 0,
            EntityType::Mob => 1,
            EntityType::Item => 2,
            EntityType::Projectile => 3,
            EntityType::Vehicle => 4,
            EntityType::Structure => 5,
            EntityType::Particle => 6,
            EntityType::Custom(id) => 7 + (*id as u64),
        }
    }
}

/// Data associated with a spatial entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    pub entity_type: EntityType,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub radius: f32,
    pub metadata: HashMap<String, String>,
}

/// A spatial entity that can be indexed
#[derive(Debug, Clone)]
pub struct SpatialEntity {
    id: u64,
    data: EntityData,
}

impl SpatialEntity {
    pub fn new(id: u64, entity_type: EntityType, position: [f32; 3], radius: f32) -> Self {
        Self {
            id,
            data: EntityData {
                entity_type,
                position,
                velocity: [0.0, 0.0, 0.0],
                radius,
                metadata: HashMap::new(),
            },
        }
    }
    
    pub fn id(&self) -> u64 {
        self.id
    }
    
    pub fn entity_type(&self) -> EntityType {
        self.data.entity_type
    }
    
    pub fn position(&self) -> [f32; 3] {
        self.data.position
    }
    
    pub fn set_position(&mut self, position: [f32; 3]) {
        self.data.position = position;
    }
    
    pub fn velocity(&self) -> [f32; 3] {
        self.data.velocity
    }
    
    pub fn set_velocity(&mut self, velocity: [f32; 3]) {
        self.data.velocity = velocity;
    }
    
    pub fn radius(&self) -> f32 {
        self.data.radius
    }
    
    pub fn data(&self) -> &EntityData {
        &self.data
    }
    
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.data.metadata
    }
    
    pub fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.data.metadata
    }
}

/// Thread-safe storage for entities
pub struct EntityStore {
    entities: RwLock<HashMap<u64, SpatialEntity>>,
    by_type: RwLock<HashMap<EntityType, Vec<u64>>>,
}

impl EntityStore {
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(HashMap::new()),
            by_type: RwLock::new(HashMap::new()),
        }
    }
    
    pub fn insert(&self, id: u64, entity: SpatialEntity) {
        let entity_type = entity.entity_type();
        
        // Insert entity
        self.entities.write().insert(id, entity);
        
        // Update type index
        self.by_type.write()
            .entry(entity_type)
            .or_insert_with(Vec::new)
            .push(id);
    }
    
    pub fn remove(&self, id: u64) -> Option<SpatialEntity> {
        // Remove entity
        let entity = self.entities.write().remove(&id)?;
        
        // Update type index
        let mut by_type = self.by_type.write();
        if let Some(type_list) = by_type.get_mut(&entity.entity_type()) {
            type_list.retain(|&x| x != id);
        }
        
        Some(entity)
    }
    
    pub fn get(&self, id: u64) -> Option<SpatialEntity> {
        self.entities.read().get(&id).cloned()
    }
    
    pub fn get_mut(&self, id: u64) -> Option<EntityRefMut> {
        // This is a bit tricky - we need to ensure thread safety
        // For now, return a clone that can be updated
        self.get(id).map(|entity| EntityRefMut {
            store: self,
            entity,
        })
    }
    
    pub fn count(&self) -> usize {
        self.entities.read().len()
    }
    
    pub fn count_by_type(&self, entity_type: EntityType) -> usize {
        self.by_type.read()
            .get(&entity_type)
            .map(|v| v.len())
            .unwrap_or(0)
    }
    
    pub fn get_all_of_type(&self, entity_type: EntityType) -> Vec<u64> {
        self.by_type.read()
            .get(&entity_type)
            .cloned()
            .unwrap_or_default()
    }
    
    pub fn iter(&self) -> Vec<(u64, SpatialEntity)> {
        self.entities.read()
            .iter()
            .map(|(&id, entity)| (id, entity.clone()))
            .collect()
    }
}

/// Mutable reference to an entity that updates the store when dropped
pub struct EntityRefMut<'a> {
    store: &'a EntityStore,
    entity: SpatialEntity,
}

impl<'a> EntityRefMut<'a> {
    pub fn position(&self) -> [f32; 3] {
        self.entity.position()
    }
    
    pub fn set_position(&mut self, position: [f32; 3]) {
        self.entity.set_position(position);
    }
    
    pub fn velocity(&self) -> [f32; 3] {
        self.entity.velocity()
    }
    
    pub fn set_velocity(&mut self, velocity: [f32; 3]) {
        self.entity.set_velocity(velocity);
    }
    
    pub fn radius(&self) -> f32 {
        self.entity.radius()
    }
    
    pub fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        self.entity.metadata_mut()
    }
}

impl<'a> Drop for EntityRefMut<'a> {
    fn drop(&mut self) {
        // Update the entity in the store
        self.store.entities.write().insert(self.entity.id, self.entity.clone());
    }
}