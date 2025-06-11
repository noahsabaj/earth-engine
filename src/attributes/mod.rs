/// Dynamic Attribute System
/// 
/// Flexible key-value attribute storage for runtime gameplay data.
/// Supports any data type, modifiers, inheritance, and bulk operations.
/// Purely data-oriented - no attribute "objects", just tables of data.
/// 
/// Part of Sprint 32: Dynamic Attribute System

pub mod attribute_storage;
pub mod attribute_value;
pub mod attribute_modifiers;
pub mod attribute_inheritance;
pub mod bulk_operations;
pub mod change_events;
pub mod computed_attributes;
pub mod error;

pub use attribute_storage::{AttributeStorage, AttributeKey, AttributeIndex};
pub use attribute_value::{AttributeValue, ValueType, TypedValue};
pub use attribute_modifiers::{
    Modifier, ModifierType, ModifierStack, ModifierPriority,
    ModifierOperation, ModifierScope
};
pub use attribute_inheritance::{
    InheritanceChain, InheritanceRule, AttributeSource,
    InheritanceResolver
};
pub use bulk_operations::{BulkUpdate, BulkQuery, BulkResult};
pub use change_events::{
    AttributeEvent, EventType, ChangeListener,
    EventDispatcher
};
pub use computed_attributes::{
    ComputedAttribute, ComputeFunction, DependencyGraph
};
pub use error::{AttributeResult, AttributeErrorContext};

use crate::instance::InstanceId;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Maximum attributes per instance (configurable)
pub const MAX_ATTRIBUTES_PER_INSTANCE: usize = 256;

/// Maximum total attributes in system
pub const MAX_TOTAL_ATTRIBUTES: usize = 1 << 20; // 1M

/// Attribute categories for organization
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeCategory {
    Core = 0,        // Health, mana, stamina
    Combat = 1,      // Attack, defense, speed
    Skills = 2,      // Mining, crafting, magic
    Status = 3,      // Buffs, debuffs, conditions
    Resources = 4,   // Gold, materials, consumables
    Metadata = 5,    // Name, description, tags
    Physics = 6,     // Mass, velocity, friction
    Rendering = 7,   // Color, transparency, glow
    Custom = 255,    // User-defined
}

/// Attribute flags for behavior control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeFlags {
    /// Can be modified at runtime
    pub mutable: bool,
    /// Persisted to save files
    pub persistent: bool,
    /// Synchronized over network
    pub networked: bool,
    /// Visible to other players
    pub public: bool,
    /// Can be inherited
    pub inheritable: bool,
    /// Triggers events on change
    pub observable: bool,
    /// Computed from other attributes
    pub computed: bool,
    /// Cached for performance
    pub cached: bool,
}

impl Default for AttributeFlags {
    fn default() -> Self {
        Self {
            mutable: true,
            persistent: true,
            networked: false,
            public: false,
            inheritable: true,
            observable: true,
            computed: false,
            cached: false,
        }
    }
}

/// Core attribute manager (Structure of Arrays)
pub struct AttributeManager {
    /// Attribute storage backend
    pub storage: AttributeStorage,
    
    /// Modifier stacks per attribute
    pub modifiers: HashMap<(InstanceId, AttributeKey), ModifierStack>,
    
    /// Inheritance chains
    pub inheritance: InheritanceResolver,
    
    /// Computed attribute definitions
    pub computed: HashMap<AttributeKey, ComputedAttribute>,
    
    /// Event dispatcher
    pub events: EventDispatcher,
    
    /// Dependency graph for computed attributes
    pub dependencies: DependencyGraph,
    
    /// Attribute metadata
    pub metadata: AttributeMetadata,
}

/// Attribute metadata storage
pub struct AttributeMetadata {
    /// Registered attribute definitions
    pub definitions: HashMap<AttributeKey, AttributeDefinition>,
    
    /// Category groupings
    pub by_category: HashMap<AttributeCategory, Vec<AttributeKey>>,
    
    /// Default values
    pub defaults: HashMap<AttributeKey, AttributeValue>,
}

/// Attribute definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    pub key: AttributeKey,
    pub name: String,
    pub category: AttributeCategory,
    pub value_type: ValueType,
    pub flags: AttributeFlags,
    pub min_value: Option<AttributeValue>,
    pub max_value: Option<AttributeValue>,
    pub description: String,
}

impl AttributeManager {
    pub fn new() -> Self {
        Self {
            storage: AttributeStorage::new(),
            modifiers: HashMap::new(),
            inheritance: InheritanceResolver::new(),
            computed: HashMap::new(),
            events: EventDispatcher::new(),
            dependencies: DependencyGraph::new(),
            metadata: AttributeMetadata {
                definitions: HashMap::new(),
                by_category: HashMap::new(),
                defaults: HashMap::new(),
            },
        }
    }
    
    /// Register an attribute definition
    pub fn register_attribute(&mut self, def: AttributeDefinition) {
        let key = def.key.clone();
        let category = def.category;
        
        self.metadata.definitions.insert(key.clone(), def);
        self.metadata.by_category
            .entry(category)
            .or_insert_with(Vec::new)
            .push(key);
    }
    
