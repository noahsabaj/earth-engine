/// Attribute Modifiers System
/// 
/// Handles attribute modifications like buffs, debuffs, equipment bonuses.
/// Supports various modifier types and stacking rules.

use crate::attributes::AttributeValue;
use crate::instance::InstanceId;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// Modifier identifier
pub type ModifierId = u64;

/// Modifier operation types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModifierOperation {
    /// Add flat value
    Add = 0,
    /// Multiply by percentage (1.0 = 100%)
    Multiply = 1,
    /// Override with new value
    Override = 2,
    /// Set minimum value
    Min = 3,
    /// Set maximum value
    Max = 4,
}

/// Modifier type categories
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierType {
    /// Temporary buff/debuff
    Temporary = 0,
    /// Equipment bonus
    Equipment = 1,
    /// Skill/talent bonus
    Skill = 2,
    /// Status effect
    Status = 3,
    /// Environmental effect
    Environmental = 4,
    /// Consumable effect
    Consumable = 5,
    /// Permanent upgrade
    Permanent = 6,
    /// Custom type
    Custom = 255,
}

/// Modifier priority (higher applies later)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ModifierPriority {
    Lowest = 0,
    Low = 50,
    Normal = 100,
    High = 150,
    Highest = 200,
    Override = 255,
}

/// Modifier scope
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModifierScope {
    /// Affects single instance
    Instance(InstanceId),
    /// Affects all instances of type
    Type(String),
    /// Affects instances with tag
    Tag(String),
    /// Global modifier
    Global,
}

/// Individual modifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modifier {
    /// Unique ID
    pub id: ModifierId,
    /// Display name
    pub name: String,
    /// Modifier type
    pub modifier_type: ModifierType,
    /// Operation to perform
    pub operation: ModifierOperation,
    /// Value to apply
    pub value: AttributeValue,
    /// Priority for ordering
    pub priority: ModifierPriority,
    /// Source of modifier
    pub source: Option<InstanceId>,
    /// Duration in ticks (None = permanent)
    pub duration: Option<u64>,
    /// Time remaining
    pub time_remaining: Option<u64>,
    /// Can stack with same source
    pub stackable: bool,
    /// Max stack count
    pub max_stacks: u32,
    /// Current stack count
    pub current_stacks: u32,
}

impl Modifier {
    /// Create a new modifier
    pub fn new(
        name: String,
        modifier_type: ModifierType,
        operation: ModifierOperation,
        value: AttributeValue,
    ) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        
        Self {
            id: COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            name,
            modifier_type,
            operation,
            value,
            priority: ModifierPriority::Normal,
            source: None,
            duration: None,
            time_remaining: None,
            stackable: false,
            max_stacks: 1,
            current_stacks: 1,
        }
    }
    
    /// Set priority
    pub fn with_priority(mut self, priority: ModifierPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set duration
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = Some(duration);
        self.time_remaining = Some(duration);
        self
    }
    
    /// Set source
    pub fn with_source(mut self, source: InstanceId) -> Self {
        self.source = Some(source);
        self
    }
    
    /// Make stackable
    pub fn stackable(mut self, max_stacks: u32) -> Self {
        self.stackable = true;
        self.max_stacks = max_stacks;
        self
    }
    
    /// Update time remaining
    pub fn update(&mut self, delta_ticks: u64) -> bool {
        if let Some(remaining) = &mut self.time_remaining {
            *remaining = remaining.saturating_sub(delta_ticks);
            *remaining > 0
        } else {
            true // Permanent
        }
    }
    
    /// Apply modifier to value
    pub fn apply(&self, base_value: AttributeValue) -> AttributeValue {
        match self.operation {
            ModifierOperation::Add => {
                base_value.add(&self.value)
                    .unwrap_or(base_value)
            }
            
            ModifierOperation::Multiply => {
                if let Some(multiplier) = self.value.as_float() {
                    base_value.multiply(multiplier)
                        .unwrap_or(base_value)
                } else {
                    base_value
                }
            }
            
            ModifierOperation::Override => {
                self.value.clone()
            }
            
            ModifierOperation::Min => {
                if base_value.less_than_or_equal(&self.value) {
                    self.value.clone()
                } else {
                    base_value
                }
            }
            
            ModifierOperation::Max => {
                if base_value.greater_than_or_equal(&self.value) {
                    self.value.clone()
                } else {
                    base_value
                }
            }
        }
    }
}

/// Stack of modifiers for an attribute
pub struct ModifierStack {
    /// Modifiers sorted by priority
    modifiers: BTreeMap<(ModifierPriority, ModifierId), Modifier>,
    
    /// Quick lookup by ID
    id_lookup: std::collections::HashMap<ModifierId, ModifierPriority>,
}

impl ModifierStack {
    pub fn new() -> Self {
        Self {
            modifiers: BTreeMap::new(),
            id_lookup: std::collections::HashMap::new(),
        }
    }
    
    /// Add modifier to stack
    pub fn add(&mut self, modifier: Modifier) -> ModifierId {
        let id = modifier.id;
        let priority = modifier.priority;
        
        // Check for stacking
        if modifier.stackable {
            // Find existing modifier from same source
            let existing = self.modifiers.values_mut()
                .find(|m| m.source == modifier.source && m.name == modifier.name);
                
            if let Some(existing_mod) = existing {
                if existing_mod.current_stacks < existing_mod.max_stacks {
                    existing_mod.current_stacks += 1;
                    return existing_mod.id;
                }
            }
        }
        
        self.modifiers.insert((priority, id), modifier);
        self.id_lookup.insert(id, priority);
        
        id
    }
    
