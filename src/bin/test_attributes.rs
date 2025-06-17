#![allow(unused_variables, dead_code, unused_imports)]
/// Test program for Dynamic Attribute System
/// 
/// Demonstrates basic attribute management functionality:
/// - String-keyed attributes
/// - Type-safe value storage
/// - Basic operations

use hearth_engine::attributes::*;
use hearth_engine::instance::{InstanceId, InstanceIdGenerator};
use std::time::Instant;

// Simple instance manager for testing - DOP style
struct InstanceManagerData {
    id_gen: InstanceIdGenerator,
}

/// Create new instance manager data
/// Pure function - returns instance manager data structure
fn create_instance_manager_data() -> InstanceManagerData {
    InstanceManagerData {
        id_gen: InstanceIdGenerator::new(1), // Use node ID 1
    }
}

/// Create instance from manager data
/// Function - transforms instance manager data by generating new instance
fn create_instance_from_manager(manager: &mut InstanceManagerData) -> InstanceId {
    manager.id_gen.generate().unwrap_or_else(|_| InstanceId::new())
}

fn main() {
    println!("=== Hearth Engine: Dynamic Attribute System Test ===\n");
    
    // Create managers
    let mut instance_mgr = create_instance_manager_data();
    let mut attr_mgr = AttributeManager::new();
    
    // Register attribute metadata
    println!("1. Registering attribute metadata...");
    register_game_attributes(&mut attr_mgr);
    
    // Create test instances
    println!("\n2. Creating test instances...");
    let player = create_instance_from_manager(&mut instance_mgr);
    let enemy = create_instance_from_manager(&mut instance_mgr);
    let weapon = create_instance_from_manager(&mut instance_mgr);
    
    println!("   Player: {:?}", player);
    println!("   Enemy:  {:?}", enemy);
    println!("   Weapon: {:?}", weapon);
    
    // Set base attributes
    println!("\n3. Setting base attributes...");
    setup_player_attributes(&mut attr_mgr, player);
    setup_enemy_attributes(&mut attr_mgr, enemy);
    setup_weapon_attributes(&mut attr_mgr, weapon);
    
    // Test modifiers
    println!("\n4. Testing attribute modifiers...");
    test_modifiers(&mut attr_mgr, player);
    
    // Test inheritance
    println!("\n5. Testing attribute inheritance...");
    test_inheritance(&mut attr_mgr, player, weapon);
    
    // Test computed attributes
    println!("\n6. Testing computed attributes...");
    test_computed_attributes(&mut attr_mgr, player);
    
    // Test bulk operations
    println!("\n7. Testing bulk operations...");
    test_bulk_operations(&mut attr_mgr, player, enemy);
    
    // Test change events
    println!("\n8. Testing change events...");
    test_change_events(&mut attr_mgr, player);
    
    // Performance test
    println!("\n9. Performance test...");
    performance_test(&mut attr_mgr);
    
    println!("\n=== Test Complete ===");
}

