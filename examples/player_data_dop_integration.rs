//! DANGER MONEY Player Data DOP Integration Example
//!
//! This example demonstrates how the new DOP player data structures maintain
//! full API compatibility with existing code while providing significant
//! performance improvements through cache-optimized data layouts.

use std::time::{Duration, Instant};
use glam::{Vec3, Quat};

use hearth_engine::persistence::{
    PlayerData, PlayerSaveData, DOPPlayerDataManager, PlayerDataMetrics,
    GameMode, PlayerStats, PersistenceResult
};
use hearth_engine::persistence::player_data_dop::{PlayerHotData, PlayerColdData};
use hearth_engine::network::{MovementState, ClientPacket};
use hearth_engine::inventory::PlayerInventoryData;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 DANGER MONEY Player Data DOP Integration Example");
    println!("===================================================");
    println!("Demonstrating backward compatibility and performance improvements\n");
    
    // Create cache-optimized player manager
    let mut player_manager = DOPPlayerDataManager::new(1000);
    
    // Example 1: Player Registration (maintains existing API)
    demo_player_registration(&mut player_manager)?;
    
    // Example 2: Physics Updates (highly optimized)
    demo_physics_updates(&mut player_manager)?;
    
    // Example 3: Network Synchronization (cache-efficient)
    demo_network_sync(&mut player_manager)?;
    
    // Example 4: Persistence Operations (compatible with existing saves)
    demo_persistence_operations(&mut player_manager)?;
    
    // Example 5: Performance Monitoring
    demo_performance_monitoring(&player_manager)?;
    
    // Example 6: Memory Optimization Analysis
    demo_memory_analysis(&player_manager)?;
    
    println!("\n🏆 Integration Example Complete!");
    println!("The DOP implementation provides:");
    println!("• Full API compatibility with existing code");
    println!("• Significant performance improvements");
    println!("• Better cache utilization and memory efficiency");
    println!("• Seamless integration with current persistence system");
    
    Ok(())
}

fn demo_player_registration(manager: &mut DOPPlayerDataManager) -> PersistenceResult<()> {
    println!("1️⃣ Player Registration Demo");
    println!("---------------------------");
    
    // Register players just like before
    let player1 = manager.register_player(
        "player-uuid-001".to_string(),
        "Alice".to_string(),
    )?;
    
    let player2 = manager.register_player(
        "player-uuid-002".to_string(),
        "Bob".to_string(),
    )?;
    
    let player3 = manager.register_player(
        "player-uuid-003".to_string(),
        "Charlie".to_string(),
    )?;
    
    println!("✅ Registered {} players", manager.active_player_count());
    println!("   Player IDs: {}, {}, {}", player1, player2, player3);
    
    // Set initial positions
    manager.update_position(player1, Vec3::new(10.0, 100.0, 20.0))?;
    manager.update_position(player2, Vec3::new(-5.0, 100.0, 15.0))?;
    manager.update_position(player3, Vec3::new(0.0, 110.0, 0.0))?;
    
    // Set velocities
    manager.update_velocity(player1, Vec3::new(2.0, 0.0, 1.0))?;
    manager.update_velocity(player2, Vec3::new(-1.0, 0.0, 0.5))?;
    manager.update_velocity(player3, Vec3::new(0.0, 0.0, -1.5))?;
    
    println!("✅ Set initial positions and velocities");
    
    Ok(())
}

fn demo_physics_updates(manager: &mut DOPPlayerDataManager) -> PersistenceResult<()> {
    println!("\n2️⃣ Physics Updates Demo");
    println!("----------------------");
    
    let start_time = Instant::now();
    
    // Simulate 1000 physics ticks (very fast due to SOA layout)
    for frame in 0..1000 {
        manager.update_physics(0.016); // 60 FPS
        
        // Print progress every 200 frames
        if frame % 200 == 0 {
            let player_ids = manager.list_players();
            if let Some(&first_player) = player_ids.first() {
                let hot_data = manager.get_hot_data(first_player)?;
                println!("   Frame {}: Player position = {:?}", frame, hot_data.position);
            }
        }
    }
    
    let physics_time = start_time.elapsed();
    println!("✅ Completed 1000 physics updates in {:?}", physics_time);
    println!("   Average time per frame: {:?}", physics_time / 1000);
    
    Ok(())
}

fn demo_network_sync(manager: &mut DOPPlayerDataManager) -> PersistenceResult<()> {
    println!("\n3️⃣ Network Synchronization Demo");
    println!("-------------------------------");
    
    let player_ids = manager.list_players();
    
    // Simulate network input updates (marks players as dirty)
    for (i, &player_id) in player_ids.iter().enumerate() {
        // Simulate player movement input
        let new_position = Vec3::new(i as f32 * 5.0, 100.0, i as f32 * 3.0);
        let new_velocity = Vec3::new((i as f32 - 1.0) * 2.0, 0.0, 1.0);
        
        manager.update_position(player_id, new_position)?;
        manager.update_velocity(player_id, new_velocity)?;
    }
    
    // Generate network packets (only for dirty players)
    let packets = manager.generate_network_packets();
    println!("✅ Generated {} network packets for dirty players", packets.len());
    
    // Clear dirty flags (simulating packet send)
    for &player_id in &player_ids {
        manager.clear_network_dirty(player_id);
    }
    
    // Verify no more packets needed
    let packets_after_clear = manager.generate_network_packets();
    println!("✅ After clearing dirty flags: {} packets needed", packets_after_clear.len());
    
    Ok(())
}

