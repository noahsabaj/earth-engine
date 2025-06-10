# Sprint 32: Dynamic Attribute System

## Overview
Sprint 32 implements a flexible, runtime attribute system for gameplay data. Unlike traditional hardcoded stats, this system allows for string-keyed attributes that can be added, modified, and computed dynamically.

## Architecture Decisions

### String-Keyed Storage
- **Decision**: Use string keys for attribute names
- **Rationale**: Maximum flexibility for modding and runtime creation
- **Trade-off**: Slightly higher memory usage vs compile-time safety

### Columnar Storage Layout
- **Decision**: Store attributes in columns by type
- **Rationale**: Cache-efficient access patterns, especially for bulk operations
- **Implementation**: Separate storage for each value type

### Type-Safe Value System
- **Decision**: Enum-based value storage without boxing
- **Rationale**: Avoid heap allocations while maintaining type safety
- **Types**: Bool, Integer, Float, String, Position, etc.

## Core Components

### 1. Attribute Storage (`/src/attributes/attribute_storage.rs`)
```rust
pub struct AttributeStorage {
    key_index: HashMap<AttributeKey, AttributeIndex>,
    instance_data: HashMap<InstanceId, InstanceAttributes>,
    columns: HashMap<AttributeIndex, AttributeColumn>,
}
```
- Sparse storage for memory efficiency
- Fast lookup via indices
- Bulk operation support

### 2. Modifier System (`/src/attributes/attribute_modifiers.rs`)
```rust
pub struct Modifier {
    pub operation: ModifierOperation,  // Add, Multiply, Override, Min, Max
    pub value: AttributeValue,
    pub priority: ModifierPriority,
    pub duration: Option<u64>,
}
```
- Priority-based application order
- Stackable modifiers with limits
- Time-based expiration

### 3. Inheritance System (`/src/attributes/attribute_inheritance.rs`)
```rust
pub enum AttributeSource {
    Parent(InstanceId),
    Template(String),
    Class(String),
    Tag(String),
    Default,
}
```
- Multiple inheritance sources
- Merge strategies (First, Sum, Max, etc.)
- Conditional inheritance

### 4. Computed Attributes (`/src/attributes/computed_attributes.rs`)
```rust
pub struct ComputedAttribute {
    pub dependencies: Vec<AttributeKey>,
    pub compute_fn: ComputeFunction,
    pub cache_policy: CachePolicy,
}
```
- Dependency graph with cycle detection
- Automatic invalidation on change
- Caching for performance

### 5. Change Events (`/src/attributes/change_events.rs`)
```rust
pub struct AttributeEvent {
    pub instance: InstanceId,
    pub key: AttributeKey,
    pub event_type: EventType,
    pub old_value: Option<AttributeValue>,
    pub new_value: Option<AttributeValue>,
}
```
- Event-driven architecture
- Filtered listeners
- Event history tracking

### 6. Bulk Operations (`/src/attributes/bulk_operations.rs`)
```rust
pub struct BulkUpdate {
    pub targets: TargetSelection,
    pub operations: Vec<BulkOperation>,
    pub atomic: bool,
}
```
- Efficient mass updates
- Target selection (by attribute, category, predicate)
- Atomic transaction support

## Usage Examples

### Basic Attribute Management
```rust
// Set attribute
manager.set_attribute(player, "health", AttributeValue::Float(100.0));

// Get with modifiers applied
let health = manager.get_attribute(player, "health");

// Remove attribute
manager.remove_attribute(player, "temp_buff");
```

### Modifier Application
```rust
// Add damage boost
let boost = Modifier::new("Damage Boost", ModifierType::Temporary, 
    ModifierOperation::Multiply, AttributeValue::Float(1.5))
    .with_duration(300);
    
manager.add_modifier(player, "damage", boost);
```

### Inheritance Setup
```rust
// Create class template
let warrior_attrs = HashMap::from([
    ("base_health", AttributeValue::Float(150.0)),
    ("strength_bonus", AttributeValue::Integer(5)),
]);

manager.inheritance.register_class("warrior", warrior_attrs);

// Set instance to inherit from class
manager.inheritance.get_or_create_chain(player)
    .add_source(AttributeSource::Class("warrior"));
```

