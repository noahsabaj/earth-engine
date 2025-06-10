/// Test program for Dynamic Attribute System
/// 
/// Demonstrates flexible runtime attribute management with:
/// - String-keyed attributes
/// - Type-safe value storage
/// - Modifiers and calculations
/// - Inheritance chains
/// - Bulk operations
/// - Change events

use earth_engine::attributes::*;
use earth_engine::instance::{InstanceId, InstanceManager};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("=== Earth Engine: Dynamic Attribute System Test ===\n");
    
    // Create managers
    let mut instance_mgr = InstanceManager::new();
    let mut attr_mgr = AttributeManager::new();
    
    // Register attribute metadata
    println!("1. Registering attribute metadata...");
    register_game_attributes(&mut attr_mgr);
    
    // Create test instances
    println!("\n2. Creating test instances...");
    let player = instance_mgr.create_instance();
    let enemy = instance_mgr.create_instance();
    let weapon = instance_mgr.create_instance();
    
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
    use AttributeCategory;
    
    // Register combat attributes
    mgr.register_attribute(AttributeMetadataBuilder::new("health", AttributeCategory::Combat)
        .display_name("Health")
        .description("Current health points")
        .default_value(AttributeValue::Float(100.0))
        .value_type(AttributeValueType::Float)
        .range(Some(0.0), Some(1000.0))
        .build());
        
    mgr.register_attribute(AttributeMetadataBuilder::new("max_health", AttributeCategory::Combat)
        .display_name("Max Health")
        .description("Maximum health points")
        .default_value(AttributeValue::Float(100.0))
        .value_type(AttributeValueType::Float)
        .build());
        
    mgr.register_attribute(AttributeMetadataBuilder::new("damage", AttributeCategory::Combat)
        .display_name("Damage")
        .description("Base damage dealt")
        .default_value(AttributeValue::Float(10.0))
        .value_type(AttributeValueType::Float)
        .build());
        
    mgr.register_attribute(AttributeMetadataBuilder::new("armor", AttributeCategory::Combat)
        .display_name("Armor")
        .description("Damage reduction")
        .default_value(AttributeValue::Float(0.0))
        .value_type(AttributeValueType::Float)
        .build());
    
    // Register stats
    mgr.register_attribute(AttributeMetadataBuilder::new("level", AttributeCategory::Progression)
        .display_name("Level")
        .default_value(AttributeValue::Integer(1))
        .value_type(AttributeValueType::Integer)
        .build());
        
    mgr.register_attribute(AttributeMetadataBuilder::new("strength", AttributeCategory::Stats)
        .display_name("Strength")
        .default_value(AttributeValue::Integer(10))
        .value_type(AttributeValueType::Integer)
        .build());
        
    mgr.register_attribute(AttributeMetadataBuilder::new("dexterity", AttributeCategory::Stats)
        .display_name("Dexterity")
        .default_value(AttributeValue::Integer(10))
        .value_type(AttributeValueType::Integer)
        .build());
        
    mgr.register_attribute(AttributeMetadataBuilder::new("constitution", AttributeCategory::Stats)
        .display_name("Constitution")
        .default_value(AttributeValue::Integer(10))
        .value_type(AttributeValueType::Integer)
        .build());
    
    // Register movement
    mgr.register_attribute(AttributeMetadataBuilder::new("base_speed", AttributeCategory::Movement)
        .display_name("Base Speed")
        .default_value(AttributeValue::Float(5.0))
        .value_type(AttributeValueType::Float)
        .build());
        
    mgr.register_attribute(AttributeMetadataBuilder::new("movement_speed", AttributeCategory::Movement)
        .display_name("Movement Speed")
        .default_value(AttributeValue::Float(5.0))
        .value_type(AttributeValueType::Float)
        .build());
    
    // Register weapon attributes
    mgr.register_attribute(AttributeMetadataBuilder::new("weapon_damage", AttributeCategory::Equipment)
        .display_name("Weapon Damage")
        .default_value(AttributeValue::Float(0.0))
        .value_type(AttributeValueType::Float)
        .build());
        
    mgr.register_attribute(AttributeMetadataBuilder::new("attack_speed", AttributeCategory::Equipment)
        .display_name("Attack Speed")
        .default_value(AttributeValue::Float(1.0))
        .value_type(AttributeValueType::Float)
        .build());
    
    // Register computed attributes
    mgr.register_computed(ComputedTemplates::max_health());
    mgr.register_computed(ComputedTemplates::attack_power());
    mgr.register_computed(ComputedTemplates::movement_speed());
    
    println!("   Registered {} attributes", mgr.metadata.definitions.len());
}

