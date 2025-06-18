//! Compatibility wrapper for DOP player data
//!
//! This module provides API compatibility with existing PlayerData and PlayerSyncState
//! while using the optimized DOP data structures internally for cache efficiency.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use glam::{Vec3, Quat};
use serde::{Serialize, Deserialize};

use crate::persistence::{PersistenceResult, PersistenceError};
use crate::network::{MovementState, Packet, ServerPacket};
use crate::persistence::player_data::{GameMode, PlayerStats, PlayerInfo};
use crate::persistence::player_data_dop::{
    PlayerDataBuffer, PlayerHotData, PlayerColdData, PlayerStatsData, PotionEffectData,
    DIRTY_POSITION, DIRTY_VELOCITY, DIRTY_ROTATION, DIRTY_HEALTH, DIRTY_HUNGER, 
    DIRTY_EXPERIENCE, DIRTY_LEVEL, DIRTY_ALL
};
use crate::inventory::PlayerInventoryData;

/// Cache-optimized player data manager
pub struct DOPPlayerDataManager {
    /// Main player data buffer (SOA layout)
    player_buffer: PlayerDataBuffer,
    /// Player ID to buffer index mapping
    id_to_index: HashMap<u32, usize>,
    /// Next available player ID
    next_player_id: u32,
    /// Performance metrics
    metrics: PlayerDataMetrics,
}

/// Performance tracking metrics
#[derive(Debug, Default)]
pub struct PlayerDataMetrics {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hot_data_accesses: u64,
    pub cold_data_accesses: u64,
    pub physics_updates: u64,
    pub network_updates: u64,
}

/// Enhanced player state with DOP optimizations
#[derive(Debug, Clone)]
pub struct DOPPlayerSyncState {
    pub player_id: u32,
    pub buffer_index: usize,
    pub last_update: u64,
    pub last_save: u64,
    pub network_dirty: bool,
    pub persistence_dirty: bool,
}

impl DOPPlayerDataManager {
    /// Create a new cache-optimized player data manager
    pub fn new(max_players: usize) -> Self {
        Self {
            player_buffer: PlayerDataBuffer::new(max_players),
            id_to_index: HashMap::with_capacity(max_players),
            next_player_id: 1,
            metrics: PlayerDataMetrics::default(),
        }
    }
    
    /// Register a new player (maintains compatibility with existing API)
    pub fn register_player(&mut self, uuid: String, username: String) -> PersistenceResult<u32> {
        let player_id = self.next_player_id;
        self.next_player_id += 1;
        
        let hot_data = PlayerHotData::default();
        let cold_data = PlayerColdData {
            uuid,
            username,
            spawn_position: None,
            last_login: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            play_time: 0,
            stats: PlayerStatsData::default(),
            effects: Vec::new(),
            achievements: Vec::new(),
            tags: Vec::new(),
        };
        
        match self.player_buffer.add_player(player_id, hot_data, cold_data) {
            Some(index) => {
                self.id_to_index.insert(player_id, index);
                Ok(player_id)
            }
            None => Err(PersistenceError::CapacityExceeded("Player buffer full".to_string()))
        }
    }
    
    /// Unregister a player
    pub fn unregister_player(&mut self, player_id: u32) -> PersistenceResult<()> {
        if let Some(index) = self.id_to_index.remove(&player_id) {
            self.player_buffer.remove_player(index);
        }
        Ok(())
    }
    
    /// Update player position (cache-optimized)
    pub fn update_position(&mut self, player_id: u32, position: Vec3) -> PersistenceResult<()> {
        if let Some(&index) = self.id_to_index.get(&player_id) {
            self.player_buffer.update_position(index, position);
            self.metrics.hot_data_accesses += 1;
            Ok(())
        } else {
            self.metrics.cache_misses += 1;
            Err(PersistenceError::PlayerNotFound(player_id.to_string()))
        }
    }
    
    /// Update player velocity
    pub fn update_velocity(&mut self, player_id: u32, velocity: Vec3) -> PersistenceResult<()> {
        if let Some(&index) = self.id_to_index.get(&player_id) {
            self.player_buffer.update_velocity(index, velocity);
            self.metrics.hot_data_accesses += 1;
            Ok(())
        } else {
            self.metrics.cache_misses += 1;
            Err(PersistenceError::PlayerNotFound(player_id.to_string()))
        }
    }
    
