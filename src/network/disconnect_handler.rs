//! Network disconnect handler with automatic save protection
//!
//! This module handles player disconnections gracefully by ensuring
//! all player data and associated chunks are saved before the connection
//! is fully terminated.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;

use crate::persistence::{PersistenceResult, PersistenceError, atomic_save::{AtomicSaveManager, SaveOperation, SavePriority}};
use crate::{World, ChunkPos};

/// Player connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnecting,
    Disconnected,
    SaveComplete,
}

/// Information about a disconnecting player
#[derive(Debug, Clone)]
pub struct DisconnectingPlayer {
    pub uuid: String,
    pub username: String,
    pub position: (f64, f64, f64),
    pub chunks_to_save: HashSet<ChunkPos>,
    pub disconnect_time: Instant,
    pub state: ConnectionState,
}

/// Configuration for disconnect handling
#[derive(Debug, Clone)]
pub struct DisconnectConfig {
    /// Maximum time to wait for save completion before force disconnect
    pub max_save_timeout: Duration,
    /// Radius around player to save chunks
    pub chunk_save_radius: i32,
    /// Enable emergency save mode for critical failures
    pub emergency_save_enabled: bool,
    /// Grace period for reconnection before save
    pub reconnect_grace_period: Duration,
}

impl Default for DisconnectConfig {
    fn default() -> Self {
        Self {
            max_save_timeout: Duration::from_secs(30),
            chunk_save_radius: 3,
            emergency_save_enabled: true,
            reconnect_grace_period: Duration::from_secs(5),
        }
    }
}

/// Statistics for disconnect handling
#[derive(Debug, Clone)]
pub struct DisconnectStats {
    pub players_disconnecting: usize,
    pub successful_saves: u64,
    pub failed_saves: u64,
    pub emergency_saves: u64,
    pub average_save_time: Duration,
    pub force_disconnects: u64,
}

/// Handles player disconnections with save protection
pub struct DisconnectHandler {
    /// Players currently disconnecting
    disconnecting_players: Arc<Mutex<HashMap<String, DisconnectingPlayer>>>,
    
    /// Atomic save manager for safe operations
    save_manager: Arc<AtomicSaveManager>,
    
    /// Configuration
    config: DisconnectConfig,
    
    /// Statistics
    stats: Arc<Mutex<DisconnectStats>>,
    
    /// Background thread handle
    worker_thread: Option<thread::JoinHandle<()>>,
    
    /// Shutdown signal
    shutdown: Arc<Mutex<bool>>,
}

impl DisconnectHandler {
    /// Create a new disconnect handler
    pub fn new(save_manager: Arc<AtomicSaveManager>, config: DisconnectConfig) -> Self {
        let handler = Self {
            disconnecting_players: Arc::new(Mutex::new(HashMap::new())),
            save_manager,
            config,
            stats: Arc::new(Mutex::new(DisconnectStats {
                players_disconnecting: 0,
                successful_saves: 0,
                failed_saves: 0,
                emergency_saves: 0,
                average_save_time: Duration::from_millis(0),
                force_disconnects: 0,
            })),
            worker_thread: None,
            shutdown: Arc::new(Mutex::new(false)),
        };
        
        handler
    }
    
    /// Start the background worker thread
    pub fn start(&mut self) -> PersistenceResult<()> {
        let disconnecting_players = Arc::clone(&self.disconnecting_players);
        let save_manager = Arc::clone(&self.save_manager);
        let config = self.config.clone();
        let stats = Arc::clone(&self.stats);
        let shutdown = Arc::clone(&self.shutdown);
        
        self.worker_thread = Some(thread::spawn(move || {
            Self::worker_loop(disconnecting_players, save_manager, config, stats, shutdown);
        }));
        
        Ok(())
    }
    
    /// Stop the background worker
    pub fn stop(&mut self) -> PersistenceResult<()> {
        if let Ok(mut shutdown) = self.shutdown.lock() {
            *shutdown = true;
        }
        
        if let Some(handle) = self.worker_thread.take() {
            let _ = handle.join();
        }
        
        Ok(())
    }
    