fn setup_player_attributes(mgr: &mut AttributeManager, player: InstanceId) {
    mgr.set_attribute(player, "level".to_string(), AttributeValue::Integer(10));
    mgr.set_attribute(player, "strength".to_string(), AttributeValue::Integer(15));
    mgr.set_attribute(player, "dexterity".to_string(), AttributeValue::Integer(12));
    mgr.set_attribute(player, "constitution".to_string(), AttributeValue::Integer(14));
    mgr.set_attribute(player, "base_speed".to_string(), AttributeValue::Float(6.0));
    mgr.set_attribute(player, "health".to_string(), AttributeValue::Float(150.0));
    
    print_attributes(mgr, player, "Player");
}

fn setup_enemy_attributes(mgr: &mut AttributeManager, enemy: InstanceId) {
    mgr.set_attribute(enemy, "level".to_string(), AttributeValue::Integer(8));
    mgr.set_attribute(enemy, "health".to_string(), AttributeValue::Float(80.0));
    mgr.set_attribute(enemy, "damage".to_string(), AttributeValue::Float(15.0));
    mgr.set_attribute(enemy, "armor".to_string(), AttributeValue::Float(5.0));
    mgr.set_attribute(enemy, "base_speed".to_string(), AttributeValue::Float(4.0));
    
    print_attributes(mgr, enemy, "Enemy");
}

fn setup_weapon_attributes(mgr: &mut AttributeManager, weapon: InstanceId) {
    mgr.set_attribute(weapon, "weapon_damage".to_string(), AttributeValue::Float(25.0));
    mgr.set_attribute(weapon, "attack_speed".to_string(), AttributeValue::Float(1.2));
    
    print_attributes(mgr, weapon, "Weapon");
}

fn test_modifiers(mgr: &mut AttributeManager, player: InstanceId) {
    println!("   Base damage: {:?}", mgr.get_attribute(player, &"damage".to_string()));
    
    // Add damage boost
    let boost = ModifierTemplates::damage_boost(0.5, 100); // 50% boost
    mgr.add_modifier(player, "damage".to_string(), boost);
    
    println!("   Damage with 50% boost: {:?}", mgr.get_attribute(player, &"damage".to_string()));
    
    // Add armor bonus
    let armor = ModifierTemplates::armor_bonus(20);
    mgr.add_modifier(player, "armor".to_string(), armor);
    
    println!("   Armor with +20 bonus: {:?}", mgr.get_attribute(player, &"armor".to_string()));
    
    // Test stacking modifiers
    let stackable = Modifier::new(
        "Power Stack".to_string(),
        ModifierType::Temporary,
        ModifierOperation::Add,
        AttributeValue::Float(5.0),
    ).stackable(5);
    
    mgr.add_modifier(player, "damage".to_string(), stackable.clone());
    mgr.add_modifier(player, "damage".to_string(), stackable);
    
    println!("   Damage with stacking buffs: {:?}", mgr.get_attribute(player, &"damage".to_string()));
}

fn test_inheritance(mgr: &mut AttributeManager, player: InstanceId, weapon: InstanceId) {
    // Create warrior template
    let mut warrior_attrs = std::collections::HashMap::new();
    warrior_attrs.insert("strength".to_string(), AttributeValue::Integer(5));
    warrior_attrs.insert("constitution".to_string(), AttributeValue::Integer(5));
    warrior_attrs.insert("base_damage".to_string(), AttributeValue::Float(15.0));
    
    mgr.inheritance.register_template("warrior".to_string(), warrior_attrs);
    
    // Set player to inherit from warrior template
    let chain = mgr.inheritance.get_or_create_chain(player);
    chain.add_source(AttributeSource::Template("warrior".to_string()));
    
    // Test inheritance resolution
    let inherited = mgr.inheritance.resolve(
        player,
        &"base_damage".to_string(),
        mgr
    );
    
    println!("   Inherited base_damage from warrior template: {:?}", inherited);
}

