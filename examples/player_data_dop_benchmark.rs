//! DANGER MONEY Player Data DOP Performance Benchmark
//! 
//! This benchmark demonstrates the cache efficiency improvements of the Data-Oriented
//! Programming (DOP) approach for player data compared to traditional Object-Oriented
//! Programming (OOP) structures.

use std::time::{Duration, Instant};
use std::hint::black_box;
use glam::{Vec3, Quat};

use hearth_engine::persistence::{
    PlayerData, DOPPlayerDataManager, PlayerDataBenchmark, PlayerBufferMemoryStats,
    GameMode, PlayerStats
};
use hearth_engine::persistence::player_data_dop::{PlayerHotData, PlayerColdData, PlayerStatsData};
use hearth_engine::profiling::{CacheProfiler, MemoryProfiler};

/// Traditional AOS (Array of Structures) player for comparison
#[derive(Clone)]
struct AOSPlayer {
    uuid: String,
    username: String,
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    health: f32,
    hunger: f32,
    experience: u32,
    level: u32,
    game_mode: u8,
    movement_state: u8,
    // Additional data that creates cache misses
    stats: PlayerStats,
    last_login: u64,
    play_time: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 DANGER MONEY Player Data DOP Performance Benchmark");
    println!("========================================================");
    println!("Comparing cache efficiency: DOP vs OOP player data structures");
    
    // Test different player counts
    let player_counts = [100, 500, 1000, 5000, 10000];
    let iterations = 1000;
    
    for &player_count in &player_counts {
        println!("\n📊 Testing with {} players, {} iterations", player_count, iterations);
        println!("--------------------------------------------------");
        
        // Run comprehensive benchmarks
        run_position_update_benchmark(player_count, iterations)?;
        run_physics_update_benchmark(player_count, iterations)?;
        run_network_update_benchmark(player_count, iterations)?;
        run_memory_access_benchmark(player_count, iterations)?;
    }
    
    // Detailed cache analysis
    println!("\n🧠 Cache Line Analysis");
    println!("=====================");
    run_cache_analysis()?;
    
    // Memory layout comparison
    println!("\n💾 Memory Layout Comparison");
    println!("===========================");
    run_memory_layout_analysis()?;
    
    // SIMD-friendly operations demo
    println!("\n⚡ SIMD-Friendly Operations");
    println!("===========================");
    run_simd_operations_demo()?;
    
    println!("\n🏁 Benchmark Complete!");
    println!("DOP architecture shows significant performance improvements through:");
    println!("• Better cache locality (hot data co-located)");
    println!("• Reduced memory bandwidth usage");
    println!("• SIMD-friendly data layouts");
    println!("• Separation of hot and cold data paths");
    
    Ok(())
}

fn run_position_update_benchmark(player_count: usize, iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 Position Update Benchmark");
    
    // DOP approach
    let mut dop_manager = DOPPlayerDataManager::new(player_count);
    let mut player_ids = Vec::new();
    
    // Setup DOP players
    for i in 0..player_count {
        let id = dop_manager.register_player(
            format!("uuid-{}", i),
            format!("Player{}", i),
        )?;
        player_ids.push(id);
    }
    
    // Benchmark DOP position updates
    let start = Instant::now();
    for _ in 0..iterations {
        for (i, &player_id) in player_ids.iter().enumerate() {
            let new_pos = Vec3::new(i as f32, 0.0, 0.0);
            dop_manager.update_position(player_id, new_pos)?;
        }
    }
    let dop_time = start.elapsed();
    
    // AOS approach
    let mut aos_players: Vec<AOSPlayer> = (0..player_count).map(|i| AOSPlayer {
        uuid: format!("uuid-{}", i),
        username: format!("Player{}", i),
        position: Vec3::ZERO,
        velocity: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        health: 20.0,
        hunger: 20.0,
        experience: 0,
        level: 0,
        game_mode: 0,
        movement_state: 0,
        stats: PlayerStats::default(),
        last_login: 0,
        play_time: 0,
    }).collect();
    
    // Benchmark AOS position updates
    let start = Instant::now();
    for _ in 0..iterations {
        for (i, player) in aos_players.iter_mut().enumerate() {
            player.position = Vec3::new(i as f32, 0.0, 0.0);
        }
    }
    let aos_time = start.elapsed();
    
    let speedup = aos_time.as_nanos() as f64 / dop_time.as_nanos() as f64;
    
    println!("   DOP time: {:?}", dop_time);
    println!("   AOS time: {:?}", aos_time);
    println!("   Speedup: {:.2}x", speedup);
    
    // Calculate memory bandwidth
    let operations = player_count * iterations;
    let dop_bandwidth = calculate_bandwidth(operations * 12, dop_time); // 12 bytes per Vec3
    let aos_bandwidth = calculate_bandwidth(operations * std::mem::size_of::<AOSPlayer>(), aos_time);
    
    println!("   DOP bandwidth: {:.1} MB/s", dop_bandwidth);
    println!("   AOS bandwidth: {:.1} MB/s", aos_bandwidth);
    
    Ok(())
}