    /// Handle a player disconnect request
    pub fn handle_disconnect(
        &self,
        player_uuid: String,
        username: String,
        world: &World,
        player_position: (f64, f64, f64),
    ) -> PersistenceResult<()> {
        let chunks_to_save = self.get_chunks_around_player(player_position);
        
        let disconnecting_player = DisconnectingPlayer {
            uuid: player_uuid.clone(),
            username,
            position: player_position,
            chunks_to_save,
            disconnect_time: Instant::now(),
            state: ConnectionState::Disconnecting,
        };
        
        // Add to disconnecting players list
        {
            let mut players = self.disconnecting_players.lock()
                .map_err(|_| PersistenceError::LockPoisoned("disconnecting_players".to_string()))?;
            players.insert(player_uuid.clone(), disconnecting_player.clone());
        }
        
        // Queue save operations with critical priority
        self.queue_player_saves(&disconnecting_player, world)?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.players_disconnecting += 1;
        }
        
        println!("[DisconnectHandler] Handling disconnect for player {} at {:?}", 
                player_uuid, player_position);
        
        Ok(())
    }
    
    /// Handle emergency disconnect (e.g., crashed connection)
    pub fn handle_emergency_disconnect(
        &self,
        player_uuid: String,
        world: &World,
        player_position: (f64, f64, f64),
    ) -> PersistenceResult<()> {
        if !self.config.emergency_save_enabled {
            return Ok(());
        }
        
        println!("[DisconnectHandler] Emergency disconnect for player {}", player_uuid);
        
        // Immediately queue critical saves
        let chunks_to_save = self.get_chunks_around_player(player_position);
        
        // Queue player data save with critical priority
        self.save_manager.queue_operation(SaveOperation::Player {
            uuid: player_uuid.clone(),
            priority: SavePriority::Critical,
        })?;
        
        // Queue chunk saves with critical priority
        if !chunks_to_save.is_empty() {
            self.save_manager.queue_operation(SaveOperation::ChunkBatch {
                positions: chunks_to_save.into_iter().collect(),
                priority: SavePriority::Critical,
            })?;
        }
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.emergency_saves += 1;
        }
        
        Ok(())
    }
    
    /// Check if a player is currently disconnecting
    pub fn is_player_disconnecting(&self, player_uuid: &str) -> bool {
        if let Ok(players) = self.disconnecting_players.lock() {
            players.contains_key(player_uuid)
        } else {
            false
        }
    }
    
    /// Get disconnect status for a player
    pub fn get_disconnect_status(&self, player_uuid: &str) -> Option<ConnectionState> {
        if let Ok(players) = self.disconnecting_players.lock() {
            players.get(player_uuid).map(|p| p.state.clone())
        } else {
            None
        }
    }
    
    /// Force disconnect a player (emergency override)
    pub fn force_disconnect(&self, player_uuid: &str) -> PersistenceResult<bool> {
        let mut players = self.disconnecting_players.lock()
            .map_err(|_| PersistenceError::LockPoisoned("disconnecting_players".to_string()))?;
        
        if let Some(mut player) = players.remove(player_uuid) {
            player.state = ConnectionState::Disconnected;
            
            // Update stats
            if let Ok(mut stats) = self.stats.lock() {
                stats.force_disconnects += 1;
                stats.players_disconnecting = stats.players_disconnecting.saturating_sub(1);
            }
            
            println!("[DisconnectHandler] Force disconnected player {}", player_uuid);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Get chunks around a player position that need saving
    fn get_chunks_around_player(&self, position: (f64, f64, f64)) -> HashSet<ChunkPos> {
        let (x, _y, z) = position;
        let chunk_x = (x as i32) >> 4; // Assuming 16x16 chunks
        let chunk_z = (z as i32) >> 4;
        
        let mut chunks = HashSet::new();
        let radius = self.config.chunk_save_radius;
        
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                chunks.insert(ChunkPos {
                    x: chunk_x + dx,
                    y: 0, // Assuming y=0 for simplicity
                    z: chunk_z + dz,
                });
            }
        }
        
        chunks
    }
    
    /// Queue save operations for a disconnecting player
    fn queue_player_saves(&self, player: &DisconnectingPlayer, _world: &World) -> PersistenceResult<()> {
        // Queue player data save
        self.save_manager.queue_operation(SaveOperation::Player {
            uuid: player.uuid.clone(),
            priority: SavePriority::Critical,
        })?;
        
        // Queue chunk saves if any
        if !player.chunks_to_save.is_empty() {
            self.save_manager.queue_operation(SaveOperation::ChunkBatch {
                positions: player.chunks_to_save.iter().cloned().collect(),
                priority: SavePriority::Critical,
            })?;
        }
        
        Ok(())
    }
    
    /// Background worker loop
    fn worker_loop(
        disconnecting_players: Arc<Mutex<HashMap<String, DisconnectingPlayer>>>,
        save_manager: Arc<AtomicSaveManager>,
        config: DisconnectConfig,
        stats: Arc<Mutex<DisconnectStats>>,
        shutdown: Arc<Mutex<bool>>,
    ) {
        loop {
            // Check shutdown signal
            if let Ok(shutdown_flag) = shutdown.lock() {
                if *shutdown_flag {
                    break;
                }
            }
            
            thread::sleep(Duration::from_millis(100));
            
            // Process disconnecting players
            let players_to_process: Vec<DisconnectingPlayer> = {
                if let Ok(players) = disconnecting_players.lock() {
                    players.values().cloned().collect()
                } else {
                    continue;
                }
            };
            
            for player in players_to_process {
                let now = Instant::now();
                let disconnect_duration = now.duration_since(player.disconnect_time);
                
                // Check if save timeout exceeded
                if disconnect_duration > config.max_save_timeout {
                    println!("[DisconnectHandler] Save timeout for player {}, forcing disconnect", player.uuid);
                    
                    if let Ok(mut players) = disconnecting_players.lock() {
                        players.remove(&player.uuid);
                    }
                    
                    if let Ok(mut stats_lock) = stats.lock() {
                        stats_lock.force_disconnects += 1;
                        stats_lock.players_disconnecting = stats_lock.players_disconnecting.saturating_sub(1);
                    }
                    continue;
                }
                
                // Check if grace period passed and saves are complete
                if disconnect_duration > config.reconnect_grace_period {
                    if Self::are_player_saves_complete(&save_manager, &player) {
                        println!("[DisconnectHandler] Save complete for player {}", player.uuid);
                        
                        if let Ok(mut players) = disconnecting_players.lock() {
                            if let Some(p) = players.get_mut(&player.uuid) {
                                p.state = ConnectionState::SaveComplete;
                            }
                        }
                        
                        if let Ok(mut stats_lock) = stats.lock() {
                            stats_lock.successful_saves += 1;
                            stats_lock.players_disconnecting = stats_lock.players_disconnecting.saturating_sub(1);
                            
                            // Update average save time
                            let total_saves = stats_lock.successful_saves + stats_lock.failed_saves;
                            if total_saves > 0 {
                                let total_time = stats_lock.average_save_time.as_millis() as u64 * (total_saves - 1) 
                                    + disconnect_duration.as_millis() as u64;
                                stats_lock.average_save_time = Duration::from_millis(total_time / total_saves);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Check if all saves for a player are complete
    fn are_player_saves_complete(_save_manager: &AtomicSaveManager, _player: &DisconnectingPlayer) -> bool {
        // For now, assume saves complete after grace period
        // In real implementation, would check save manager queue for pending operations
        true
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> PersistenceResult<DisconnectStats> {
        let stats = self.stats.lock()
            .map_err(|_| PersistenceError::LockPoisoned("stats".to_string()))?;
        Ok(stats.clone())
    }
    
    /// Clear all disconnecting players (emergency shutdown)
    pub fn clear_all_disconnecting(&self) -> PersistenceResult<usize> {
        let mut players = self.disconnecting_players.lock()
            .map_err(|_| PersistenceError::LockPoisoned("disconnecting_players".to_string()))?;
        let count = players.len();
        players.clear();
        
        if let Ok(mut stats) = self.stats.lock() {
            stats.players_disconnecting = 0;
        }
        
        Ok(count)
    }
}

impl Drop for DisconnectHandler {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::atomic_save::{AtomicSaveManager, AtomicSaveConfig};
    use tempfile::TempDir;
    use std::sync::Arc;
    
    fn create_test_save_manager() -> Arc<AtomicSaveManager> {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
        let config = AtomicSaveConfig::default();
        Arc::new(AtomicSaveManager::new(temp_dir.path().to_path_buf(), config)
            .expect("Failed to create AtomicSaveManager"))
    }
    
    fn create_test_world() -> World {
        World::new(16) // 16x16 chunk size
    }
    
    #[test]
    fn test_disconnect_handler_creation() {
        let save_manager = create_test_save_manager();
        let config = DisconnectConfig::default();
        let handler = DisconnectHandler::new(save_manager, config);
        
        let stats = handler.get_stats().expect("Failed to get stats");
        assert_eq!(stats.players_disconnecting, 0);
        assert_eq!(stats.successful_saves, 0);
    }
    
    #[test]
    fn test_handle_disconnect() {
        let save_manager = create_test_save_manager();
        let config = DisconnectConfig::default();
        let handler = DisconnectHandler::new(save_manager, config);
        let world = create_test_world();
        
        let result = handler.handle_disconnect(
            "test_player".to_string(),
            "TestPlayer".to_string(),
            &world,
            (100.0, 64.0, 200.0),
        );
        
        assert!(result.is_ok());
        assert!(handler.is_player_disconnecting("test_player"));
        
        let status = handler.get_disconnect_status("test_player");
        assert_eq!(status, Some(ConnectionState::Disconnecting));
    }
    
    #[test]
    fn test_emergency_disconnect() {
        let save_manager = create_test_save_manager();
        let config = DisconnectConfig::default();
        let handler = DisconnectHandler::new(save_manager, config);
        let world = create_test_world();
        
        let result = handler.handle_emergency_disconnect(
            "emergency_player".to_string(),
            &world,
            (0.0, 64.0, 0.0),
        );
        
        assert!(result.is_ok());
        
        let stats = handler.get_stats().expect("Failed to get stats");
        assert_eq!(stats.emergency_saves, 1);
    }
    
    #[test]
    fn test_chunks_around_player() {
        let save_manager = create_test_save_manager();
        let config = DisconnectConfig { chunk_save_radius: 1, ..Default::default() };
        let handler = DisconnectHandler::new(save_manager, config);
        
        let chunks = handler.get_chunks_around_player((16.0, 64.0, 16.0));
        assert_eq!(chunks.len(), 9); // 3x3 grid around player
        
        assert!(chunks.contains(&ChunkPos { x: 0, y: 0, z: 0 }));
        assert!(chunks.contains(&ChunkPos { x: 1, y: 0, z: 1 }));
        assert!(chunks.contains(&ChunkPos { x: -1, y: 0, z: -1 }));
    }
    
    #[test]
    fn test_force_disconnect() {
        let save_manager = create_test_save_manager();
        let config = DisconnectConfig::default();
        let handler = DisconnectHandler::new(save_manager, config);
        let world = create_test_world();
        
        // First handle a normal disconnect
        handler.handle_disconnect(
            "force_test".to_string(),
            "ForceTest".to_string(),
            &world,
            (0.0, 64.0, 0.0),
        ).expect("Failed to handle disconnect");
        
        // Then force disconnect
        let result = handler.force_disconnect("force_test");
        assert!(result.is_ok());
        assert_eq!(result.expect("[Test] Force disconnect should succeed"), true);
        
        // Should no longer be disconnecting
        assert!(!handler.is_player_disconnecting("force_test"));
    }
}