    /// Set attribute value
    pub fn set_attribute(
        &mut self,
        instance: InstanceId,
        key: AttributeKey,
        value: AttributeValue,
    ) -> Result<(), String> {
        // Check if attribute is defined
        let def = self.metadata.definitions.get(&key)
            .ok_or_else(|| format!("Attribute '{}' not registered", key))?;
            
        // Validate type
        if value.value_type() != def.value_type {
            return Err(format!("Type mismatch: expected {:?}, got {:?}",
                def.value_type, value.value_type()));
        }
        
        // Validate range
        if let Some(ref min) = def.min_value {
            if !value.greater_than_or_equal(min) {
                return Err(format!("Value below minimum"));
            }
        }
        
        if let Some(ref max) = def.max_value {
            if !value.less_than_or_equal(max) {
                return Err(format!("Value above maximum"));
            }
        }
        
        // Store value
        let old_value = self.storage.get(instance, &key).cloned();
        self.storage.set(instance, key.clone(), value.clone())?;
        
        // Trigger events
        if def.flags.observable {
            self.events.dispatch(AttributeEvent {
                instance,
                key: key.clone(),
                event_type: EventType::Changed,
                old_value,
                new_value: Some(value),
                timestamp: std::time::Instant::now(),
            });
        }
        
        // Invalidate computed attributes that depend on this
        self.dependencies.invalidate_dependents(&key);
        
        Ok(())
    }
    
    /// Get attribute value (with modifiers applied)
    pub fn get_attribute(
        &self,
        instance: InstanceId,
        key: &AttributeKey,
    ) -> Option<AttributeValue> {
        // Check if computed
        if let Some(computed) = self.computed.get(key) {
            return computed.compute(instance, self);
        }
        
        // Get base value
        let mut value = self.storage.get(instance, key)?
            .clone();
            
        // Apply modifiers
        if let Some(modifiers) = self.modifiers.get(&(instance, key.clone())) {
            value = modifiers.apply(value);
        }
        
        // Apply inheritance if no local value
        if value.is_null() {
            if let Some(inherited) = self.inheritance.resolve(instance, key, self) {
                value = inherited;
            }
        }
        
        Some(value)
    }
    
    /// Add modifier to attribute
    pub fn add_modifier(
        &mut self,
        instance: InstanceId,
        key: AttributeKey,
        modifier: Modifier,
    ) -> Result<u64, String> {
        let stack = self.modifiers
            .entry((instance, key.clone()))
            .or_insert_with(ModifierStack::new);
            
        let id = stack.add(modifier);
        
        // Trigger recalculation event
        self.events.dispatch(AttributeEvent {
            instance,
            key,
            event_type: EventType::ModifierAdded,
            old_value: None,
            new_value: None,
            timestamp: std::time::Instant::now(),
        });
        
        Ok(id)
    }
    
    /// Remove modifier
    pub fn remove_modifier(
        &mut self,
        instance: InstanceId,
        key: &AttributeKey,
        modifier_id: u64,
    ) -> bool {
        if let Some(stack) = self.modifiers.get_mut(&(instance, key.clone())) {
            if stack.remove(modifier_id) {
                self.events.dispatch(AttributeEvent {
                    instance,
                    key: key.clone(),
                    event_type: EventType::ModifierRemoved,
                    old_value: None,
                    new_value: None,
                    timestamp: std::time::Instant::now(),
                });
                return true;
            }
        }
        false
    }
}

/// Common attribute keys
pub struct CommonAttributes;

impl CommonAttributes {
    pub const HEALTH: &'static str = "health";
    pub const MAX_HEALTH: &'static str = "max_health";
    pub const MANA: &'static str = "mana";
    pub const MAX_MANA: &'static str = "max_mana";
    pub const STAMINA: &'static str = "stamina";
    pub const MAX_STAMINA: &'static str = "max_stamina";
    
    pub const ATTACK: &'static str = "attack";
    pub const DEFENSE: &'static str = "defense";
    pub const SPEED: &'static str = "speed";
    pub const CRITICAL_CHANCE: &'static str = "critical_chance";
    pub const CRITICAL_DAMAGE: &'static str = "critical_damage";
    
    pub const LEVEL: &'static str = "level";
    pub const EXPERIENCE: &'static str = "experience";
    pub const SKILL_POINTS: &'static str = "skill_points";
    
    pub const NAME: &'static str = "name";
    pub const DESCRIPTION: &'static str = "description";
    pub const ICON: &'static str = "icon";
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_attribute_registration() {
        let mut manager = AttributeManager::new();
        
        let health_def = AttributeDefinition {
            key: CommonAttributes::HEALTH.to_string(),
            name: "Health".to_string(),
            category: AttributeCategory::Core,
            value_type: ValueType::Float,
            flags: AttributeFlags::default(),
            min_value: Some(AttributeValue::Float(0.0)),
            max_value: Some(AttributeValue::Float(9999.0)),
            description: "Current health points".to_string(),
        };
        
        manager.register_attribute(health_def);
        
        assert!(manager.metadata.definitions.contains_key(CommonAttributes::HEALTH));
    }
}