fn register_game_attributes(mgr: &mut AttributeManager) {
    // Register basic combat attributes using correct API
    mgr.register_attribute(AttributeDefinition {
        key: "health".to_string(),
        name: "Health".to_string(),
        category: AttributeCategory::Combat,
        value_type: ValueType::Float,
        flags: AttributeFlags::default(),
        min_value: Some(AttributeValue::Float(0.0)),
        max_value: Some(AttributeValue::Float(1000.0)),
        description: "Current health points".to_string(),
    });
    
    mgr.register_attribute(AttributeDefinition {
        key: "max_health".to_string(),
        name: "Max Health".to_string(),
        category: AttributeCategory::Combat,
        value_type: ValueType::Float,
        flags: AttributeFlags::default(),
        min_value: None,
        max_value: None,
        description: "Maximum health points".to_string(),
    });
    
    mgr.register_attribute(AttributeDefinition {
        key: "damage".to_string(),
        name: "Damage".to_string(),
        category: AttributeCategory::Combat,
        value_type: ValueType::Float,
        flags: AttributeFlags::default(),
        min_value: None,
        max_value: None,
        description: "Base damage dealt".to_string(),
    });
    
    mgr.register_attribute(AttributeDefinition {
        key: "armor".to_string(),
        name: "Armor".to_string(),
        category: AttributeCategory::Combat,
        value_type: ValueType::Float,
        flags: AttributeFlags::default(),
        min_value: None,
        max_value: None,
        description: "Damage reduction".to_string(),
    });
    
    // Register basic stats
    mgr.register_attribute(AttributeDefinition {
        key: "level".to_string(),
        name: "Level".to_string(),
        category: AttributeCategory::Skills,
        value_type: ValueType::Integer,
        flags: AttributeFlags::default(),
        min_value: Some(AttributeValue::Integer(1)),
        max_value: None,
        description: "Character level".to_string(),
    });
    
    mgr.register_attribute(AttributeDefinition {
        key: "strength".to_string(),
        name: "Strength".to_string(),
        category: AttributeCategory::Core,
        value_type: ValueType::Integer,
        flags: AttributeFlags::default(),
        min_value: None,
        max_value: None,
        description: "Physical strength".to_string(),
    });
    
    mgr.register_attribute(AttributeDefinition {
        key: "speed".to_string(),
        name: "Speed".to_string(),
        category: AttributeCategory::Physics,
        value_type: ValueType::Float,
        flags: AttributeFlags::default(),
        min_value: None,
        max_value: None,
        description: "Movement speed".to_string(),
    });
    
    println!("   Registered {} attributes", mgr.metadata.definitions.len());
}

fn setup_player_attributes(mgr: &mut AttributeManager, player: InstanceId) {
    let _ = mgr.set_attribute(player, "level".to_string(), AttributeValue::Integer(10));
    let _ = mgr.set_attribute(player, "strength".to_string(), AttributeValue::Integer(15));
    let _ = mgr.set_attribute(player, "speed".to_string(), AttributeValue::Float(6.0));
    let _ = mgr.set_attribute(player, "health".to_string(), AttributeValue::Float(150.0));
    
    print_attributes(mgr, player, "Player");
}

fn setup_enemy_attributes(mgr: &mut AttributeManager, enemy: InstanceId) {
    let _ = mgr.set_attribute(enemy, "level".to_string(), AttributeValue::Integer(8));
    let _ = mgr.set_attribute(enemy, "health".to_string(), AttributeValue::Float(80.0));
    let _ = mgr.set_attribute(enemy, "damage".to_string(), AttributeValue::Float(15.0));
    let _ = mgr.set_attribute(enemy, "armor".to_string(), AttributeValue::Float(5.0));
    let _ = mgr.set_attribute(enemy, "speed".to_string(), AttributeValue::Float(4.0));
    
    print_attributes(mgr, enemy, "Enemy");
}

fn setup_weapon_attributes(mgr: &mut AttributeManager, weapon: InstanceId) {
    // Create some basic weapon stats using available attributes
    let _ = mgr.set_attribute(weapon, "damage".to_string(), AttributeValue::Float(25.0));
    let _ = mgr.set_attribute(weapon, "speed".to_string(), AttributeValue::Float(1.2));
    
    print_attributes(mgr, weapon, "Weapon");
}

fn test_modifiers(mgr: &mut AttributeManager, player: InstanceId) {
    println!("   Base damage: {:?}", mgr.get_attribute(player, &"damage".to_string()));
    
    // Create basic modifier
    let damage_modifier = Modifier::new(
        "Damage Boost".to_string(),
        ModifierType::Temporary,
        ModifierOperation::Add,
        AttributeValue::Float(10.0),
    );
    
    let _ = mgr.add_modifier(player, "damage".to_string(), damage_modifier);
    
    println!("   Damage with modifier: {:?}", mgr.get_attribute(player, &"damage".to_string()));
}

fn test_inheritance(mgr: &mut AttributeManager, player: InstanceId, weapon: InstanceId) {
    // Basic inheritance test - just check that basic attributes are accessible
    let strength = mgr.get_attribute(player, &"strength".to_string());
    println!("   Player strength (basic inheritance test): {:?}", strength);
    
    // Note: Full inheritance system would require more complete implementation
    println!("   Inheritance system requires full template implementation");
}