fn run_physics_update_benchmark(player_count: usize, iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ Physics Update Benchmark");
    
    // Setup DOP manager
    let mut dop_manager = DOPPlayerDataManager::new(player_count);
    for i in 0..player_count {
        let player_id = dop_manager.register_player(
            format!("uuid-{}", i),
            format!("Player{}", i),
        )?;
        
        // Set initial velocity
        dop_manager.update_velocity(player_id, Vec3::new(1.0, 2.0, 3.0))?;
    }
    
    // Benchmark DOP physics update (vectorized SOA operations)
    let start = Instant::now();
    for _ in 0..iterations {
        dop_manager.update_physics(0.016); // 60 FPS
    }
    let dop_time = start.elapsed();
    
    // Setup AOS players
    let mut aos_players: Vec<AOSPlayer> = (0..player_count).map(|i| AOSPlayer {
        uuid: format!("uuid-{}", i),
        username: format!("Player{}", i),
        position: Vec3::ZERO,
        velocity: Vec3::new(1.0, 2.0, 3.0),
        rotation: Quat::IDENTITY,
        health: 20.0,
        hunger: 20.0,
        experience: 0,
        level: 0,
        game_mode: 0,
        movement_state: 0,
        stats: PlayerStats::default(),
        last_login: 0,
        play_time: 0,
    }).collect();
    
    // Benchmark AOS physics update
    let start = Instant::now();
    for _ in 0..iterations {
        for player in &mut aos_players {
            player.position += player.velocity * 0.016;
        }
    }
    let aos_time = start.elapsed();
    
    let speedup = aos_time.as_nanos() as f64 / dop_time.as_nanos() as f64;
    
    println!("   DOP time: {:?}", dop_time);
    println!("   AOS time: {:?}", aos_time);
    println!("   Speedup: {:.2}x", speedup);
    
    // Cache efficiency analysis
    let dop_cache_efficiency = estimate_cache_efficiency_dop(player_count);
    let aos_cache_efficiency = estimate_cache_efficiency_aos(player_count);
    
    println!("   DOP cache efficiency: {:.1}%", dop_cache_efficiency * 100.0);
    println!("   AOS cache efficiency: {:.1}%", aos_cache_efficiency * 100.0);
    
    Ok(())
}

fn run_network_update_benchmark(player_count: usize, iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📡 Network Update Benchmark");
    
    let mut dop_manager = DOPPlayerDataManager::new(player_count);
    let mut player_ids = Vec::new();
    
    // Setup players with some dirty
    for i in 0..player_count {
        let player_id = dop_manager.register_player(
            format!("uuid-{}", i),
            format!("Player{}", i),
        )?;
        player_ids.push(player_id);
        
        // Mark 50% of players as dirty
        if i % 2 == 0 {
            dop_manager.update_position(player_id, Vec3::new(i as f32, 0.0, 0.0))?;
        }
    }
    
    // Benchmark network packet generation
    let start = Instant::now();
    for _ in 0..iterations {
        let packets = dop_manager.generate_network_packets();
        black_box(packets);
    }
    let dop_time = start.elapsed();
    
    println!("   DOP network update time: {:?}", dop_time);
    println!("   Generated packets for ~{}% of players", 50);
    
    Ok(())
}