    /// Update player rotation
    pub fn update_rotation(&mut self, player_id: u32, rotation: Quat) -> PersistenceResult<()> {
        if let Some(&index) = self.id_to_index.get(&player_id) {
            // Update rotation components in SOA layout
            if index < self.player_buffer.count {
                self.player_buffer.rotation_x[index] = rotation.x;
                self.player_buffer.rotation_y[index] = rotation.y;
                self.player_buffer.rotation_z[index] = rotation.z;
                self.player_buffer.rotation_w[index] = rotation.w;
                self.player_buffer.dirty_flags[index] |= DIRTY_ROTATION;
            }
            self.metrics.hot_data_accesses += 1;
            Ok(())
        } else {
            self.metrics.cache_misses += 1;
            Err(PersistenceError::PlayerNotFound(player_id.to_string()))
        }
    }
    
    /// Update player health
    pub fn update_health(&mut self, player_id: u32, health: f32) -> PersistenceResult<()> {
        if let Some(&index) = self.id_to_index.get(&player_id) {
            self.player_buffer.update_health(index, health);
            self.metrics.hot_data_accesses += 1;
            Ok(())
        } else {
            self.metrics.cache_misses += 1;
            Err(PersistenceError::PlayerNotFound(player_id.to_string()))
        }
    }
    
    /// Get player hot data (frequently accessed properties)
    pub fn get_hot_data(&self, player_id: u32) -> PersistenceResult<PlayerHotData> {
        if let Some(&index) = self.id_to_index.get(&player_id) {
            if let Some(data) = self.player_buffer.get_hot_data(index) {
                Ok(data)
            } else {
                Err(PersistenceError::PlayerNotFound(player_id.to_string()))
            }
        } else {
            Err(PersistenceError::PlayerNotFound(player_id.to_string()))
        }
    }
    
    /// Get player cold data (infrequently accessed metadata)
    pub fn get_cold_data(&self, player_id: u32) -> PersistenceResult<&PlayerColdData> {
        if let Some(data) = self.player_buffer.get_cold_data(player_id) {
            Ok(data)
        } else {
            Err(PersistenceError::PlayerNotFound(player_id.to_string()))
        }
    }
    
    /// Batch physics update for all players (highly cache-efficient)
    pub fn update_physics(&mut self, dt: f32) {
        self.player_buffer.update_physics(dt);
        self.metrics.physics_updates += 1;
    }
    
