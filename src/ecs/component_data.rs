use super::entity_data::{EntityId, MAX_ENTITIES};
use cgmath::{Vector3, Point3};
use crate::{BlockId, AABB};
use crate::item::ItemId;

/// Component type IDs as constants
pub const COMPONENT_TRANSFORM: u8 = 0;
pub const COMPONENT_PHYSICS: u8 = 1;
pub const COMPONENT_ITEM: u8 = 2;
pub const COMPONENT_RENDERABLE: u8 = 3;

/// Transform component data (POD)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TransformData {
    pub position: [f32; 3],
    pub rotation: [f32; 3], // Euler angles
    pub scale: [f32; 3],
}

impl Default for TransformData {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// Physics component data (POD)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PhysicsData {
    pub velocity: [f32; 3],
    pub acceleration: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub mass: f32,
    pub inverse_mass: f32,
    pub gravity_scale: f32,
    pub drag: f32,
    pub grounded: bool,
    // AABB stored inline
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
}

impl Default for PhysicsData {
    fn default() -> Self {
        Self {
            velocity: [0.0, 0.0, 0.0],
            acceleration: [0.0, 0.0, 0.0],
            angular_velocity: [0.0, 0.0, 0.0],
            mass: 1.0,
            inverse_mass: 1.0,
            gravity_scale: 1.0,
            drag: 0.1,
            grounded: false,
            aabb_min: [-0.5, -0.5, -0.5],
            aabb_max: [0.5, 0.5, 0.5],
        }
    }
}

/// Item component data (POD)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ItemData {
    pub item_id: u32, // Assuming ItemId is u32
    pub stack_size: u32,
    pub pickup_delay: f32,
    pub lifetime: f32,
}

impl Default for ItemData {
    fn default() -> Self {
        Self {
            item_id: 0,
            stack_size: 1,
            pickup_delay: 0.5,
            lifetime: 300.0,
        }
    }
}

/// Renderable component data (POD)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RenderableData {
    pub model_type: u32, // 0 = Cube, 1 = Item
    pub model_data: u32, // BlockId for Item type
    pub color: [f32; 3],
    pub scale: f32,
}

impl Default for RenderableData {
    fn default() -> Self {
        Self {
            model_type: 0,
            model_data: 0,
            color: [1.0, 1.0, 1.0],
            scale: 1.0,
        }
    }
}

/// Pre-allocated component storage for all component types
pub struct ComponentData {
    // Transform components
    pub transforms: Vec<TransformData>,
    pub transform_sparse: Vec<Option<usize>>, // Entity index -> component index
    pub transform_entities: Vec<EntityId>, // Component index -> entity
    pub transform_count: usize,
    
    // Physics components
    pub physics: Vec<PhysicsData>,
    pub physics_sparse: Vec<Option<usize>>,
    pub physics_entities: Vec<EntityId>,
    pub physics_count: usize,
    
    // Item components
    pub items: Vec<ItemData>,
    pub items_sparse: Vec<Option<usize>>,
    pub items_entities: Vec<EntityId>,
    pub items_count: usize,
    
    // Renderable components
    pub renderables: Vec<RenderableData>,
    pub renderables_sparse: Vec<Option<usize>>,
    pub renderables_entities: Vec<EntityId>,
    pub renderables_count: usize,
}

impl ComponentData {
    pub fn new() -> Self {
        Self {
            // Pre-allocate transform storage
            transforms: vec![TransformData::default(); MAX_ENTITIES],
            transform_sparse: vec![None; MAX_ENTITIES],
            transform_entities: vec![EntityId::INVALID; MAX_ENTITIES],
            transform_count: 0,
            
            // Pre-allocate physics storage
            physics: vec![PhysicsData::default(); MAX_ENTITIES],
            physics_sparse: vec![None; MAX_ENTITIES],
            physics_entities: vec![EntityId::INVALID; MAX_ENTITIES],
            physics_count: 0,
            
            // Pre-allocate item storage
            items: vec![ItemData::default(); MAX_ENTITIES],
            items_sparse: vec![None; MAX_ENTITIES],
            items_entities: vec![EntityId::INVALID; MAX_ENTITIES],
            items_count: 0,
            
            // Pre-allocate renderable storage
            renderables: vec![RenderableData::default(); MAX_ENTITIES],
            renderables_sparse: vec![None; MAX_ENTITIES],
            renderables_entities: vec![EntityId::INVALID; MAX_ENTITIES],
            renderables_count: 0,
        }
    }
    
    /// Add transform component to entity
    pub fn add_transform(&mut self, entity: EntityId, data: TransformData) -> bool {
        if !entity.is_valid() {
            return false;
        }
        
        let entity_idx = entity.idx();
        
        // Check if entity already has this component
        if self.transform_sparse[entity_idx].is_some() {
            // Update existing
            let comp_idx = self.transform_sparse[entity_idx].unwrap();
            self.transforms[comp_idx] = data;
            return true;
        }
        
        // Add new component
        if self.transform_count >= MAX_ENTITIES {
            return false;
        }
        
        let comp_idx = self.transform_count;
        self.transforms[comp_idx] = data;
        self.transform_sparse[entity_idx] = Some(comp_idx);
        self.transform_entities[comp_idx] = entity;
        self.transform_count += 1;
        
        true
    }
    