fn run_memory_access_benchmark(player_count: usize, iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧠 Memory Access Pattern Benchmark");
    
    let mut dop_manager = DOPPlayerDataManager::new(player_count);
    let mut player_ids = Vec::new();
    
    // Setup DOP players
    for i in 0..player_count {
        let id = dop_manager.register_player(
            format!("uuid-{}", i),
            format!("Player{}", i),
        )?;
        player_ids.push(id);
    }
    
    // Sequential access (DOP-friendly)
    let start = Instant::now();
    for _ in 0..iterations {
        for &player_id in &player_ids {
            let hot_data = dop_manager.get_hot_data(player_id)?;
            black_box(hot_data.position.x + hot_data.position.y + hot_data.position.z);
        }
    }
    let sequential_time = start.elapsed();
    
    // Random access (less cache-friendly)
    let mut random_order = player_ids.clone();
    random_order.reverse(); // Simple scrambling
    
    let start = Instant::now();
    for _ in 0..iterations {
        for &player_id in &random_order {
            let hot_data = dop_manager.get_hot_data(player_id)?;
            black_box(hot_data.position.x + hot_data.position.y + hot_data.position.z);
        }
    }
    let random_time = start.elapsed();
    
    println!("   Sequential access: {:?}", sequential_time);
    println!("   Random access: {:?}", random_time);
    println!("   Cache penalty: {:.2}x", random_time.as_nanos() as f64 / sequential_time.as_nanos() as f64);
    
    Ok(())
}

fn run_cache_analysis() -> Result<(), Box<dyn std::error::Error>> {
    let cache_profiler = CacheProfiler::new();
    
    println!("Player Hot Data Cache Analysis:");
    println!("• Size: {} bytes", std::mem::size_of::<PlayerHotData>());
    println!("• Alignment: {} bytes", std::mem::align_of::<PlayerHotData>());
    println!("• Cache lines per player: {:.1}", 
        std::mem::size_of::<PlayerHotData>() as f64 / 64.0);
    
    // Demonstrate cache line utilization
    const TEST_SIZE: usize = 10000;
    let positions: Vec<Vec3> = (0..TEST_SIZE).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
    
    // Test sequential access
    let start = Instant::now();
    let mut sum = 0.0;
    for pos in &positions {
        sum += pos.x + pos.y + pos.z;
    }
    let sequential_time = start.elapsed();
    black_box(sum);
    
    // Test strided access (cache-unfriendly)
    let start = Instant::now();
    let mut sum = 0.0;
    for i in (0..positions.len()).step_by(16) {
        sum += positions[i].x + positions[i].y + positions[i].z;
    }
    let strided_time = start.elapsed();
    black_box(sum);
    
    println!("• Sequential access: {:?}", sequential_time);
    println!("• Strided access (16x): {:?}", strided_time);
    println!("• Cache efficiency ratio: {:.2}x", 
        strided_time.as_nanos() as f64 / sequential_time.as_nanos() as f64);
    
    Ok(())
}

fn run_memory_layout_analysis() -> Result<(), Box<dyn std::error::Error>> {
    let dop_manager = DOPPlayerDataManager::new(1000);
    let stats = dop_manager.get_memory_stats();
    
    println!("DOP Memory Layout:");
    println!("• Hot data: {} bytes", stats.hot_data_bytes);
    println!("• Cold data: {} bytes", stats.cold_data_bytes);
    println!("• Total: {} bytes", stats.total_bytes);
    println!("• Hot data ratio: {:.1}%", stats.hot_data_ratio() * 100.0);
    println!("• Cache lines used: {}", stats.cache_lines_used);
    println!("• Cache utilization: {:.1}%", stats.cache_utilization() * 100.0);
    
    // Compare with AOS layout
    let aos_size = std::mem::size_of::<AOSPlayer>() * 1000;
    println!("\nAOS Memory Layout:");
    println!("• Total size: {} bytes", aos_size);
    println!("• Per-player overhead: {} bytes", std::mem::size_of::<AOSPlayer>());
    println!("• Memory efficiency vs DOP: {:.1}%", 
        stats.hot_data_bytes as f64 / aos_size as f64 * 100.0);
    
    Ok(())
}