### Computed Attributes
```rust
// Register max health computation
let max_health = ComputedAttribute::new(
    "max_health",
    vec!["level", "constitution"],
    Arc::new(|instance, manager| {
        let level = manager.get_attribute(instance, "level")?.as_integer()?;
        let con = manager.get_attribute(instance, "constitution")?.as_integer()?;
        Some(AttributeValue::Float(100.0 + level * 10.0 + con * 5.0))
    })
);

manager.register_computed(max_health);
```

### Bulk Operations
```rust
// Heal all players
let heal = BulkUpdateBuilder::new()
    .targets(TargetSelection::WithAttribute("player_tag"))
    .add("health", AttributeValue::Float(50.0))
    .atomic()
    .build();
    
let result = BulkExecutor::execute_update(&heal, &mut manager);
```

## Performance Characteristics

### Memory Usage
- Base overhead: ~48 bytes per attribute definition
- Per-instance: 8 bytes (index) + value size
- Sparse storage reduces memory for unused attributes

### Access Patterns
- Get attribute: O(1) average case
- Set attribute: O(1) average case
- Bulk update: O(n) where n = affected instances
- Computed resolution: O(d) where d = dependency depth

### Optimization Strategies
1. **Column Storage**: Groups same attributes for cache efficiency
2. **Index-based Access**: Avoids string hashing in hot paths
3. **Lazy Computation**: Only compute when requested
4. **Event Batching**: Process changes in bulk
5. **Parallel Bulk Ops**: Use Rayon for large operations

## Integration Points

### With Instance System (Sprint 30)
- Attributes tied to InstanceId
- Lifecycle management via instance events

### With Process System (Sprint 31)
- Processes can modify attributes over time
- State transitions based on attribute values

### Future: With Ability System
- Abilities will modify attributes
- Cooldowns as temporary attributes
- Resource costs via attribute checks

## Common Patterns

### 1. Buff/Debuff Stacking
```rust
// Stackable poison effect
let poison = Modifier::new("Poison", ModifierType::Status,
    ModifierOperation::Add, AttributeValue::Float(-5.0))
    .stackable(3)
    .with_duration(100);
```

### 2. Equipment Bonuses
```rust
// Permanent while equipped
let sword_bonus = Modifier::new("Sword Damage", ModifierType::Equipment,
    ModifierOperation::Add, AttributeValue::Float(25.0))
    .with_source(sword_id);
```

### 3. Conditional Attributes
```rust
// Only inherit if not already set
let rule = InheritanceRule {
    attribute: "faction",
    condition: Some(InheritanceCondition::NotSet),
    strategy: MergeStrategy::First,
};
```

## Testing Strategy

### Unit Tests
- Each component has dedicated test module
- Focus on edge cases (cycles, overflow, etc.)
- Performance benchmarks for critical paths

### Integration Test (`/src/bin/test_attributes.rs`)
- Full system demonstration
- Performance metrics
- Common gameplay scenarios

## Migration Guide

### From Hardcoded Stats
```rust
// Before: struct Player { health: f32, damage: f32 }
// After:
manager.set_attribute(player, "health", AttributeValue::Float(100.0));
manager.set_attribute(player, "damage", AttributeValue::Float(10.0));
```

### From Component System
```rust
// Before: world.get_component::<Health>(entity)
// After: manager.get_attribute(instance, "health")
```

## Future Enhancements

### Sprint 40+: Persistence
- Efficient serialization format
- Delta compression for networking
- Save/load attribute snapshots

### Sprint 45+: Scripting
- Lua bindings for attribute access
- Custom computed attribute formulas
- Runtime attribute definition

### Performance Optimizations
- SIMD operations for bulk math
- Memory pooling for events
- Compressed attribute storage

## Conclusion
The Dynamic Attribute System provides the flexibility needed for complex gameplay mechanics while maintaining the performance requirements of a real-time engine. Its data-oriented design aligns with the engine's architecture principles, and its extensible nature supports future gameplay features.