fn test_computed_attributes(mgr: &mut AttributeManager, player: InstanceId) {
    // Basic computed attribute test - just show current values
    let max_health = mgr.get_attribute(player, &"max_health".to_string());
    println!("   Max health: {:?}", max_health);
    
    // Update strength to test attribute changes
    let _ = mgr.set_attribute(player, "strength".to_string(), AttributeValue::Integer(20));
    let new_strength = mgr.get_attribute(player, &"strength".to_string());
    println!("   Updated strength: {:?}", new_strength);
    
    // Note: Full computed attributes system would require ComputedAttribute implementations
    println!("   Computed attributes require full dependency system implementation");
}

fn test_bulk_operations(mgr: &mut AttributeManager, player: InstanceId, enemy: InstanceId) {
    // Simple bulk-like operation - heal both units
    let heal_amount = AttributeValue::Float(50.0);
    
    // Get current health
    let player_health = mgr.get_attribute(player, &"health".to_string());
    let enemy_health = mgr.get_attribute(enemy, &"health".to_string());
    
    // Apply healing if attributes exist
    if let Some(health) = player_health {
        if let Some(healed) = health.add(&heal_amount) {
            let _ = mgr.set_attribute(player, "health".to_string(), healed);
        }
    }
    
    if let Some(health) = enemy_health {
        if let Some(healed) = health.add(&heal_amount) {
            let _ = mgr.set_attribute(enemy, "health".to_string(), healed);
        }
    }
    
    println!("   Applied bulk healing operation to both instances");
    println!("   Player health: {:?}", mgr.get_attribute(player, &"health".to_string()));
    println!("   Enemy health: {:?}", mgr.get_attribute(enemy, &"health".to_string()));
}

fn test_change_events(mgr: &mut AttributeManager, player: InstanceId) {
    // Simple change event test - just demonstrate attribute changes
    let old_health = mgr.get_attribute(player, &"health".to_string());
    println!("   Old health: {:?}", old_health);
    
    // Change health
    let _ = mgr.set_attribute(player, "health".to_string(), AttributeValue::Float(75.0));
    let new_health = mgr.get_attribute(player, &"health".to_string());
    println!("   New health: {:?}", new_health);
    
    // Note: Full event system would require proper listener implementation
    println!("   Change events demonstrated (full listener system needs implementation)");
}

fn performance_test(mgr: &mut AttributeManager) {
    let start = Instant::now();
    let mut id_gen_data = create_instance_manager_data();
    let instances: Vec<_> = (0..1000)
        .map(|_| create_instance_from_manager(&mut id_gen_data))
        .collect();
    
    // Set attributes
    let mut success_count = 0;
    for &instance in &instances {
        if mgr.set_attribute(instance, "health".to_string(), AttributeValue::Float(100.0)).is_ok() {
            success_count += 1;
        }
        if mgr.set_attribute(instance, "damage".to_string(), AttributeValue::Float(10.0)).is_ok() {
            success_count += 1;
        }
    }
    
    let set_time = start.elapsed();
    
    // Manual "bulk" update - multiply damage by 1.5
    let bulk_start = Instant::now();
    let mut update_count = 0;
    for &instance in &instances {
        if let Some(damage) = mgr.get_attribute(instance, &"damage".to_string()) {
            if let Some(new_damage) = damage.multiply(1.5) {
                if mgr.set_attribute(instance, "damage".to_string(), new_damage).is_ok() {
                    update_count += 1;
                }
            }
        }
    }
    let bulk_time = bulk_start.elapsed();
    
    println!("   Set {} attributes on {} instances: {:?}", success_count, instances.len(), set_time);
    println!("   Updated {} damage attributes: {:?}", update_count, bulk_time);
}

fn print_attributes(mgr: &AttributeManager, instance: InstanceId, name: &str) {
    println!("   {} attributes:", name);
    
    // Check registered attributes and show their values for this instance
    for (key, _def) in &mgr.metadata.definitions {
        if let Some(value) = mgr.get_attribute(instance, key) {
            println!("     {}: {}", key, value);
        }
    }
}