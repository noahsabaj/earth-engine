//! Demonstration of the new save/load integration features
//!
//! This example showcases:
//! - Atomic save operations
//! - State validation
//! - Disconnect handling with save protection
//! - Save/load stress testing capabilities

use std::sync::Arc;
use tempfile::TempDir;

use earth_engine::world::World;
use earth_engine::persistence::{
    atomic_save::{AtomicSaveManager, AtomicSaveConfig, SaveOperation, SavePriority},
    state_validator::{StateValidator, ValidationConfig},
};
use earth_engine::network::disconnect_handler::{DisconnectHandler, DisconnectConfig};

fn main() {
    println!("Earth Engine Save/Load Integration Demo");
    println!("======================================");
    
    // Create temporary directory for saves
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    println!("Using temporary directory: {}", temp_dir.path().display());
    
    // Demo 1: Atomic Save Operations
    println!("\n1. Atomic Save Operations Demo");
    demo_atomic_saves(&temp_dir);
    
    // Demo 2: State Validation
    println!("\n2. State Validation Demo");
    demo_state_validation();
    
    // Demo 3: Disconnect Handling
    println!("\n3. Disconnect Handling Demo");
    demo_disconnect_handling(&temp_dir);
    
    // Demo 4: Integration Test
    println!("\n4. Integration Test Demo");
    demo_integration_test(&temp_dir);
    
    println!("\nDemo completed successfully!");
}

fn demo_atomic_saves(temp_dir: &TempDir) {
    println!("Creating atomic save manager...");
    
    let config = AtomicSaveConfig {
        max_concurrent_operations: 4,
        enable_checksums: true,
        backup_count: 3,
        ..Default::default()
    };
    
    let atomic_manager = AtomicSaveManager::new(temp_dir.path().to_path_buf(), config)
        .expect("Failed to create AtomicSaveManager");
    
    println!("Queueing save operations...");
    
    // Queue some save operations
    atomic_manager.queue_operation(SaveOperation::Chunk {
        pos: earth_engine::world::ChunkPos { x: 0, y: 0, z: 0 },
        priority: SavePriority::Normal,
    }).expect("Failed to queue chunk save");
    
    atomic_manager.queue_operation(SaveOperation::Player {
        uuid: "demo_player".to_string(),
        priority: SavePriority::High,
    }).expect("Failed to queue player save");
    
    atomic_manager.queue_operation(SaveOperation::Metadata {
        priority: SavePriority::Critical,
    }).expect("Failed to queue metadata save");
    
    let stats = atomic_manager.get_stats().expect("Failed to get stats");
    println!("Queued {} operations", stats.queue_length);
    
    // Process operations
    let world = World::new(16);
    let mut processed = 0;
    while let Ok(Some(result)) = atomic_manager.process_next_operation(&world) {
        processed += 1;
        println!("Processed operation: success={}, duration={:?}", 
                result.success, result.duration);
        
        if processed >= 3 {
            break;
        }
    }
    
    let final_stats = atomic_manager.get_stats().expect("Failed to get final stats");
    println!("Final stats: {} completed, {} failed", 
             final_stats.operations_completed, final_stats.operations_failed);
}

fn demo_state_validation() {
    println!("Creating state validator...");
    
    let config = ValidationConfig {
        auto_validate: true,
        enable_checksums: true,
        enable_deep_validation: false,
        ..Default::default()
    };
    
    let mut validator = StateValidator::new(config);
    let world = World::new(16);
    
    println!("Taking state snapshots...");
    
    // Take snapshots
    validator.take_network_snapshot(&world, "demo_snapshot".to_string())
        .expect("Failed to take network snapshot");
    validator.take_persistence_snapshot(&world, "demo_snapshot".to_string())
        .expect("Failed to take persistence snapshot");
    
    println!("Validating state consistency...");
    
    // Validate consistency
    let result = validator.validate_consistency("demo_snapshot")
        .expect("Failed to validate consistency");
    
    println!("Validation result: success={}, errors={}, warnings={}", 
             result.success, result.errors.len(), result.warnings.len());
    
    if !result.errors.is_empty() {
        println!("Validation errors:");
        for error in &result.errors {
            println!("  - {}", error.description);
        }
    }
    
    if !result.warnings.is_empty() {
        println!("Validation warnings:");
        for warning in &result.warnings {
            println!("  - {}", warning.description);
        }
    }
    
    let stats = validator.get_validation_stats();
    println!("Validation stats: {} total, {} successful, avg time: {:?}",
             stats.total_validations, stats.successful_validations, stats.average_duration);
}