    /// Remove modifier
    pub fn remove(&mut self, id: ModifierId) -> bool {
        if let Some(priority) = self.id_lookup.remove(&id) {
            self.modifiers.remove(&(priority, id)).is_some()
        } else {
            false
        }
    }
    
    /// Update all modifiers
    pub fn update(&mut self, delta_ticks: u64) {
        let mut to_remove = Vec::new();
        
        for ((priority, id), modifier) in &mut self.modifiers {
            if !modifier.update(delta_ticks) {
                to_remove.push((*priority, *id));
            }
        }
        
        for key in to_remove {
            self.modifiers.remove(&key);
            self.id_lookup.remove(&key.1);
        }
    }
    
    /// Apply all modifiers to base value
    pub fn apply(&self, base_value: AttributeValue) -> AttributeValue {
        let mut value = base_value;
        
        // Apply in priority order
        for modifier in self.modifiers.values() {
            let stack_multiplier = if modifier.stackable {
                modifier.current_stacks as f64
            } else {
                1.0
            };
            
            // Apply modifier multiple times for stacks
            for _ in 0..modifier.current_stacks {
                value = modifier.apply(value.clone());
            }
        }
        
        value
    }
    
    /// Get all active modifiers
    pub fn get_modifiers(&self) -> Vec<&Modifier> {
        self.modifiers.values().collect()
    }
    
    /// Get modifier by ID
    pub fn get(&self, id: ModifierId) -> Option<&Modifier> {
        self.id_lookup.get(&id)
            .and_then(|&priority| self.modifiers.get(&(priority, id)))
    }
    
    /// Clear all modifiers of type
    pub fn clear_type(&mut self, modifier_type: ModifierType) {
        let to_remove: Vec<_> = self.modifiers
            .iter()
            .filter(|(_, m)| m.modifier_type == modifier_type)
            .map(|((p, id), _)| (*p, *id))
            .collect();
            
        for key in to_remove {
            self.modifiers.remove(&key);
            self.id_lookup.remove(&key.1);
        }
    }
}

/// Modifier templates for common effects
pub struct ModifierTemplates;

impl ModifierTemplates {
    /// Health regeneration modifier
    pub fn health_regen(amount: f64, duration: u64) -> Modifier {
        Modifier::new(
            "Health Regeneration".to_string(),
            ModifierType::Temporary,
            ModifierOperation::Add,
            AttributeValue::Float(amount),
        )
        .with_duration(duration)
    }
    
    /// Damage boost modifier
    pub fn damage_boost(percentage: f64, duration: u64) -> Modifier {
        Modifier::new(
            "Damage Boost".to_string(),
            ModifierType::Temporary,
            ModifierOperation::Multiply,
            AttributeValue::Float(1.0 + percentage),
        )
        .with_duration(duration)
        .with_priority(ModifierPriority::High)
    }
    
    /// Speed reduction (slow)
    pub fn slow(percentage: f64, duration: u64) -> Modifier {
        Modifier::new(
            "Slow".to_string(),
            ModifierType::Status,
            ModifierOperation::Multiply,
            AttributeValue::Float(1.0 - percentage),
        )
        .with_duration(duration)
    }
    
    /// Armor bonus from equipment
    pub fn armor_bonus(amount: i64) -> Modifier {
        Modifier::new(
            "Armor Bonus".to_string(),
            ModifierType::Equipment,
            ModifierOperation::Add,
            AttributeValue::Integer(amount),
        )
        .with_priority(ModifierPriority::Low)
    }
    
    /// Attribute cap
    pub fn attribute_cap(max_value: f64) -> Modifier {
        Modifier::new(
            "Attribute Cap".to_string(),
            ModifierType::Permanent,
            ModifierOperation::Max,
            AttributeValue::Float(max_value),
        )
        .with_priority(ModifierPriority::Highest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_modifier_application() {
        let base = AttributeValue::Float(100.0);
        
        let add_mod = Modifier::new(
            "Test Add".to_string(),
            ModifierType::Temporary,
            ModifierOperation::Add,
            AttributeValue::Float(50.0),
        );
        
        let result = add_mod.apply(base);
        assert_eq!(result, AttributeValue::Float(150.0));
    }
    
    #[test]
    fn test_modifier_stack() {
        let mut stack = ModifierStack::new();
        
        // Add modifiers in different priorities
        let mod1 = Modifier::new(
            "Base Bonus".to_string(),
            ModifierType::Equipment,
            ModifierOperation::Add,
            AttributeValue::Float(10.0),
        ).with_priority(ModifierPriority::Low);
        
        let mod2 = Modifier::new(
            "Percentage Boost".to_string(),
            ModifierType::Temporary,
            ModifierOperation::Multiply,
            AttributeValue::Float(1.5),
        ).with_priority(ModifierPriority::Normal);
        
        stack.add(mod1);
        stack.add(mod2);
        
        // Base 100 + 10 = 110, then * 1.5 = 165
        let result = stack.apply(AttributeValue::Float(100.0));
        assert_eq!(result, AttributeValue::Float(165.0));
    }
    
    #[test]
    fn test_modifier_stacking() {
        let mut stack = ModifierStack::new();
        let source = InstanceId::new();
        
        let stackable = Modifier::new(
            "Stackable Buff".to_string(),
            ModifierType::Temporary,
            ModifierOperation::Add,
            AttributeValue::Float(5.0),
        )
        .with_source(source)
        .stackable(3);
        
        // Add same modifier multiple times
        stack.add(stackable.clone());
        stack.add(stackable.clone());
        
        let modifiers = stack.get_modifiers();
        assert_eq!(modifiers.len(), 1);
        assert_eq!(modifiers[0].current_stacks, 2);
    }
}