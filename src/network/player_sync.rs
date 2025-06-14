//! Player state synchronization between network and persistence systems
//!
//! This module provides a bridge between the network system and persistence system
//! for player data synchronization. It ensures that player state is consistently
//! maintained across network operations and save/load cycles.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Serialize, Deserialize};
use glam::{Vec3, Quat};

use crate::persistence::{
    PlayerSaveData, PlayerData,
    PersistenceResult, PersistenceError,
};
use crate::persistence::player_data::{GameMode, PlayerStats};
use crate::network::{Packet, ServerPacket, ClientPacket, MovementState};
use crate::inventory::PlayerInventoryData;

/// Player state synchronization events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerSyncEvent {
    /// Player data has been loaded from disk
    PlayerLoaded {
        player_id: u32,
        player_data: PlayerData,
    },
    /// Player data needs to be saved
    PlayerSaveRequested {
        player_id: u32,
        urgent: bool,
    },
    /// Player data has been saved successfully
    PlayerSaved {
        player_id: u32,
        save_time: u64,
    },
    /// Player position/state has been updated
    PlayerUpdated {
        player_id: u32,
        position: Vec3,
        rotation: Quat,
        health: f32,
    },
    /// Player disconnected, save required
    PlayerDisconnected {
        player_id: u32,
        save_data: PlayerSaveData,
    },
}

/// Current player state for synchronization
#[derive(Debug, Clone)]
pub struct PlayerSyncState {
    pub player_id: u32,
    pub uuid: String,
    pub username: String,
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub health: f32,
    pub hunger: f32,
    pub movement_state: MovementState,
    pub game_mode: GameMode,
    pub last_update: u64,
    pub last_save: u64,
    pub dirty: bool,
    pub inventory_dirty: bool,
}

/// Player state bridge for network/persistence synchronization
#[derive(Debug)]
pub struct PlayerSyncBridge {
    /// Currently active player states
    player_states: Arc<Mutex<HashMap<u32, PlayerSyncState>>>,
    /// Pending sync events
    sync_events: Arc<Mutex<Vec<PlayerSyncEvent>>>,
    /// Player ID to UUID mapping
    player_id_to_uuid: Arc<Mutex<HashMap<u32, String>>>,
    /// Configuration
    config: PlayerSyncConfig,
}

/// Configuration for player synchronization
#[derive(Debug, Clone)]
pub struct PlayerSyncConfig {
    /// How often to check for save operations
    pub save_check_interval: Duration,
    /// Maximum time between forced saves
    pub max_save_interval: Duration,
    /// Position change threshold for marking dirty
    pub position_threshold: f32,
    /// Health change threshold for marking dirty
    pub health_threshold: f32,
    /// Auto-save on disconnect
    pub auto_save_on_disconnect: bool,
}

impl Default for PlayerSyncConfig {
    fn default() -> Self {
        Self {
            save_check_interval: Duration::from_secs(30),
            max_save_interval: Duration::from_secs(300), // 5 minutes
            position_threshold: 1.0, // 1 meter
            health_threshold: 1.0,
            auto_save_on_disconnect: true,
        }
    }
}