fn demo_disconnect_handling(temp_dir: &TempDir) {
    println!("Setting up disconnect handling...");
    
    // Create atomic save manager for disconnect handler
    let atomic_config = AtomicSaveConfig::default();
    let save_manager = Arc::new(AtomicSaveManager::new(
        temp_dir.path().to_path_buf(), 
        atomic_config
    ).expect("Failed to create AtomicSaveManager"));
    
    let disconnect_config = DisconnectConfig {
        max_save_timeout: std::time::Duration::from_secs(10),
        chunk_save_radius: 2,
        emergency_save_enabled: true,
        reconnect_grace_period: std::time::Duration::from_millis(500),
        ..Default::default()
    };
    
    let mut disconnect_handler = DisconnectHandler::new(save_manager, disconnect_config);
    
    println!("Starting disconnect handler...");
    disconnect_handler.start().expect("Failed to start disconnect handler");
    
    let world = World::new(16);
    
    println!("Simulating player disconnects...");
    
    // Simulate normal disconnect
    disconnect_handler.handle_disconnect(
        "player_1".to_string(),
        "Player1".to_string(),
        &world,
        (100.0, 64.0, 200.0),
    ).expect("Failed to handle disconnect");
    
    // Simulate emergency disconnect
    disconnect_handler.handle_emergency_disconnect(
        "player_2".to_string(),
        &world,
        (200.0, 64.0, 300.0),
    ).expect("Failed to handle emergency disconnect");
    
    // Check status
    println!("Player 1 disconnecting: {}", 
             disconnect_handler.is_player_disconnecting("player_1"));
    println!("Player 2 disconnecting: {}", 
             disconnect_handler.is_player_disconnecting("player_2"));
    
    // Wait a bit for processing
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let stats = disconnect_handler.get_stats().expect("Failed to get disconnect stats");
    println!("Disconnect stats: {} disconnecting, {} successful saves, {} emergency saves",
             stats.players_disconnecting, stats.successful_saves, stats.emergency_saves);
    
    println!("Stopping disconnect handler...");
    disconnect_handler.stop().expect("Failed to stop disconnect handler");
}

fn demo_integration_test(temp_dir: &TempDir) {
    println!("Running integration test with all components...");
    
    // Create all components
    let atomic_config = AtomicSaveConfig {
        max_concurrent_operations: 2,
        enable_checksums: true,
        ..Default::default()
    };
    
    let atomic_manager = Arc::new(AtomicSaveManager::new(
        temp_dir.path().to_path_buf(),
        atomic_config,
    ).expect("Failed to create AtomicSaveManager"));
    
    let validation_config = ValidationConfig {
        auto_validate: false, // Manual control for demo
        enable_checksums: true,
        ..Default::default()
    };
    
    let mut validator = StateValidator::new(validation_config);
    
    let disconnect_config = DisconnectConfig::default();
    let disconnect_handler = DisconnectHandler::new(
        Arc::clone(&atomic_manager),
        disconnect_config,
    );
    
    let world = World::new(16);
    
    println!("Performing integrated save/load cycle...");
    
    // 1. Take initial state snapshot
    validator.take_network_snapshot(&world, "integration_test".to_string())
        .expect("Failed to take network snapshot");
    
    // 2. Queue some saves
    atomic_manager.queue_operation(SaveOperation::FullWorld {
        priority: SavePriority::Normal,
    }).expect("Failed to queue world save");
    
    // 3. Process saves
    let mut save_results = Vec::new();
    while let Ok(Some(result)) = atomic_manager.process_next_operation(&world) {
        save_results.push(result);
    }
    
    // 4. Take post-save snapshot
    validator.take_persistence_snapshot(&world, "integration_test".to_string())
        .expect("Failed to take persistence snapshot");
    
    // 5. Validate consistency
    let validation_result = validator.validate_consistency("integration_test")
        .expect("Failed to validate consistency");
    
    println!("Integration test results:");
    println!("  Save operations: {} completed", save_results.len());
    println!("  Validation: success={}, errors={}", 
             validation_result.success, validation_result.errors.len());
    
    let atomic_stats = atomic_manager.get_stats().expect("Failed to get atomic stats");
    println!("  Atomic saves: {} completed, {} failed", 
             atomic_stats.operations_completed, atomic_stats.operations_failed);
    
    let disconnect_stats = disconnect_handler.get_stats().expect("Failed to get disconnect stats");
    println!("  Disconnect handler ready: {} players being tracked",
             disconnect_stats.players_disconnecting);
    
    println!("Integration test completed successfully!");
}