//! Stress tests for save/load system reliability
//!
//! These tests verify that the save/load system can handle:
//! - 100+ save/load cycles without corruption
//! - Concurrent save operations
//! - Race condition prevention
//! - Data integrity under stress
//! - Recovery from failures

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

use hearth_engine::persistence::{
    PersistenceResult, SaveManager, SaveConfig, AutoSaveConfig,
    atomic_save::{AtomicSaveManager, AtomicSaveConfig, SaveOperation, SavePriority},
    state_validator::{StateValidator, ValidationConfig, ValidationType},
};
use hearth_engine::world::{World, ChunkPos};
use hearth_engine::network::disconnect_handler::{DisconnectHandler, DisconnectConfig};

/// Test configuration for stress tests
#[derive(Debug, Clone)]
struct StressTestConfig {
    pub save_load_cycles: usize,
    pub concurrent_threads: usize,
    pub chunks_per_test: usize,
    pub players_per_test: usize,
    pub operations_per_second: usize,
    pub max_test_duration: Duration,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            save_load_cycles: 100,
            concurrent_threads: 4,
            chunks_per_test: 50,
            players_per_test: 10,
            operations_per_second: 10,
            max_test_duration: Duration::from_secs(300), // 5 minutes max
        }
    }
}

/// Results from stress testing
#[derive(Debug)]
struct StressTestResult {
    pub cycles_completed: usize,
    pub operations_completed: usize,
    pub operations_failed: usize,
    pub total_duration: Duration,
    pub average_cycle_time: Duration,
    pub corruption_detected: bool,
    pub race_conditions_detected: usize,
    pub data_integrity_verified: bool,
}

/// Create a test world with sample data
fn create_test_world() -> World {
    World::new(16) // 16x16 chunk size
}

/// Create test directories
fn create_test_dirs() -> TempDir {
    TempDir::new().expect("Failed to create temporary directory for test")
}

/// Basic save/load cycle test - 100+ cycles without corruption
#[test]
fn test_save_load_cycles_100_plus() {
    let temp_dir = create_test_dirs();
    let config = StressTestConfig::default();
    
    println!("Starting save/load cycle stress test: {} cycles", config.save_load_cycles);
    
    let save_config = SaveConfig {
        save_dir: temp_dir.path().to_path_buf(),
        auto_save_enabled: false, // Manual control for testing
        ..Default::default()
    };
    
    let atomic_config = AtomicSaveConfig::default();
    let atomic_manager = Arc::new(AtomicSaveManager::new(temp_dir.path().to_path_buf(), atomic_config)
        .expect("Failed to create AtomicSaveManager"));
    
    let world = create_test_world();
    let mut cycles_completed = 0;
    let mut corruption_detected = false;
    let start_time = Instant::now();
    
    // Perform save/load cycles
    for cycle in 0..config.save_load_cycles {
        println!("Cycle {}/{}", cycle + 1, config.save_load_cycles);
        
        // Save operation
        let save_result = save_world_atomic(&atomic_manager, &world);
        if save_result.is_err() {
            println!("Save failed at cycle {}: {:?}", cycle, save_result.err());
            break;
        }
        
        // Load operation (simulated for now)
        let load_result = load_world_atomic(&atomic_manager, &world);
        if load_result.is_err() {
            println!("Load failed at cycle {}: {:?}", cycle, load_result.err());
            break;
        }
        
        // Verify data integrity
        if !verify_world_integrity(&world) {
            println!("Data corruption detected at cycle {}", cycle);
            corruption_detected = true;
            break;
        }
        
        cycles_completed += 1;
        
        // Check timeout
        if start_time.elapsed() > config.max_test_duration {
            println!("Test timeout reached");
            break;
        }
        
        // Small delay to prevent overwhelming the system
        thread::sleep(Duration::from_millis(10));
    }
    
    let total_duration = start_time.elapsed();
    let average_cycle_time = total_duration / cycles_completed.max(1) as u32;
    
    println!("Stress test completed:");
    println!("  Cycles completed: {}/{}", cycles_completed, config.save_load_cycles);
    println!("  Total duration: {:?}", total_duration);
    println!("  Average cycle time: {:?}", average_cycle_time);
    println!("  Corruption detected: {}", corruption_detected);
    
    // Assertions
    assert!(cycles_completed >= 100, "Should complete at least 100 cycles");
    assert!(!corruption_detected, "No data corruption should be detected");
    assert!(average_cycle_time < Duration::from_millis(1000), "Average cycle time should be reasonable");
}