fn test_computed_attributes(mgr: &mut AttributeManager, player: InstanceId) {
    // Max health (level * 10 + constitution * 5 + 100)
    let max_health = mgr.get_attribute(player, &"max_health".to_string());
    println!("   Computed max_health: {:?}", max_health);
    
    // Set weapon damage for attack power calculation
    mgr.set_attribute(player, "weapon_damage".to_string(), AttributeValue::Float(25.0));
    
    // Attack power (strength * 2 + weapon_damage)
    let attack_power = mgr.get_attribute(player, &"attack_power".to_string());
    println!("   Computed attack_power: {:?}", attack_power);
    
    // Test dependency invalidation
    mgr.set_attribute(player, "strength".to_string(), AttributeValue::Integer(20));
    let new_attack = mgr.get_attribute(player, &"attack_power".to_string());
    println!("   Attack power after strength increase: {:?}", new_attack);
}

fn test_bulk_operations(mgr: &mut AttributeManager, player: InstanceId, enemy: InstanceId) {
    // Heal all units
    let heal = BulkUpdateBuilder::new()
        .targets(TargetSelection::Instances(vec![player, enemy]))
        .add("health".to_string(), AttributeValue::Float(50.0))
        .build();
        
    let result = BulkExecutor::execute_update(&heal, mgr);
    println!("   Healed {} instances, modified {} attributes", 
        result.affected_count, result.modified_attributes);
    
    // Query all combat attributes
    let query = BulkQuery {
        targets: TargetSelection::Instances(vec![player, enemy]),
        attributes: AttributeSelection::Category(AttributeCategory::Combat),
        sort_by: Some(("health".to_string(), SortOrder::Descending)),
        limit: None,
        parallel: false,
    };
    
    let results = BulkExecutor::execute_query(&query, mgr);
    println!("   Combat attributes sorted by health:");
    for (instance, attrs) in results {
        println!("     {:?}: {:?}", instance, attrs.get("health"));
    }
}

fn test_change_events(mgr: &mut AttributeManager, player: InstanceId) {
    // Add event listener
    let watcher = AttributeWatcher {
        watched_keys: vec!["health".to_string()],
        callback: Arc::new(|event| {
            println!("   Health changed from {:?} to {:?}", 
                event.old_value, event.new_value);
        }),
    };
    
    mgr.events.register(Box::new(watcher));
    
    // Trigger health change
    let old_health = mgr.get_attribute(player, &"health".to_string());
    mgr.set_attribute(player, "health".to_string(), AttributeValue::Float(75.0));
    
    // Process event queue
    let events = mgr.events.process_queue();
    println!("   Processed {} events", events.len());
}

fn performance_test(mgr: &mut AttributeManager) {
    let start = Instant::now();
    let instances: Vec<_> = (0..10000)
        .map(|_| InstanceId::new())
        .collect();
    
    // Set attributes
    for &instance in &instances {
        mgr.set_attribute(instance, "health".to_string(), AttributeValue::Float(100.0));
        mgr.set_attribute(instance, "damage".to_string(), AttributeValue::Float(10.0));
    }
    
    let set_time = start.elapsed();
    
    // Bulk update
    let bulk_start = Instant::now();
    let update = BulkUpdateBuilder::new()
        .targets(TargetSelection::Instances(instances.clone()))
        .multiply("damage".to_string(), 1.5)
        .build();
        
    let result = BulkExecutor::execute_update(&update, mgr);
    let bulk_time = bulk_start.elapsed();
    
    println!("   Set 20k attributes: {:?}", set_time);
    println!("   Bulk update 10k instances: {:?} ({} µs/instance)", 
        bulk_time, result.execution_time_us / instances.len() as u64);
}

fn print_attributes(mgr: &AttributeManager, instance: InstanceId, name: &str) {
    let attrs = mgr.storage.get_all(instance);
    println!("   {} attributes:", name);
    for (key, value) in attrs {
        println!("     {}: {:?}", key, value);
    }
}