    /// Get players that need network synchronization
    pub fn get_network_dirty_players(&self) -> Vec<u32> {
        let dirty_indices = self.player_buffer.get_dirty_players(
            DIRTY_POSITION | DIRTY_VELOCITY | DIRTY_ROTATION | DIRTY_HEALTH
        );
        
        dirty_indices.into_iter()
            .filter_map(|index| {
                if index < self.player_buffer.player_ids.len() {
                    let player_id = self.player_buffer.player_ids[index];
                    if player_id != u32::MAX {
                        Some(player_id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Clear network dirty flags for a player
    pub fn clear_network_dirty(&mut self, player_id: u32) {
        if let Some(&index) = self.id_to_index.get(&player_id) {
            self.player_buffer.clear_dirty_flags(
                index, 
                DIRTY_POSITION | DIRTY_VELOCITY | DIRTY_ROTATION | DIRTY_HEALTH
            );
        }
    }
    
    /// Generate network packets for player updates
    pub fn generate_network_packets(&self) -> Vec<Packet> {
        let mut packets = Vec::new();
        let dirty_players = self.get_network_dirty_players();
        
        for player_id in dirty_players {
            if let Ok(hot_data) = self.get_hot_data(player_id) {
                packets.push(Packet::Server(ServerPacket::PlayerUpdate {
                    player_id,
                    position: hot_data.position,
                    rotation: hot_data.rotation,
                    velocity: hot_data.velocity,
                    movement_state: MovementState::from_u8(hot_data.movement_state),
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                }));
            }
        }
        
        packets
    }
    
    /// Convert to legacy PlayerData format for persistence
    pub fn to_legacy_player_data(&self, player_id: u32) -> PersistenceResult<crate::persistence::player_data::PlayerData> {
        let hot_data = self.get_hot_data(player_id)?;
        let cold_data = self.get_cold_data(player_id)?;
        
        Ok(crate::persistence::player_data::PlayerData {
            uuid: cold_data.uuid.clone(),
            username: cold_data.username.clone(),
            position: hot_data.position,
            rotation: hot_data.rotation,
            health: hot_data.health,
            hunger: hot_data.hunger,
            experience: hot_data.experience,
            level: hot_data.level,
            game_mode: GameMode::from_u8(hot_data.game_mode),
            spawn_position: cold_data.spawn_position,
            last_login: cold_data.last_login,
            play_time: cold_data.play_time,
            stats: PlayerStats::from_dop_stats(&cold_data.stats),
        })
    }
    
    /// Load from legacy PlayerData format
    pub fn from_legacy_player_data(&mut self, data: &crate::persistence::player_data::PlayerData) -> PersistenceResult<u32> {
        let player_id = self.next_player_id;
        self.next_player_id += 1;
        
        let hot_data = PlayerHotData {
            position: data.position,
            velocity: Vec3::ZERO, // Not stored in legacy format
            rotation: data.rotation,
            health: data.health,
            hunger: data.hunger,
            experience: data.experience,
            level: data.level,
            game_mode: data.game_mode.to_u8(),
            movement_state: 0,
            dirty_flags: 0,
            _padding: [0; 61],
        };
        
        let cold_data = PlayerColdData {
            uuid: data.uuid.clone(),
            username: data.username.clone(),
            spawn_position: data.spawn_position,
            last_login: data.last_login,
            play_time: data.play_time,
            stats: PlayerStatsData::from_legacy_stats(&data.stats),
            effects: Vec::new(), // Convert if needed
            achievements: Vec::new(),
            tags: Vec::new(),
        };
        
        match self.player_buffer.add_player(player_id, hot_data, cold_data) {
            Some(index) => {
                self.id_to_index.insert(player_id, index);
                Ok(player_id)
            }
            None => Err(PersistenceError::CapacityExceeded("Player buffer full".to_string()))
        }
    }
    
    /// Get performance metrics
    pub fn get_metrics(&self) -> &PlayerDataMetrics {
        &self.metrics
    }
    
    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> crate::persistence::player_data_dop::PlayerBufferMemoryStats {
        self.player_buffer.memory_usage()
    }
    
    /// Get active player count
    pub fn active_player_count(&self) -> usize {
        self.id_to_index.len()
    }
    
    /// List all active players
    pub fn list_players(&self) -> Vec<u32> {
        self.id_to_index.keys().copied().collect()
    }
}

// Extension traits for type conversions
impl GameMode {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => GameMode::Survival,
            1 => GameMode::Creative,
            2 => GameMode::Adventure,
            3 => GameMode::Spectator,
            _ => GameMode::Survival,
        }
    }
    
    fn to_u8(&self) -> u8 {
        match self {
            GameMode::Survival => 0,
            GameMode::Creative => 1,
            GameMode::Adventure => 2,
            GameMode::Spectator => 3,
        }
    }
}

impl MovementState {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => MovementState::Normal,
            1 => MovementState::Sprinting,
            2 => MovementState::Flying,
            3 => MovementState::Swimming,
            4 => MovementState::Crouching,
            _ => MovementState::Normal,
        }
    }
}

impl PlayerStats {
    fn from_dop_stats(stats: &PlayerStatsData) -> Self {
        Self {
            blocks_broken: stats.blocks_broken,
            blocks_placed: stats.blocks_placed,
            distance_walked: stats.distance_walked,
            distance_sprinted: stats.distance_sprinted,
            distance_fallen: stats.distance_fallen,
            distance_climbed: stats.distance_climbed,
            distance_flown: stats.distance_flown,
            jumps: stats.jumps,
            deaths: stats.deaths,
            mob_kills: stats.mob_kills,
            player_kills: stats.player_kills,
            damage_dealt: stats.damage_dealt,
            damage_taken: stats.damage_taken,
            play_time: stats.play_time,
        }
    }
}

impl PlayerStatsData {
    fn from_legacy_stats(stats: &PlayerStats) -> Self {
        Self {
            blocks_broken: stats.blocks_broken,
            blocks_placed: stats.blocks_placed,
            distance_walked: stats.distance_walked,
            distance_sprinted: stats.distance_sprinted,
            distance_fallen: stats.distance_fallen,
            distance_climbed: stats.distance_climbed,
            distance_flown: stats.distance_flown,
            jumps: stats.jumps,
            deaths: stats.deaths,
            mob_kills: stats.mob_kills,
            player_kills: stats.player_kills,
            damage_dealt: stats.damage_dealt,
            damage_taken: stats.damage_taken,
            play_time: stats.play_time,
        }
    }
}

/// Performance benchmarking for DOP vs legacy implementations
pub struct PlayerDataBenchmark {
    dop_manager: DOPPlayerDataManager,
    iteration_count: usize,
    player_count: usize,
}

impl PlayerDataBenchmark {
    pub fn new(player_count: usize, iteration_count: usize) -> Self {
        Self {
            dop_manager: DOPPlayerDataManager::new(player_count),
            iteration_count,
            player_count,
        }
    }
    
    /// Benchmark physics update performance
    pub fn benchmark_physics_update(&mut self) -> Duration {
        // Setup players
        for i in 0..self.player_count {
            let _ = self.dop_manager.register_player(
                format!("uuid-{}", i),
                format!("Player{}", i),
            );
        }
        
        // Benchmark physics updates
        let start = std::time::Instant::now();
        for _ in 0..self.iteration_count {
            self.dop_manager.update_physics(0.016); // 60 FPS
        }
        start.elapsed()
    }
    
    /// Benchmark network packet generation  
    pub fn benchmark_network_updates(&mut self) -> Duration {
        // Mark some existing players as dirty
        let player_ids = self.dop_manager.list_players();
        for (i, &player_id) in player_ids.iter().enumerate() {
            if i % 2 == 0 {
                let _ = self.dop_manager.update_position(player_id, Vec3::new(i as f32, 0.0, 0.0));
            }
        }
        
        // Benchmark network packet generation
        let start = std::time::Instant::now();
        for _ in 0..self.iteration_count {
            let _ = self.dop_manager.generate_network_packets();
        }
        start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dop_manager_creation() {
        let manager = DOPPlayerDataManager::new(100);
        assert_eq!(manager.active_player_count(), 0);
    }
    
    #[test]
    fn test_player_registration() {
        let mut manager = DOPPlayerDataManager::new(100);
        
        let player_id = manager.register_player(
            "test-uuid".to_string(),
            "TestPlayer".to_string(),
        ).expect("Failed to register player");
        
        assert_eq!(manager.active_player_count(), 1);
        assert!(manager.get_hot_data(player_id).is_ok());
    }
    
    #[test]
    fn test_position_update() {
        let mut manager = DOPPlayerDataManager::new(100);
        
        let player_id = manager.register_player(
            "test-uuid".to_string(),
            "TestPlayer".to_string(),
        ).unwrap();
        
        let new_position = Vec3::new(10.0, 20.0, 30.0);
        manager.update_position(player_id, new_position).unwrap();
        
        let hot_data = manager.get_hot_data(player_id).unwrap();
        assert_eq!(hot_data.position, new_position);
    }
    
    #[test]
    fn test_physics_update() {
        let mut manager = DOPPlayerDataManager::new(100);
        
        let player_id = manager.register_player(
            "test-uuid".to_string(),
            "TestPlayer".to_string(),
        ).unwrap();
        
        manager.update_velocity(player_id, Vec3::new(1.0, 2.0, 3.0)).unwrap();
        manager.update_physics(0.1);
        
        let hot_data = manager.get_hot_data(player_id).unwrap();
        assert_eq!(hot_data.position, Vec3::new(0.1, 100.2, 0.3));
    }
    
    #[test]
    fn test_benchmark() {
        let mut benchmark = PlayerDataBenchmark::new(100, 10);
        let physics_time = benchmark.benchmark_physics_update();
        let network_time = benchmark.benchmark_network_updates();
        
        println!("Physics update time: {:?}", physics_time);
        println!("Network update time: {:?}", network_time);
        
        // Verify benchmarks completed
        assert!(physics_time > Duration::from_nanos(1));
        assert!(network_time > Duration::from_nanos(1));
    }
}