use std::sync::atomic::{AtomicU32, Ordering};

/// Maximum number of entities in the ECS
pub const MAX_ENTITIES: usize = 65536;

/// Entity identifier with generation to detect use-after-free
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct EntityId {
    /// Index into entity arrays
    pub index: u32,
    /// Generation counter to detect stale references
    pub generation: u32,
}

impl EntityId {
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };
    
    pub fn is_valid(self) -> bool {
        self.index < MAX_ENTITIES as u32
    }
    
    pub fn idx(self) -> usize {
        self.index as usize
    }
}

/// Entity metadata stored in arrays
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EntityMeta {
    /// Current generation of this slot
    pub generation: u32,
    /// Bitset of component types this entity has
    pub component_mask: u64,
    /// Whether this slot is currently in use
    pub alive: bool,
}

/// Pre-allocated entity storage
pub struct EntityData {
    /// Number of active entities
    pub count: AtomicU32,
    
    /// Entity metadata (generation, component masks, etc)
    pub metas: Vec<EntityMeta>,
    
    /// Free list for recycling entity slots
    pub free_list: Vec<u32>,
    
    /// Next generation to use for new entities
    next_generation: AtomicU32,
}

impl EntityData {
    pub fn new() -> Self {
        let mut metas = Vec::with_capacity(MAX_ENTITIES);
        let mut free_list = Vec::with_capacity(MAX_ENTITIES);
        
        // Initialize all slots as free
        for i in 0..MAX_ENTITIES {
            metas.push(EntityMeta {
                generation: 0,
                component_mask: 0,
                alive: false,
            });
            // Add to free list in reverse order so we allocate from the front
            free_list.push((MAX_ENTITIES - i - 1) as u32);
        }
        
        Self {
            count: AtomicU32::new(0),
            metas,
            free_list,
            next_generation: AtomicU32::new(1),
        }
    }
    
    /// Allocate a new entity
    pub fn create(&mut self) -> EntityId {
        if let Some(index) = self.free_list.pop() {
            let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
            
            self.metas[index as usize] = EntityMeta {
                generation,
                component_mask: 0,
                alive: true,
            };
            
            self.count.fetch_add(1, Ordering::Relaxed);
            
            EntityId { index, generation }
        } else {
            EntityId::INVALID
        }
    }
    
    /// Destroy an entity (returns true if it existed)
    pub fn destroy(&mut self, entity: EntityId) -> bool {
        if !entity.is_valid() {
            return false;
        }
        
        let idx = entity.idx();
        let meta = &mut self.metas[idx];
        
        // Check if entity is valid and alive
        if meta.generation == entity.generation && meta.alive {
            meta.alive = false;
            meta.component_mask = 0;
            meta.generation = meta.generation.wrapping_add(1); // Increment generation
            
            self.free_list.push(entity.index);
            self.count.fetch_sub(1, Ordering::Relaxed);
            
            true
        } else {
            false
        }
    }
    
    /// Check if an entity reference is still valid
    pub fn is_alive(&self, entity: EntityId) -> bool {
        if !entity.is_valid() {
            return false;
        }
        
        let meta = &self.metas[entity.idx()];
        meta.generation == entity.generation && meta.alive
    }
    
    /// Set component bit in entity's mask
    pub fn set_component_bit(&mut self, entity: EntityId, bit: u8) {
        if self.is_alive(entity) {
            self.metas[entity.idx()].component_mask |= 1u64 << bit;
        }
    }
    
    /// Clear component bit in entity's mask
    pub fn clear_component_bit(&mut self, entity: EntityId, bit: u8) {
        if self.is_alive(entity) {
            self.metas[entity.idx()].component_mask &= !(1u64 << bit);
        }
    }
    
    /// Check if entity has a component
    pub fn has_component_bit(&self, entity: EntityId, bit: u8) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        
        (self.metas[entity.idx()].component_mask & (1u64 << bit)) != 0
    }
    
    /// Get active entity count
    pub fn entity_count(&self) -> usize {
        self.count.load(Ordering::Relaxed) as usize
    }
    
    /// Clear all entities
    pub fn clear(&mut self) {
        self.count.store(0, Ordering::Relaxed);
        self.free_list.clear();
        
        // Reset all slots
        for i in 0..MAX_ENTITIES {
            self.metas[i] = EntityMeta {
                generation: 0,
                component_mask: 0,
                alive: false,
            };
            self.free_list.push((MAX_ENTITIES - i - 1) as u32);
        }
    }
}