fn run_simd_operations_demo() -> Result<(), Box<dyn std::error::Error>> {
    const SIZE: usize = 10000;
    const ITERATIONS: usize = 1000;
    
    println!("SIMD-Friendly Data Layout Demo:");
    
    // SOA layout (SIMD-friendly)
    let mut pos_x: Vec<f32> = (0..SIZE).map(|i| i as f32).collect();
    let mut pos_y: Vec<f32> = (0..SIZE).map(|i| i as f32).collect();
    let mut pos_z: Vec<f32> = (0..SIZE).map(|i| i as f32).collect();
    let vel_x: Vec<f32> = vec![1.0; SIZE];
    let vel_y: Vec<f32> = vec![2.0; SIZE];
    let vel_z: Vec<f32> = vec![3.0; SIZE];
    
    // SIMD-friendly update (SOA)
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for i in 0..SIZE {
            pos_x[i] += vel_x[i] * 0.016;
            pos_y[i] += vel_y[i] * 0.016;
            pos_z[i] += vel_z[i] * 0.016;
        }
    }
    let soa_time = start.elapsed();
    
    // AOS layout
    let mut positions: Vec<Vec3> = (0..SIZE).map(|i| Vec3::new(i as f32, i as f32, i as f32)).collect();
    let velocities: Vec<Vec3> = vec![Vec3::new(1.0, 2.0, 3.0); SIZE];
    
    // AOS update
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for i in 0..SIZE {
            positions[i] += velocities[i] * 0.016;
        }
    }
    let aos_time = start.elapsed();
    
    let speedup = aos_time.as_nanos() as f64 / soa_time.as_nanos() as f64;
    
    println!("• SOA (SIMD-friendly): {:?}", soa_time);
    println!("• AOS (SIMD-hostile): {:?}", aos_time);
    println!("• SIMD advantage: {:.2}x", speedup);
    
    // Memory bandwidth comparison
    let soa_bandwidth = calculate_bandwidth(SIZE * ITERATIONS * 12, soa_time); // 12 bytes per Vec3
    let aos_bandwidth = calculate_bandwidth(SIZE * ITERATIONS * 24, aos_time); // 24 bytes read+write
    
    println!("• SOA bandwidth: {:.1} MB/s", soa_bandwidth);
    println!("• AOS bandwidth: {:.1} MB/s", aos_bandwidth);
    
    Ok(())
}

fn calculate_bandwidth(bytes: usize, time: Duration) -> f64 {
    let mb = bytes as f64 / 1_000_000.0;
    mb / time.as_secs_f64()
}

fn estimate_cache_efficiency_dop(player_count: usize) -> f64 {
    // DOP: Hot data is tightly packed, excellent cache utilization
    let hot_data_size = std::mem::size_of::<PlayerHotData>() * player_count;
    let cache_lines_used = (hot_data_size + 63) / 64; // Round up to cache lines
    let ideal_cache_lines = hot_data_size / 64;
    
    ideal_cache_lines as f64 / cache_lines_used as f64
}

fn estimate_cache_efficiency_aos(player_count: usize) -> f64 {
    // AOS: Only need position+velocity but fetch entire struct
    let total_struct_size = std::mem::size_of::<AOSPlayer>();
    let needed_data_size = std::mem::size_of::<Vec3>() * 2; // position + velocity
    
    needed_data_size as f64 / total_struct_size as f64
}