    /// Remove transform component from entity
    pub fn remove_transform(&mut self, entity: EntityId) -> bool {
        if !entity.is_valid() {
            return false;
        }
        
        let entity_idx = entity.idx();
        
        if let Some(comp_idx) = self.transform_sparse[entity_idx] {
            // Swap with last element
            let last_idx = self.transform_count - 1;
            
            if comp_idx != last_idx {
                self.transforms[comp_idx] = self.transforms[last_idx];
                let moved_entity = self.transform_entities[last_idx];
                self.transform_entities[comp_idx] = moved_entity;
                
                // Update sparse array for moved entity
                if moved_entity.is_valid() {
                    self.transform_sparse[moved_entity.idx()] = Some(comp_idx);
                }
            }
            
            // Clear the slot
            self.transform_sparse[entity_idx] = None;
            self.transform_count -= 1;
            
            true
        } else {
            false
        }
    }
    
    /// Get transform component for entity
    pub fn get_transform(&self, entity: EntityId) -> Option<&TransformData> {
        if !entity.is_valid() {
            return None;
        }
        
        self.transform_sparse[entity.idx()]
            .map(|idx| &self.transforms[idx])
    }
    
    /// Get mutable transform component for entity
    pub fn get_transform_mut(&mut self, entity: EntityId) -> Option<&mut TransformData> {
        if !entity.is_valid() {
            return None;
        }
        
        self.transform_sparse[entity.idx()]
            .map(|idx| &mut self.transforms[idx])
    }
    
    // Similar methods for physics components
    pub fn add_physics(&mut self, entity: EntityId, data: PhysicsData) -> bool {
        if !entity.is_valid() {
            return false;
        }
        
        let entity_idx = entity.idx();
        
        if self.physics_sparse[entity_idx].is_some() {
            let comp_idx = self.physics_sparse[entity_idx].unwrap();
            self.physics[comp_idx] = data;
            return true;
        }
        
        if self.physics_count >= MAX_ENTITIES {
            return false;
        }
        
        let comp_idx = self.physics_count;
        self.physics[comp_idx] = data;
        self.physics_sparse[entity_idx] = Some(comp_idx);
        self.physics_entities[comp_idx] = entity;
        self.physics_count += 1;
        
        true
    }
    
    pub fn remove_physics(&mut self, entity: EntityId) -> bool {
        if !entity.is_valid() {
            return false;
        }
        
        let entity_idx = entity.idx();
        
        if let Some(comp_idx) = self.physics_sparse[entity_idx] {
            let last_idx = self.physics_count - 1;
            
            if comp_idx != last_idx {
                self.physics[comp_idx] = self.physics[last_idx];
                let moved_entity = self.physics_entities[last_idx];
                self.physics_entities[comp_idx] = moved_entity;
                
                if moved_entity.is_valid() {
                    self.physics_sparse[moved_entity.idx()] = Some(comp_idx);
                }
            }
            
            self.physics_sparse[entity_idx] = None;
            self.physics_count -= 1;
            
            true
        } else {
            false
        }
    }
    
    pub fn get_physics(&self, entity: EntityId) -> Option<&PhysicsData> {
        if !entity.is_valid() {
            return None;
        }
        
        self.physics_sparse[entity.idx()]
            .map(|idx| &self.physics[idx])
    }
    
    pub fn get_physics_mut(&mut self, entity: EntityId) -> Option<&mut PhysicsData> {
        if !entity.is_valid() {
            return None;
        }
        
        self.physics_sparse[entity.idx()]
            .map(|idx| &mut self.physics[idx])
    }
    
    // Similar methods for item components
    pub fn add_item(&mut self, entity: EntityId, data: ItemData) -> bool {
        if !entity.is_valid() {
            return false;
        }
        
        let entity_idx = entity.idx();
        
        if self.items_sparse[entity_idx].is_some() {
            let comp_idx = self.items_sparse[entity_idx].unwrap();
            self.items[comp_idx] = data;
            return true;
        }
        
        if self.items_count >= MAX_ENTITIES {
            return false;
        }
        
        let comp_idx = self.items_count;
        self.items[comp_idx] = data;
        self.items_sparse[entity_idx] = Some(comp_idx);
        self.items_entities[comp_idx] = entity;
        self.items_count += 1;
        
        true
    }
    
    pub fn remove_item(&mut self, entity: EntityId) -> bool {
        if !entity.is_valid() {
            return false;
        }
        
        let entity_idx = entity.idx();
        
        if let Some(comp_idx) = self.items_sparse[entity_idx] {
            let last_idx = self.items_count - 1;
            
            if comp_idx != last_idx {
                self.items[comp_idx] = self.items[last_idx];
                let moved_entity = self.items_entities[last_idx];
                self.items_entities[comp_idx] = moved_entity;
                
                if moved_entity.is_valid() {
                    self.items_sparse[moved_entity.idx()] = Some(comp_idx);
                }
            }
            
            self.items_sparse[entity_idx] = None;
            self.items_count -= 1;
            
            true
        } else {
            false
        }
    }
    
    // Clear all components for an entity
    pub fn clear_entity(&mut self, entity: EntityId) {
        self.remove_transform(entity);
        self.remove_physics(entity);
        self.remove_item(entity);
        // Add other component removals as needed
    }
    
    /// Clear all component data
    pub fn clear(&mut self) {
        self.transform_count = 0;
        self.physics_count = 0;
        self.items_count = 0;
        self.renderables_count = 0;
        
        // Clear sparse arrays
        for i in 0..MAX_ENTITIES {
            self.transform_sparse[i] = None;
            self.physics_sparse[i] = None;
            self.items_sparse[i] = None;
            self.renderables_sparse[i] = None;
        }
    }
}