fn demo_persistence_operations(manager: &mut DOPPlayerDataManager) -> PersistenceResult<()> {
    println!("\n4️⃣ Persistence Operations Demo");
    println!("------------------------------");
    
    let player_ids = manager.list_players();
    
    // Convert to legacy format for saving (maintains compatibility)
    for &player_id in &player_ids {
        let legacy_data = manager.to_legacy_player_data(player_id)?;
        println!("✅ Converted player {} to legacy format", legacy_data.username);
        println!("   UUID: {}", legacy_data.uuid);
        println!("   Position: {:?}", legacy_data.position);
        println!("   Health: {}", legacy_data.health);
    }
    
    // Simulate loading from legacy format
    let legacy_player = PlayerData {
        uuid: "legacy-player-uuid".to_string(),
        username: "LegacyPlayer".to_string(),
        position: Vec3::new(50.0, 120.0, -10.0),
        rotation: Quat::from_rotation_y(45.0f32.to_radians()),
        health: 18.5,
        hunger: 16.0,
        experience: 1250,
        level: 5,
        game_mode: GameMode::Creative,
        spawn_position: Some(Vec3::new(0.0, 100.0, 0.0)),
        last_login: 1634567890,
        play_time: 3600,
        stats: PlayerStats::default(),
    };
    
    let loaded_player_id = manager.from_legacy_player_data(&legacy_player)?;
    println!("✅ Loaded legacy player data as player ID {}", loaded_player_id);
    
    // Verify the loaded data
    let hot_data = manager.get_hot_data(loaded_player_id)?;
    let cold_data = manager.get_cold_data(loaded_player_id)?;
    
    println!("   Loaded position: {:?}", hot_data.position);
    println!("   Loaded username: {}", cold_data.username);
    
    Ok(())
}

fn demo_performance_monitoring(manager: &DOPPlayerDataManager) -> PersistenceResult<()> {
    println!("\n5️⃣ Performance Monitoring Demo");
    println!("------------------------------");
    
    let metrics = manager.get_metrics();
    
    println!("📊 Performance Metrics:");
    println!("   Cache hits: {}", metrics.cache_hits);
    println!("   Cache misses: {}", metrics.cache_misses);
    println!("   Hot data accesses: {}", metrics.hot_data_accesses);
    println!("   Cold data accesses: {}", metrics.cold_data_accesses);
    println!("   Physics updates: {}", metrics.physics_updates);
    
    if metrics.cache_hits + metrics.cache_misses > 0 {
        let hit_rate = metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64;
        println!("   Cache hit rate: {:.1}%", hit_rate * 100.0);
    }
    
    if metrics.hot_data_accesses + metrics.cold_data_accesses > 0 {
        let hot_ratio = metrics.hot_data_accesses as f64 / (metrics.hot_data_accesses + metrics.cold_data_accesses) as f64;
        println!("   Hot data access ratio: {:.1}%", hot_ratio * 100.0);
    }
    
    Ok(())
}

fn demo_memory_analysis(manager: &DOPPlayerDataManager) -> PersistenceResult<()> {
    println!("\n6️⃣ Memory Analysis Demo");
    println!("----------------------");
    
    let memory_stats = manager.get_memory_stats();
    
    println!("💾 Memory Usage Analysis:");
    println!("   Active players: {}", memory_stats.active_players);
    println!("   Buffer capacity: {}", memory_stats.capacity);
    println!("   Hot data: {} bytes ({:.1} KB)", 
        memory_stats.hot_data_bytes, 
        memory_stats.hot_data_bytes as f64 / 1024.0);
    println!("   Cold data: {} bytes ({:.1} KB)", 
        memory_stats.cold_data_bytes,
        memory_stats.cold_data_bytes as f64 / 1024.0);
    println!("   Total: {} bytes ({:.1} KB)", 
        memory_stats.total_bytes,
        memory_stats.total_bytes as f64 / 1024.0);
    
    println!("\n📈 Efficiency Metrics:");
    println!("   Hot data ratio: {:.1}%", memory_stats.hot_data_ratio() * 100.0);
    println!("   Cache lines used: {}", memory_stats.cache_lines_used);
    println!("   Cache utilization: {:.1}%", memory_stats.cache_utilization() * 100.0);
    
    // Calculate per-player overhead
    if memory_stats.active_players > 0 {
        let hot_per_player = memory_stats.hot_data_bytes / memory_stats.active_players;
        let total_per_player = memory_stats.total_bytes / memory_stats.active_players;
        
        println!("   Hot data per player: {} bytes", hot_per_player);
        println!("   Total data per player: {} bytes", total_per_player);
    }
    
    // Compare with traditional AOS approach
    let aos_overhead = std::mem::size_of::<TraditionalPlayer>() * memory_stats.active_players;
    let memory_savings = if aos_overhead > memory_stats.total_bytes {
        aos_overhead - memory_stats.total_bytes
    } else {
        0
    };
    
    println!("\n💡 Compared to AOS approach:");
    println!("   Traditional AOS size: {} bytes", aos_overhead);
    println!("   DOP memory savings: {} bytes ({:.1}%)", 
        memory_savings,
        memory_savings as f64 / aos_overhead as f64 * 100.0);
    
    Ok(())
}

/// Traditional AOS player structure for comparison
#[derive(Clone)]
struct TraditionalPlayer {
    uuid: String,
    username: String,
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    health: f32,
    hunger: f32,
    experience: u32,
    level: u32,
    game_mode: GameMode,
    spawn_position: Option<Vec3>,
    last_login: u64,
    play_time: u64,
    stats: PlayerStats,
    // Additional fields that create cache inefficiency
    inventory: PlayerInventoryData,
    achievements: Vec<String>,
    effects: Vec<u32>,
}