/// Test binary for instance module
/// Run with: cargo run --bin test_instance

use earth_engine::instance::*;

fn main() {
    println!("Testing Instance Module...\n");
    
    // Test 1: Instance ID generation
    println!("1. Testing Instance ID generation:");
    let id1 = InstanceId::new();
    let id2 = InstanceId::new();
    println!("  ID1: {}", id1);
    println!("  ID2: {}", id2);
    println!("  Unique: {}", id1 != id2);
    
    // Test 2: Metadata storage
    println!("\n2. Testing Metadata Storage:");
    let mut store = MetadataStore::new();
    let item_id = InstanceId::new();
    
    store.set(item_id, "name", MetadataValue::String("Iron Sword".to_string())).unwrap();
    store.set(item_id, "damage", MetadataValue::I32(10)).unwrap();
    store.set(item_id, "position", MetadataValue::Position([100.0, 64.0, 200.0])).unwrap();
    
    println!("  Name: {:?}", store.get(&item_id, "name"));
    println!("  Damage: {:?}", store.get(&item_id, "damage"));
    println!("  Position: {:?}", store.get(&item_id, "position"));
    
    // Test 3: History tracking
    println!("\n3. Testing History Tracking:");
    let mut history = HistoryLog::new(10);
    let actor = InstanceId::new();
    let builder = HistoryBuilder::new(actor);
    
    history.record(item_id, builder.created(1));
    history.record(item_id, builder.metadata_changed(
        2,
        "damage",
        Some(MetadataValue::I32(10)),
        Some(MetadataValue::I32(15))
    ));
    
    let item_history = history.get_instance_history(&item_id, 10);
    println!("  History entries: {}", item_history.len());
    for (i, entry) in item_history.iter().enumerate() {
        println!("    {}: {:?} (v{})", i, entry.event, entry.version);
    }
    
    // Test 4: Query system
    println!("\n4. Testing Query System:");
    let mut data = InstanceData::new();
    let sword1 = InstanceId::new();
    let sword2 = InstanceId::new();
    let shield = InstanceId::new();
    
    data.add(sword1, InstanceType::Item, actor);
    data.add(sword2, InstanceType::Item, actor);
    data.add(shield, InstanceType::Tool, actor);
    
    store.set(sword1, "type", MetadataValue::String("weapon".to_string())).unwrap();
    store.set(sword2, "type", MetadataValue::String("weapon".to_string())).unwrap();
    store.set(shield, "type", MetadataValue::String("armor".to_string())).unwrap();
    
    let executor = QueryExecutor::new(&data, &store);
    let filter = InstanceQuery::new()
        .with_type(InstanceType::Item)
        .active()
        .build();
    
    let result = executor.execute(filter.as_ref());
    println!("  Found {} items", result.indices.len());
    println!("  Query time: {}μs", result.execution_time_us);
    
    // Test 5: Copy-on-write
    println!("\n5. Testing Copy-on-Write:");
    let mut cow = CowMetadata::new();
    
    // Register sword template
    let mut sword_template = std::collections::HashMap::new();
    sword_template.insert("type", MetadataValue::String("weapon".to_string()));
    sword_template.insert("damage", MetadataValue::I32(10));
    sword_template.insert("durability", MetadataValue::I32(100));
    cow.register_template("sword", sword_template);
    
    // Create instances from template
    let sword3 = InstanceId::new();
    let sword4 = InstanceId::new();
    cow.create_from_template(sword3, "sword").unwrap();
    cow.create_from_template(sword4, "sword").unwrap();
    
    // Modify one sword
    cow.set(sword3, "damage", MetadataValue::I32(15)).unwrap();
    
    println!("  Sword3 damage: {:?}", cow.get(&sword3, "damage"));
    println!("  Sword4 damage: {:?}", cow.get(&sword4, "damage"));
    
    let stats = cow.stats();
    println!("  Shared instances: {}", stats.shared_instances);
    println!("  Modified instances: {}", stats.modified_instances);
    println!("  Memory saved: {} bytes", stats.memory_saved);
    
    // Test 6: Network sync
    println!("\n6. Testing Network Sync:");
    let mut sync = InstanceSync::new();
    sync.add_peer("peer1".to_string());
    
    let snapshot = InstanceSnapshot {
        id: sword3,
        instance_type: InstanceType::Item,
        version: 1,
        metadata: std::collections::HashMap::new(),
        created_at: 12345,
        created_by: actor,
    };
    
    let instances = vec![(sword3, snapshot.clone(), 1)];
    if let Some(packet) = sync.generate_sync_packet("peer1", &instances) {
        println!("  Generated sync packet: {:?}", 
            match packet {
                SyncPacket::Snapshot(_) => "Snapshot",
                SyncPacket::Delta(_) => "Delta",
                SyncPacket::Batch(_) => "Batch",
                _ => "Other",
            }
        );
    }
    
    println!("\nAll tests completed!");
}