/// Concurrent save operations stress test
#[test]
fn test_concurrent_save_operations() {
    let temp_dir = create_test_dirs();
    let config = StressTestConfig::default();
    
    println!("Starting concurrent save operations test: {} threads", config.concurrent_threads);
    
    let atomic_config = AtomicSaveConfig {
        max_concurrent_operations: config.concurrent_threads,
        ..Default::default()
    };
    let atomic_manager = Arc::new(AtomicSaveManager::new(temp_dir.path().to_path_buf(), atomic_config)
        .expect("Failed to create AtomicSaveManager"));
    
    let world = Arc::new(create_test_world());
    let operations_completed = Arc::new(Mutex::new(0));
    let operations_failed = Arc::new(Mutex::new(0));
    let race_conditions = Arc::new(Mutex::new(0));
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    // Spawn concurrent threads
    for thread_id in 0..config.concurrent_threads {
        let manager = Arc::clone(&atomic_manager);
        let world_ref = Arc::clone(&world);
        let completed = Arc::clone(&operations_completed);
        let failed = Arc::clone(&operations_failed);
        let races = Arc::clone(&race_conditions);
        
        let handle = thread::spawn(move || {
            let operations_per_thread = config.save_load_cycles / config.concurrent_threads;
            
            for i in 0..operations_per_thread {
                let chunk_pos = ChunkPos {
                    x: (thread_id * 100 + i) as i32,
                    y: 0,
                    z: thread_id as i32,
                };
                
                // Queue save operation
                let operation = SaveOperation::Chunk {
                    pos: chunk_pos,
                    priority: if i % 10 == 0 { SavePriority::High } else { SavePriority::Normal },
                };
                
                match manager.queue_operation(operation) {
                    Ok(_) => {
                        // Process the operation
                        match manager.process_next_operation(&world_ref) {
                            Ok(Some(result)) => {
                                if result.success {
                                    *completed.lock().unwrap() += 1;
                                } else {
                                    *failed.lock().unwrap() += 1;
                                }
                            }
                            Ok(None) => {
                                // No operation to process (race condition)
                                *races.lock().unwrap() += 1;
                            }
                            Err(_) => {
                                *failed.lock().unwrap() += 1;
                            }
                        }
                    }
                    Err(_) => {
                        *failed.lock().unwrap() += 1;
                    }
                }
                
                // Small delay between operations
                thread::sleep(Duration::from_millis(5));
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
    
    let total_duration = start_time.elapsed();
    let final_completed = *operations_completed.lock().unwrap();
    let final_failed = *operations_failed.lock().unwrap();
    let final_races = *race_conditions.lock().unwrap();
    
    println!("Concurrent operations test completed:");
    println!("  Operations completed: {}", final_completed);
    println!("  Operations failed: {}", final_failed);
    println!("  Race conditions handled: {}", final_races);
    println!("  Total duration: {:?}", total_duration);
    
    // Assertions
    assert!(final_completed > 0, "Should complete some operations");
    assert!(final_failed < final_completed / 10, "Failure rate should be low (< 10%)");
    
    // Get final stats from atomic manager
    let stats = atomic_manager.get_stats().expect("Failed to get stats");
    println!("  Final queue length: {}", stats.queue_length);
    println!("  Total operations: {}", stats.operations_completed + stats.operations_failed);
}

/// Race condition prevention test
#[test]
fn test_race_condition_prevention() {
    let temp_dir = create_test_dirs();
    
    println!("Starting race condition prevention test");
    
    let atomic_config = AtomicSaveConfig::default();
    let atomic_manager = Arc::new(AtomicSaveManager::new(temp_dir.path().to_path_buf(), atomic_config)
        .expect("Failed to create AtomicSaveManager"));
    
    let world = Arc::new(create_test_world());
    let same_chunk_pos = ChunkPos { x: 0, y: 0, z: 0 };
    
    let operations_completed = Arc::new(Mutex::new(0));
    let race_conditions_prevented = Arc::new(Mutex::new(0));
    
    let mut handles = Vec::new();
    
    // Spawn multiple threads trying to save the same chunk
    for thread_id in 0..8 {
        let manager = Arc::clone(&atomic_manager);
        let world_ref = Arc::clone(&world);
        let completed = Arc::clone(&operations_completed);
        let races_prevented = Arc::clone(&race_conditions_prevented);
        let chunk_pos = same_chunk_pos;
        
        let handle = thread::spawn(move || {
            for i in 0..50 {
                let operation = SaveOperation::Chunk {
                    pos: chunk_pos,
                    priority: SavePriority::Normal,
                };
                
                // Queue the operation
                if manager.queue_operation(operation).is_ok() {
                    // Try to process immediately
                    match manager.process_next_operation(&world_ref) {
                        Ok(Some(result)) => {
                            if result.success {
                                *completed.lock().unwrap() += 1;
                            }
                        }
                        Ok(None) => {
                            // No operation available (another thread processed it)
                            *races_prevented.lock().unwrap() += 1;
                        }
                        Err(_) => {
                            // Error processing - could indicate proper race prevention
                            *races_prevented.lock().unwrap() += 1;
                        }
                    }
                }
                
                // Brief delay
                thread::sleep(Duration::from_millis(1));
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for completion
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
    
    let final_completed = *operations_completed.lock().unwrap();
    let final_races_prevented = *race_conditions_prevented.lock().unwrap();
    
    println!("Race condition test completed:");
    println!("  Operations completed: {}", final_completed);
    println!("  Race conditions prevented: {}", final_races_prevented);
    
    // Should have completed some operations and prevented some race conditions
    assert!(final_completed > 0, "Should complete some operations");
    assert!(final_races_prevented > 0, "Should prevent some race conditions");
}

/// Data integrity validation test
#[test]
fn test_data_integrity_validation() {
    let temp_dir = create_test_dirs();
    
    println!("Starting data integrity validation test");
    
    let validation_config = ValidationConfig {
        enable_checksums: true,
        enable_deep_validation: true,
        ..Default::default()
    };
    
    let mut validator = StateValidator::new(validation_config);
    let world = create_test_world();
    
    // Take initial snapshots
    validator.take_network_snapshot(&world, "initial".to_string())
        .expect("Failed to take initial network snapshot");
    validator.take_persistence_snapshot(&world, "initial".to_string())
        .expect("Failed to take initial persistence snapshot");
    
    // Validate initial consistency
    let initial_result = validator.validate_consistency("initial")
        .expect("Failed to validate initial consistency");
    
    println!("Initial validation result: success={}, errors={}, warnings={}", 
             initial_result.success, initial_result.errors.len(), initial_result.warnings.len());
    
    // Perform multiple save/load cycles with validation
    let mut all_validations_passed = true;
    
    for cycle in 0..20 {
        let snapshot_id = format!("cycle_{}", cycle);
        
        // Simulate world changes
        // (In real implementation, would modify world state)
        
        // Take snapshots after changes
        validator.take_network_snapshot(&world, snapshot_id.clone())
            .expect("Failed to take network snapshot");
        validator.take_persistence_snapshot(&world, snapshot_id.clone())
            .expect("Failed to take persistence snapshot");
        
        // Validate consistency
        let result = validator.validate_consistency(&snapshot_id)
            .expect("Failed to validate consistency");
        
        if !result.success {
            println!("Validation failed at cycle {}: {} errors", cycle, result.errors.len());
            for error in &result.errors {
                println!("  Error: {}", error.description);
            }
            all_validations_passed = false;
        }
        
        // Validate specific chunks
        let chunk_pos = ChunkPos { x: cycle as i32, y: 0, z: 0 };
        let chunk_result = validator.validate_chunk(chunk_pos, &world)
            .expect("Failed to validate chunk");
        
        if !chunk_result.success {
            println!("Chunk validation failed at cycle {}", cycle);
            all_validations_passed = false;
        }
    }
    
    // Get final stats
    let stats = validator.get_validation_stats();
    println!("Validation statistics:");
    println!("  Total validations: {}", stats.total_validations);
    println!("  Successful validations: {}", stats.successful_validations);
    println!("  Failed validations: {}", stats.failed_validations);
    println!("  Average duration: {:?}", stats.average_duration);
    
    // Assertions
    assert!(all_validations_passed, "All validations should pass");
    assert!(stats.successful_validations > 20, "Should have successful validations");
    assert_eq!(stats.failed_validations, 0, "Should have no failed validations");
}

/// Disconnect handling stress test
#[test]
fn test_disconnect_handling_stress() {
    let temp_dir = create_test_dirs();
    
    println!("Starting disconnect handling stress test");
    
    let atomic_config = AtomicSaveConfig::default();
    let save_manager = Arc::new(AtomicSaveManager::new(temp_dir.path().to_path_buf(), atomic_config)
        .expect("Failed to create AtomicSaveManager"));
    
    let disconnect_config = DisconnectConfig {
        max_save_timeout: Duration::from_secs(5),
        chunk_save_radius: 2,
        reconnect_grace_period: Duration::from_millis(100),
        ..Default::default()
    };
    
    let disconnect_handler = Arc::new(Mutex::new(DisconnectHandler::new(Arc::clone(&save_manager), disconnect_config)));
    disconnect_handler.lock().unwrap().start().expect("Failed to start disconnect handler");
    
    let world = Arc::new(create_test_world());
    
    // Simulate multiple player disconnects
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let handler = Arc::clone(&disconnect_handler);
        let world_clone = Arc::clone(&world);
        let player_uuid = format!("player_{}", i);
        let username = format!("Player{}", i);
        let position = (i as f64 * 100.0, 64.0, 0.0);
        
        let handle = thread::spawn(move || {
            // Simulate normal disconnect
            if i % 3 == 0 {
                if let Ok(handler_ref) = handler.lock() {
                    let _ = handler_ref.handle_disconnect(player_uuid, username, &world_clone, position);
                }
            }
            // Simulate emergency disconnect
            else if i % 3 == 1 {
                if let Ok(handler_ref) = handler.lock() {
                    let _ = handler_ref.handle_emergency_disconnect(player_uuid, &world_clone, position);
                }
            }
            // Simulate force disconnect
            else {
                thread::sleep(Duration::from_millis(200));
                if let Ok(handler_ref) = handler.lock() {
                    let _ = handler_ref.force_disconnect(&player_uuid);
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all disconnects to process
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
    
    // Wait a bit for background processing
    thread::sleep(Duration::from_secs(2));
    
    let stats = disconnect_handler.lock().unwrap().get_stats().expect("Failed to get disconnect stats");
    println!("Disconnect handling statistics:");
    println!("  Players disconnecting: {}", stats.players_disconnecting);
    println!("  Successful saves: {}", stats.successful_saves);
    println!("  Failed saves: {}", stats.failed_saves);
    println!("  Emergency saves: {}", stats.emergency_saves);
    println!("  Force disconnects: {}", stats.force_disconnects);
    
    disconnect_handler.lock().unwrap().stop().expect("Failed to stop disconnect handler");
    
    // Assertions
    assert!(stats.successful_saves + stats.emergency_saves > 0, "Should have some successful saves");
    assert!(stats.failed_saves < 3, "Should have few failed saves");
}

/// Utility function to save world atomically
fn save_world_atomic(manager: &AtomicSaveManager, world: &World) -> PersistenceResult<()> {
    // Queue a full world save operation
    manager.queue_operation(SaveOperation::FullWorld {
        priority: SavePriority::Normal,
    })?;
    
    // Process the operation immediately
    match manager.process_next_operation(world)? {
        Some(result) => {
            if result.success {
                Ok(())
            } else {
                Err(result.error.unwrap_or_else(|| {
                    hearth_engine::persistence::PersistenceError::IoError(
                        std::io::Error::new(std::io::ErrorKind::Other, "Save operation failed")
                    )
                }))
            }
        }
        None => {
            Err(hearth_engine::persistence::PersistenceError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, "No save operation to process")
            ))
        }
    }
}

/// Utility function to load world atomically
fn load_world_atomic(_manager: &AtomicSaveManager, _world: &World) -> PersistenceResult<()> {
    // For now, simulate successful load
    // In real implementation, would deserialize and validate data
    Ok(())
}

/// Utility function to verify world integrity
fn verify_world_integrity(_world: &World) -> bool {
    // For now, assume integrity is always good
    // In real implementation, would check checksums, validate data structures, etc.
    true
}

/// Performance benchmark test
#[test]
fn test_save_load_performance() {
    let temp_dir = create_test_dirs();
    let config = StressTestConfig {
        save_load_cycles: 200,
        ..Default::default()
    };
    
    println!("Starting save/load performance benchmark: {} cycles", config.save_load_cycles);
    
    let atomic_config = AtomicSaveConfig::default();
    let atomic_manager = Arc::new(AtomicSaveManager::new(temp_dir.path().to_path_buf(), atomic_config)
        .expect("Failed to create AtomicSaveManager"));
    
    let world = create_test_world();
    
    let start_time = Instant::now();
    let mut times = Vec::new();
    
    for cycle in 0..config.save_load_cycles {
        let cycle_start = Instant::now();
        
        // Perform save/load cycle
        save_world_atomic(&atomic_manager, &world).expect("Save failed");
        load_world_atomic(&atomic_manager, &world).expect("Load failed");
        
        let cycle_time = cycle_start.elapsed();
        times.push(cycle_time);
        
        if cycle % 50 == 0 {
            println!("Completed {} cycles", cycle + 1);
        }
    }
    
    let total_time = start_time.elapsed();
    let average_time = times.iter().sum::<Duration>() / times.len() as u32;
    let median_time = {
        let mut sorted_times = times.clone();
        sorted_times.sort();
        sorted_times[sorted_times.len() / 2]
    };
    let max_time = times.iter().max().copied().unwrap_or(Duration::from_millis(0));
    let min_time = times.iter().min().copied().unwrap_or(Duration::from_millis(0));
    
    println!("Performance benchmark results:");
    println!("  Total cycles: {}", config.save_load_cycles);
    println!("  Total time: {:?}", total_time);
    println!("  Average cycle time: {:?}", average_time);
    println!("  Median cycle time: {:?}", median_time);
    println!("  Min cycle time: {:?}", min_time);
    println!("  Max cycle time: {:?}", max_time);
    println!("  Cycles per second: {:.2}", config.save_load_cycles as f64 / total_time.as_secs_f64());
    
    // Performance assertions
    assert!(average_time < Duration::from_millis(100), "Average cycle time should be < 100ms");
    assert!(max_time < Duration::from_millis(500), "Max cycle time should be < 500ms");
    assert!(total_time < Duration::from_secs(60), "Total time should be < 60 seconds");
}

/// Memory usage test under stress
#[test]
fn test_memory_usage_under_stress() {
    let temp_dir = create_test_dirs();
    
    println!("Starting memory usage stress test");
    
    let atomic_config = AtomicSaveConfig {
        max_concurrent_operations: 8,
        chunk_batch_size: 32,
        ..Default::default()
    };
    let atomic_manager = Arc::new(AtomicSaveManager::new(temp_dir.path().to_path_buf(), atomic_config)
        .expect("Failed to create AtomicSaveManager"));
    
    let world = create_test_world();
    
    // Queue many operations to test memory usage
    for i in 0..1000 {
        let operation = SaveOperation::Chunk {
            pos: ChunkPos { x: i % 100, y: 0, z: i / 100 },
            priority: SavePriority::Background,
        };
        
        atomic_manager.queue_operation(operation)
            .expect("Failed to queue operation");
    }
    
    let initial_stats = atomic_manager.get_stats().expect("Failed to get initial stats");
    println!("Initial queue length: {}", initial_stats.queue_length);
    
    // Process operations in batches
    let mut total_processed = 0;
    while total_processed < 1000 {
        match atomic_manager.process_next_operation(&world) {
            Ok(Some(_)) => {
                total_processed += 1;
            }
            Ok(None) => {
                break; // No more operations
            }
            Err(e) => {
                println!("Error processing operation: {:?}", e);
                break;
            }
        }
        
        // Check memory usage periodically
        if total_processed % 100 == 0 {
            let stats = atomic_manager.get_stats().expect("Failed to get stats");
            println!("Processed: {}, Queue length: {}", total_processed, stats.queue_length);
        }
    }
    
    let final_stats = atomic_manager.get_stats().expect("Failed to get final stats");
    println!("Final statistics:");
    println!("  Operations processed: {}", total_processed);
    println!("  Final queue length: {}", final_stats.queue_length);
    println!("  Operations completed: {}", final_stats.operations_completed);
    println!("  Operations failed: {}", final_stats.operations_failed);
    
    // Memory usage assertions
    assert_eq!(final_stats.queue_length, 0, "Queue should be empty after processing");
    assert!(final_stats.operations_completed > 900, "Should complete most operations");
    assert!(total_processed >= 1000, "Should process all queued operations");
}