impl PlayerSyncBridge {
    /// Create a new player sync bridge
    pub fn new(config: PlayerSyncConfig) -> Self {
        Self {
            player_states: Arc::new(Mutex::new(HashMap::new())),
            sync_events: Arc::new(Mutex::new(Vec::new())),
            player_id_to_uuid: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Register a player with the sync bridge
    pub fn register_player(&self, player_id: u32, uuid: String, username: String, 
                          initial_data: Option<PlayerData>) -> PersistenceResult<()> {
        let mut states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;
        let mut id_mapping = self.player_id_to_uuid.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_id_to_uuid lock poisoned".to_string()))?;

        // Create initial state
        let (position, rotation, health, hunger, game_mode) = if let Some(data) = initial_data {
            (data.position, data.rotation, data.health, data.hunger, data.game_mode)
        } else {
            (Vec3::new(0.0, 100.0, 0.0), Quat::IDENTITY, 20.0, 20.0, GameMode::Survival)
        };

        let state = PlayerSyncState {
            player_id,
            uuid: uuid.clone(),
            username,
            position,
            rotation,
            velocity: Vec3::ZERO,
            health,
            hunger,
            movement_state: MovementState::Normal,
            game_mode,
            last_update: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            last_save: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            dirty: false,
            inventory_dirty: false,
        };

        states.insert(player_id, state);
        id_mapping.insert(player_id, uuid);

        Ok(())
    }

    /// Unregister a player from the sync bridge
    pub fn unregister_player(&self, player_id: u32) -> PersistenceResult<Option<PlayerSaveData>> {
        let mut states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;
        let mut id_mapping = self.player_id_to_uuid.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_id_to_uuid lock poisoned".to_string()))?;

        if let Some(state) = states.remove(&player_id) {
            id_mapping.remove(&player_id);

            // Create save data if needed
            if state.dirty || state.inventory_dirty {
                let player_data = PlayerData {
                    uuid: state.uuid.clone(),
                    username: state.username.clone(),
                    position: state.position,
                    rotation: state.rotation,
                    health: state.health,
                    hunger: state.hunger,
                    experience: 0, // TODO: Track experience
                    level: 0,
                    game_mode: state.game_mode,
                    spawn_position: None, // TODO: Track spawn position
                    last_login: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    play_time: 0, // TODO: Track play time
                    stats: PlayerStats::default(), // TODO: Track stats
                };

                // TODO: Get actual inventory data
                let inventory = PlayerInventoryData::default();
                
                let save_data = PlayerSaveData::from_player(player_data, &inventory);
                
                // Queue save event
                self.queue_sync_event(PlayerSyncEvent::PlayerDisconnected {
                    player_id,
                    save_data: save_data.clone(),
                })?;

                return Ok(Some(save_data));
            }
        }

        Ok(None)
    }

    /// Update player state from network packet
    pub fn update_from_network(&self, player_id: u32, packet: &ClientPacket) -> PersistenceResult<()> {
        let mut states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;

        if let Some(state) = states.get_mut(&player_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            match packet {
                ClientPacket::PlayerInput { position, rotation, velocity, movement_state, .. } => {
                    // Check if position changed significantly
                    let position_delta = (*position - state.position).length();
                    if position_delta > self.config.position_threshold {
                        state.dirty = true;
                    }

                    // Update state
                    state.position = *position;
                    state.rotation = *rotation;
                    state.velocity = *velocity;
                    state.movement_state = *movement_state;
                    state.last_update = now;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Update player state from persistence data
    pub fn update_from_persistence(&self, player_id: u32, save_data: &PlayerSaveData) -> PersistenceResult<()> {
        let mut states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;

        if let Some(state) = states.get_mut(&player_id) {
            // Update from loaded data
            state.position = save_data.player_data.position;
            state.rotation = save_data.player_data.rotation;
            state.health = save_data.player_data.health;
            state.hunger = save_data.player_data.hunger;
            state.game_mode = save_data.player_data.game_mode;
            state.last_save = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            state.dirty = false;
            state.inventory_dirty = false;

            // Queue load event
            self.queue_sync_event(PlayerSyncEvent::PlayerLoaded {
                player_id,
                player_data: save_data.player_data.clone(),
            })?;
        }

        Ok(())
    }

    /// Get network packets for player state synchronization
    pub fn get_network_updates(&self) -> PersistenceResult<Vec<Packet>> {
        let states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut packets = Vec::new();

        for (player_id, state) in states.iter() {
            // Send player updates for significant changes
            if state.dirty {
                packets.push(Packet::Server(ServerPacket::PlayerUpdate {
                    player_id: *player_id,
                    position: state.position,
                    rotation: state.rotation,
                    velocity: state.velocity,
                    movement_state: state.movement_state,
                    timestamp: now.saturating_sub(state.last_update) * 1000, // Convert to milliseconds
                }));
            }
        }

        Ok(packets)
    }

    /// Get players that need saving
    pub fn get_players_needing_save(&self) -> PersistenceResult<Vec<u32>> {
        let states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut players_to_save = Vec::new();

        for (player_id, state) in states.iter() {
            let time_since_save = now.saturating_sub(state.last_save);
            let needs_save = state.dirty || state.inventory_dirty ||
                time_since_save > self.config.max_save_interval.as_secs();

            if needs_save {
                players_to_save.push(*player_id);
            }
        }

        Ok(players_to_save)
    }

    /// Create save data for a player
    pub fn create_save_data(&self, player_id: u32, inventory: &PlayerInventoryData) -> PersistenceResult<Option<PlayerSaveData>> {
        let states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;

        if let Some(state) = states.get(&player_id) {
            let player_data = PlayerData {
                uuid: state.uuid.clone(),
                username: state.username.clone(),
                position: state.position,
                rotation: state.rotation,
                health: state.health,
                hunger: state.hunger,
                experience: 0, // TODO: Track experience
                level: 0,
                game_mode: state.game_mode,
                spawn_position: None, // TODO: Track spawn position
                last_login: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                play_time: 0, // TODO: Track play time
                stats: PlayerStats::default(), // TODO: Track stats
            };

            let save_data = PlayerSaveData::from_player(player_data, inventory);
            return Ok(Some(save_data));
        }

        Ok(None)
    }

    /// Mark player as saved
    pub fn mark_player_saved(&self, player_id: u32) -> PersistenceResult<()> {
        let mut states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;

        if let Some(state) = states.get_mut(&player_id) {
            state.dirty = false;
            state.inventory_dirty = false;
            state.last_save = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Queue save event
            self.queue_sync_event(PlayerSyncEvent::PlayerSaved {
                player_id,
                save_time: state.last_save,
            })?;
        }

        Ok(())
    }

    /// Queue a sync event
    fn queue_sync_event(&self, event: PlayerSyncEvent) -> PersistenceResult<()> {
        let mut events = self.sync_events.lock()
            .map_err(|_| PersistenceError::LockPoisoned("sync_events lock poisoned".to_string()))?;
        events.push(event);
        Ok(())
    }

    /// Get and clear pending sync events
    pub fn get_sync_events(&self) -> PersistenceResult<Vec<PlayerSyncEvent>> {
        let mut events = self.sync_events.lock()
            .map_err(|_| PersistenceError::LockPoisoned("sync_events lock poisoned".to_string()))?;
        Ok(std::mem::take(&mut *events))
    }

    /// Get player state for debugging
    pub fn get_player_state(&self, player_id: u32) -> PersistenceResult<Option<PlayerSyncState>> {
        let states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;
        Ok(states.get(&player_id).cloned())
    }

    /// Get synchronization statistics
    pub fn get_sync_stats(&self) -> PersistenceResult<PlayerSyncStats> {
        let states = self.player_states.lock()
            .map_err(|_| PersistenceError::LockPoisoned("player_states lock poisoned".to_string()))?;
        let events = self.sync_events.lock()
            .map_err(|_| PersistenceError::LockPoisoned("sync_events lock poisoned".to_string()))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut dirty_players = 0;
        let mut players_needing_save = 0;

        for state in states.values() {
            if state.dirty || state.inventory_dirty {
                dirty_players += 1;
            }
            if now.saturating_sub(state.last_save) > self.config.max_save_interval.as_secs() {
                players_needing_save += 1;
            }
        }

        Ok(PlayerSyncStats {
            total_players: states.len(),
            dirty_players,
            players_needing_save,
            pending_events: events.len(),
        })
    }
}

/// Statistics for player synchronization
#[derive(Debug, Clone)]
pub struct PlayerSyncStats {
    pub total_players: usize,
    pub dirty_players: usize,
    pub players_needing_save: usize,
    pub pending_events: usize,
}

/// Player synchronization manager
pub struct PlayerSyncManager {
    bridge: PlayerSyncBridge,
}

impl PlayerSyncManager {
    /// Create a new player sync manager
    pub fn new(config: PlayerSyncConfig) -> Self {
        Self {
            bridge: PlayerSyncBridge::new(config),
        }
    }

    /// Get the sync bridge
    pub fn bridge(&self) -> &PlayerSyncBridge {
        &self.bridge
    }

    /// Process player synchronization for a tick
    pub fn process_tick(&self) -> PersistenceResult<PlayerSyncTickResult> {
        // Get network updates
        let network_packets = self.bridge.get_network_updates()?;
        
        // Get players needing save
        let players_to_save = self.bridge.get_players_needing_save()?;
        
        // Get sync events
        let sync_events = self.bridge.get_sync_events()?;
        
        // Get stats
        let stats = self.bridge.get_sync_stats()?;

        Ok(PlayerSyncTickResult {
            network_packets,
            players_to_save,
            sync_events,
            stats,
        })
    }
}

/// Result of a player sync tick
#[derive(Debug)]
pub struct PlayerSyncTickResult {
    pub network_packets: Vec<Packet>,
    pub players_to_save: Vec<u32>,
    pub sync_events: Vec<PlayerSyncEvent>,
    pub stats: PlayerSyncStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_player_sync_bridge_creation() {
        let config = PlayerSyncConfig::default();
        let bridge = PlayerSyncBridge::new(config);
        
        let stats = bridge.get_sync_stats().expect("Failed to get sync stats");
        assert_eq!(stats.total_players, 0);
        assert_eq!(stats.dirty_players, 0);
        assert_eq!(stats.pending_events, 0);
    }

    #[test]
    fn test_player_registration() {
        let config = PlayerSyncConfig::default();
        let bridge = PlayerSyncBridge::new(config);
        
        bridge.register_player(1, "test-uuid".to_string(), "TestPlayer".to_string(), None)
            .expect("Failed to register player");
        
        let stats = bridge.get_sync_stats().expect("Failed to get sync stats");
        assert_eq!(stats.total_players, 1);
        
        let state = bridge.get_player_state(1).expect("Failed to get player state");
        assert!(state.is_some());
        let state = state.unwrap();
        assert_eq!(state.uuid, "test-uuid");
        assert_eq!(state.username, "TestPlayer");
    }

    #[test]
    fn test_player_unregistration() {
        let config = PlayerSyncConfig::default();
        let bridge = PlayerSyncBridge::new(config);
        
        bridge.register_player(1, "test-uuid".to_string(), "TestPlayer".to_string(), None)
            .expect("Failed to register player");
        
        let save_data = bridge.unregister_player(1).expect("Failed to unregister player");
        assert!(save_data.is_none()); // No dirty data, so no save data
        
        let stats = bridge.get_sync_stats().expect("Failed to get sync stats");
        assert_eq!(stats.total_players, 0);
    }

    #[test]
    fn test_player_update_from_network() {
        let config = PlayerSyncConfig::default();
        let bridge = PlayerSyncBridge::new(config);
        
        bridge.register_player(1, "test-uuid".to_string(), "TestPlayer".to_string(), None)
            .expect("Failed to register player");
        
        let packet = ClientPacket::PlayerInput {
            position: Vec3::new(10.0, 20.0, 30.0),
            rotation: Quat::IDENTITY,
            velocity: Vec3::new(1.0, 0.0, 0.0),
            movement_state: MovementState::Sprinting,
            sequence: 1,
        };
        
        bridge.update_from_network(1, &packet).expect("Failed to update from network");
        
        let state = bridge.get_player_state(1).expect("Failed to get player state").unwrap();
        assert_eq!(state.position, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(state.movement_state, MovementState::Sprinting);
        assert!(state.dirty); // Should be marked dirty due to position change
    }

    #[test]
    fn test_sync_manager() {
        let config = PlayerSyncConfig::default();
        let manager = PlayerSyncManager::new(config);
        
        manager.bridge().register_player(1, "test-uuid".to_string(), "TestPlayer".to_string(), None)
            .expect("Failed to register player");
        
        let result = manager.process_tick().expect("Failed to process tick");
        assert_eq!(result.stats.total_players, 1);
        assert_eq!(result.network_packets.len(), 0); // No updates initially
        assert_eq!(result.players_to_save.len(), 0); // No saves needed initially
    }
}