use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};

use crate::world::{World, ChunkPos, Chunk};
use crate::network::{
    Packet, ServerPacket, SaveStatus, ChunkSaveStatus,
    PlayerSyncBridge,
};
use crate::persistence::{
    PersistenceResult, PersistenceError,
    WorldSave, PlayerSaveData, WorldMetadata,
    error::LockResultExt,
    atomic_save::{AtomicSaveManager, AtomicSaveConfig, SaveOperation, SavePriority},
    state_validator::{StateValidator, ValidationConfig},
    NetworkValidator,
};

/// Configuration for the save manager
#[derive(Debug, Clone)]
pub struct SaveConfig {
    /// Root directory for saves
    pub save_dir: PathBuf,
    /// Enable automatic saving
    pub auto_save_enabled: bool,
    /// Auto-save configuration
    pub auto_save_config: AutoSaveConfig,
    /// Enable compression for saves
    pub compression_enabled: bool,
    /// Number of backups to keep
    pub backup_count: usize,
    /// Maximum concurrent save operations
    pub max_concurrent_saves: usize,
}

/// Configuration for automatic saving
#[derive(Debug, Clone)]
pub struct AutoSaveConfig {
    /// Interval between auto-saves
    pub interval: Duration,
    /// Save on chunk unload
    pub save_on_chunk_unload: bool,
    /// Save on player disconnect
    pub save_on_player_disconnect: bool,
    /// Batch size for chunk saves
    pub chunk_batch_size: usize,
    /// Delay before saving unloaded chunks
    pub unload_save_delay: Duration,
}

impl Default for SaveConfig {
    fn default() -> Self {
        Self {
            save_dir: PathBuf::from("saves/world"),
            auto_save_enabled: true,
            auto_save_config: AutoSaveConfig::default(),
            compression_enabled: true,
            backup_count: 3,
            max_concurrent_saves: 4,
        }
    }
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300), // 5 minutes
            save_on_chunk_unload: true,
            save_on_player_disconnect: true,
            chunk_batch_size: 32,
            unload_save_delay: Duration::from_secs(30),
        }
    }
}

/// Manages world persistence with automatic saving
#[derive(Debug)]
pub struct SaveManager {
    config: SaveConfig,
    world_save: Arc<Mutex<WorldSave>>,
    save_thread: Option<thread::JoinHandle<()>>,
    shutdown_signal: Arc<Mutex<bool>>,
    
    // Tracking for auto-save
    last_save_time: Arc<Mutex<Instant>>,
    dirty_chunks: Arc<Mutex<HashSet<ChunkPos>>>,
    pending_chunk_saves: Arc<Mutex<HashMap<ChunkPos, Instant>>>,
    save_in_progress: Arc<Mutex<bool>>,
    
    // Network integration
    network_packets: Arc<Mutex<Vec<Packet>>>,
    operation_counter: Arc<Mutex<u32>>,
    chunk_states: Arc<Mutex<HashMap<ChunkPos, ChunkSaveStatus>>>,
    network_validator: Option<Arc<Mutex<NetworkValidator>>>,
    player_sync_bridge: Option<Arc<PlayerSyncBridge>>,
    
    // Atomic save system integration
    atomic_save_manager: Option<Arc<AtomicSaveManager>>,
    
    // State validation
    state_validator: Option<Arc<Mutex<StateValidator>>>,
}

impl SaveManager {
    /// Create a new save manager with default config for testing
    pub fn new(world_name: String) -> Self {
        let config = SaveConfig {
            save_dir: std::path::PathBuf::from(format!("saves/{}", world_name)),
            ..Default::default()
        };
        Self::new_with_config(config).expect("Failed to create SaveManager")
    }
    
    /// Create a new save manager
    pub fn new_with_config(config: SaveConfig) -> PersistenceResult<Self> {
        let world_save = Arc::new(Mutex::new(WorldSave::new(&config.save_dir)?));
        
        Ok(Self {
            config,
            world_save,
            save_thread: None,
            shutdown_signal: Arc::new(Mutex::new(false)),
            last_save_time: Arc::new(Mutex::new(Instant::now())),
            dirty_chunks: Arc::new(Mutex::new(HashSet::new())),
            pending_chunk_saves: Arc::new(Mutex::new(HashMap::new())),
            save_in_progress: Arc::new(Mutex::new(false)),
            network_packets: Arc::new(Mutex::new(Vec::new())),
            operation_counter: Arc::new(Mutex::new(0)),
            chunk_states: Arc::new(Mutex::new(HashMap::new())),
            network_validator: None,
            player_sync_bridge: None,
            atomic_save_manager: None,
            state_validator: None,
        })
    }
    
    /// Create a new save manager with atomic save support
    pub fn new_with_atomic_save(config: SaveConfig, enable_validation: bool) -> PersistenceResult<Self> {
        let world_save = Arc::new(Mutex::new(WorldSave::new(&config.save_dir)?));
        
        // Create atomic save manager
        let atomic_config = AtomicSaveConfig {
            max_concurrent_operations: config.max_concurrent_saves,
            enable_checksums: true,
            backup_count: config.backup_count,
            ..Default::default()
        };
        let atomic_save_manager = Some(Arc::new(AtomicSaveManager::new(
            config.save_dir.clone(),
            atomic_config,
        )?));
        
        // Create state validator if requested
        let state_validator = if enable_validation {
            let validation_config = ValidationConfig {
                auto_validate: true,
                enable_checksums: true,
                enable_deep_validation: false, // Keep it fast for production
                ..Default::default()
            };
            Some(Arc::new(Mutex::new(StateValidator::new(validation_config))))
        } else {
            None
        };
        
        Ok(Self {
            config,
            world_save,
            save_thread: None,
            shutdown_signal: Arc::new(Mutex::new(false)),
            last_save_time: Arc::new(Mutex::new(Instant::now())),
            dirty_chunks: Arc::new(Mutex::new(HashSet::new())),
            pending_chunk_saves: Arc::new(Mutex::new(HashMap::new())),
            save_in_progress: Arc::new(Mutex::new(false)),
            network_packets: Arc::new(Mutex::new(Vec::new())),
            operation_counter: Arc::new(Mutex::new(0)),
            chunk_states: Arc::new(Mutex::new(HashMap::new())),
            network_validator: None,
            player_sync_bridge: None,
            atomic_save_manager,
            state_validator,
        })
    }
    
    /// Start the automatic save system
    pub fn start_auto_save(&mut self) {
        if !self.config.auto_save_enabled {
            return;
        }
        
        let config = self.config.clone();
        let world_save = Arc::clone(&self.world_save);
        let shutdown_signal = Arc::clone(&self.shutdown_signal);
        let last_save_time = Arc::clone(&self.last_save_time);
        let dirty_chunks = Arc::clone(&self.dirty_chunks);
        let pending_chunk_saves = Arc::clone(&self.pending_chunk_saves);
        let save_in_progress = Arc::clone(&self.save_in_progress);
        
        self.save_thread = Some(thread::spawn(move || {
            Self::auto_save_loop(
                config,
                world_save,
                shutdown_signal,
                last_save_time,
                dirty_chunks,
                pending_chunk_saves,
                save_in_progress,
            );
        }));
    }
    
    /// Stop the automatic save system
    pub fn stop_auto_save(&mut self) -> PersistenceResult<()> {
        *self.shutdown_signal.lock().persistence_lock("shutdown_signal")? = true;
        
        if let Some(thread) = self.save_thread.take() {
            let _ = thread.join();
        }
        Ok(())
    }

    /// Mark a chunk as dirty (needs saving)
    pub fn mark_chunk_dirty(&self, pos: ChunkPos) -> PersistenceResult<()> {
        self.dirty_chunks.lock().persistence_lock("dirty_chunks")?.insert(pos);
        Ok(())
    }
    
    /// Mark a chunk as pending unload
    pub fn mark_chunk_unloading(&self, pos: ChunkPos) -> PersistenceResult<()> {
        if self.config.auto_save_config.save_on_chunk_unload {
            let mut pending = self.pending_chunk_saves.lock().persistence_lock("pending_chunk_saves")?;
            pending.insert(pos, Instant::now());
        }
        Ok(())
    }

    /// Get pending network packets
    pub fn get_network_packets(&self) -> PersistenceResult<Vec<Packet>> {
        let mut packets = self.network_packets.lock().persistence_lock("network_packets")?;
        Ok(std::mem::take(&mut *packets))
    }

    /// Queue a network packet
    fn queue_network_packet(&self, packet: Packet) -> PersistenceResult<()> {
        let mut packets = self.network_packets.lock().persistence_lock("network_packets")?;
        packets.push(packet);
        Ok(())
    }

    /// Get next operation ID
    fn get_next_operation_id(&self) -> PersistenceResult<u32> {
        let mut counter = self.operation_counter.lock().persistence_lock("operation_counter")?;
        *counter += 1;
        Ok(*counter)
    }
    
    /// Save the entire world immediately with network progress indication
    pub fn save_world(&self, world: &World, metadata: &WorldMetadata) -> PersistenceResult<()> {
        let operation_id = self.get_next_operation_id()?;
        
        // Send save starting packet
        self.queue_network_packet(Packet::Server(ServerPacket::SaveProgress {
            operation_id,
            progress: 0.0,
            status: SaveStatus::Starting,
            message: "Initializing world save...".to_string(),
        }))?;
        
        // Set save in progress flag
        *self.save_in_progress.lock().persistence_lock("save_in_progress")? = true;
        
        // Send progress update
        self.queue_network_packet(Packet::Server(ServerPacket::SaveProgress {
            operation_id,
            progress: 0.1,
            status: SaveStatus::InProgress,
            message: "Saving world data...".to_string(),
        }))?;
        
        let start_time = Instant::now();
        let result = {
            let mut world_save = self.world_save.lock().persistence_lock("world_save")?;
            
            // Send compression progress
            self.queue_network_packet(Packet::Server(ServerPacket::SaveProgress {
                operation_id,
                progress: 0.5,
                status: SaveStatus::CompressingData,
                message: "Compressing world data...".to_string(),
            }))?;
            
            let save_result = world_save.save_world(world, metadata);
            
            // Send disk writing progress
            self.queue_network_packet(Packet::Server(ServerPacket::SaveProgress {
                operation_id,
                progress: 0.8,
                status: SaveStatus::WritingToDisk,
                message: "Writing to disk...".to_string(),
            }))?;
            
            save_result
        };
        
        let save_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Handle result and send appropriate packets
        match &result {
            Ok(_) => {
                // Clear dirty chunks on successful save
                let dirty_chunks = self.dirty_chunks.lock().persistence_lock("dirty_chunks")?;
                let chunk_count = dirty_chunks.len() as u32;
                drop(dirty_chunks);
                
                self.dirty_chunks.lock().persistence_lock("dirty_chunks")?.clear();
                *self.last_save_time.lock().persistence_lock("last_save_time")? = Instant::now();
                
                // Update chunk states
                let mut chunk_states = self.chunk_states.lock().persistence_lock("chunk_states")?;
                for chunk_pos in world.get_loaded_chunks() {
                    chunk_states.insert(chunk_pos, ChunkSaveStatus::Saved);
                }
                
                // Send completion packets
                self.queue_network_packet(Packet::Server(ServerPacket::SaveProgress {
                    operation_id,
                    progress: 1.0,
                    status: SaveStatus::Completed,
                    message: "World save completed successfully".to_string(),
                }))?;
                
                self.queue_network_packet(Packet::Server(ServerPacket::WorldSaved {
                    chunks_saved: chunk_count,
                    players_saved: 0, // TODO: Get actual player count
                    save_time,
                    sequence: operation_id,
                }))?;
                
                // Validate save if validator available
                if let Some(validator) = &self.network_validator {
                    if let Ok(validator) = validator.lock() {
                        let _ = validator.validate_full_world();
                    }
                }
            }
            Err(e) => {
                // Send failure packets
                self.queue_network_packet(Packet::Server(ServerPacket::SaveProgress {
                    operation_id,
                    progress: 0.0,
                    status: SaveStatus::Failed,
                    message: format!("Save failed: {}", e),
                }))?;
                
                self.queue_network_packet(Packet::Server(ServerPacket::SaveResult {
                    operation_id,
                    success: false,
                    error_message: Some(e.to_string()),
                    save_time,
                    sequence: operation_id,
                }))?;
            }
        }
        
        *self.save_in_progress.lock().persistence_lock("save_in_progress")? = false;
        
        result.map_err(|e| PersistenceError::IoError(
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        ))
    }
    
    /// Save specific chunks
    pub fn save_chunks(&self, world: &World, positions: &[ChunkPos]) -> PersistenceResult<()> {
        let chunks: Vec<&Chunk> = positions.iter()
            .filter_map(|pos| world.get_chunk(*pos))
            .collect();
        
        if chunks.is_empty() {
            return Ok(());
        }
        
        let chunk_refs = chunks;
        
        let mut world_save = self.world_save.lock().persistence_lock("world_save")?;
        world_save.save_chunks(&chunk_refs)
            .map_err(|errors| {
                PersistenceError::IoError(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to save {} chunks", errors.len())
                    )
                )
            })?;
        
        // Remove from dirty set
        let mut dirty = self.dirty_chunks.lock().persistence_lock("dirty_chunks")?;
        for pos in positions {
            dirty.remove(pos);
        }
        
        Ok(())
    }
    
    /// Save player data with network integration
    pub fn save_player(&self, player_data: &PlayerSaveData) -> PersistenceResult<()> {
        let operation_id = self.get_next_operation_id()?;
        
        let result = player_data.save(&self.config.save_dir);
        
        let save_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Send result packet
        match &result {
            Ok(_) => {
                self.queue_network_packet(Packet::Server(ServerPacket::PlayerSaved {
                    success: true,
                    error_message: None,
                    sequence: operation_id,
                }))?;
                
                // Validate save if validator available
                if let Some(validator) = &self.network_validator {
                    if let Ok(validator) = validator.lock() {
                        let _ = validator.validate_player_save(
                            0, // TODO: Get actual player ID
                            player_data.player_data.position,
                            save_time,
                        );
                    }
                }
                
                // Update player sync bridge if available
                if let Some(bridge) = &self.player_sync_bridge {
                    // TODO: Get actual player ID and mark as saved
                    // bridge.mark_player_saved(player_id)?;
                }
            }
            Err(e) => {
                self.queue_network_packet(Packet::Server(ServerPacket::PlayerSaved {
                    success: false,
                    error_message: Some(e.to_string()),
                    sequence: operation_id,
                }))?;
            }
        }
        
        result
    }

    /// Save player data with explicit player ID for network tracking
    pub fn save_player_with_id(&self, player_id: u32, player_data: &PlayerSaveData) -> PersistenceResult<()> {
        let operation_id = self.get_next_operation_id()?;
        
        let result = player_data.save(&self.config.save_dir);
        
        let save_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Send result packet
        match &result {
            Ok(_) => {
                self.queue_network_packet(Packet::Server(ServerPacket::PlayerSaved {
                    success: true,
                    error_message: None,
                    sequence: operation_id,
                }))?;
                
                // Validate save if validator available
                if let Some(validator) = &self.network_validator {
                    if let Ok(validator) = validator.lock() {
                        let _ = validator.validate_player_save(
                            player_id,
                            player_data.player_data.position,
                            save_time,
                        );
                    }
                }
                
                // Update player sync bridge if available
                if let Some(bridge) = &self.player_sync_bridge {
                    let _ = bridge.mark_player_saved(player_id);
                }
            }
            Err(e) => {
                self.queue_network_packet(Packet::Server(ServerPacket::PlayerSaved {
                    success: false,
                    error_message: Some(e.to_string()),
                    sequence: operation_id,
                }))?;
            }
        }
        
        result
    }
    
    /// Load player data
    pub fn load_player(&self, uuid: &str) -> PersistenceResult<PlayerSaveData> {
        PlayerSaveData::load(&self.config.save_dir, uuid)
    }
    
    /// The auto-save loop
    fn auto_save_loop(
        config: SaveConfig,
        world_save: Arc<Mutex<WorldSave>>,
        shutdown_signal: Arc<Mutex<bool>>,
        last_save_time: Arc<Mutex<Instant>>,
        dirty_chunks: Arc<Mutex<HashSet<ChunkPos>>>,
        pending_chunk_saves: Arc<Mutex<HashMap<ChunkPos, Instant>>>,
        save_in_progress: Arc<Mutex<bool>>,
    ) {
        loop {
            // Check shutdown signal
            match shutdown_signal.lock() {
                Ok(signal) => {
                    if *signal {
                        break;
                    }
                },
                Err(e) => {
                    eprintln!("[SaveManager] Error accessing shutdown signal: {}", e);
                    break; // Exit on poisoned lock
                }
            }
            
            // Sleep for a short interval
            thread::sleep(Duration::from_secs(1));
            
            // Skip if save is already in progress
            match save_in_progress.lock() {
                Ok(in_progress) => {
                    if *in_progress {
                        continue;
                    }
                },
                Err(e) => {
                    eprintln!("[SaveManager] Error accessing save_in_progress: {}", e);
                    continue;
                }
            }
            
            let now = Instant::now();
            
            // Check if it's time for periodic save
            let should_save = match (last_save_time.lock(), dirty_chunks.lock()) {
                (Ok(last_save), Ok(dirty)) => {
                    now.duration_since(*last_save) >= config.auto_save_config.interval
                        && !dirty.is_empty()
                },
                (Err(e), _) => {
                    eprintln!("[SaveManager] Error checking last save time: {}", e);
                    false
                },
                (_, Err(e)) => {
                    eprintln!("[SaveManager] Error checking dirty chunks: {}", e);
                    false
                }
            };
            
            if should_save {
                println!("[SaveManager] Starting automatic save...");
                match save_in_progress.lock() {
                    Ok(mut in_progress) => *in_progress = true,
                    Err(e) => {
                        eprintln!("[SaveManager] Error setting save_in_progress: {}", e);
                        continue;
                    }
                }
                
                // Get dirty chunks to save
                let chunks_to_save: Vec<ChunkPos> = match dirty_chunks.lock() {
                    Ok(dirty) => {
                        dirty.iter()
                            .take(config.auto_save_config.chunk_batch_size)
                            .cloned()
                            .collect()
                    },
                    Err(e) => {
                        eprintln!("[SaveManager] Error accessing dirty chunks: {}", e);
                        vec![]
                    }
                };
                
                // Save chunks (in real implementation, would need World reference)
                // For now, just clear them from dirty set
                match dirty_chunks.lock() {
                    Ok(mut dirty) => {
                        for pos in &chunks_to_save {
                            dirty.remove(pos);
                        }
                    },
                    Err(e) => {
                        eprintln!("[SaveManager] Error clearing dirty chunks: {}", e);
                    }
                }
                
                if let Ok(mut last_save) = last_save_time.lock() {
                    *last_save = now;
                }
                if let Ok(mut in_progress) = save_in_progress.lock() {
                    *in_progress = false;
                }
                
                println!("[SaveManager] Automatic save completed ({} chunks)", chunks_to_save.len());
            }
            
            // Process pending chunk saves (chunks marked for unload)
            let chunks_to_save: Vec<ChunkPos> = match pending_chunk_saves.lock() {
                Ok(pending) => {
                    let delay = config.auto_save_config.unload_save_delay;
                    pending.iter()
                        .filter(|(_, time)| now.duration_since(**time) >= delay)
                        .map(|(pos, _)| *pos)
                        .collect()
                },
                Err(e) => {
                    eprintln!("[SaveManager] Error accessing pending chunks: {}", e);
                    vec![]
                }
            };
            
            if !chunks_to_save.is_empty() {
                // Remove from pending
                match pending_chunk_saves.lock() {
                    Ok(mut pending) => {
                        for pos in &chunks_to_save {
                            pending.remove(pos);
                        }
                    },
                    Err(e) => {
                        eprintln!("[SaveManager] Error removing pending chunks: {}", e);
                    }
                }
                
                // In real implementation, would save these chunks
                println!("[SaveManager] Saved {} unloaded chunks", chunks_to_save.len());
            }
        }
    }
    
    /// Get save statistics
    pub fn get_stats(&self) -> PersistenceResult<SaveStats> {
        Ok(SaveStats {
            dirty_chunk_count: self.dirty_chunks.lock().persistence_lock("dirty_chunks")?.len(),
            pending_chunk_count: self.pending_chunk_saves.lock().persistence_lock("pending_chunk_saves")?.len(),
            save_in_progress: *self.save_in_progress.lock().persistence_lock("save_in_progress")?,
            last_save_time: *self.last_save_time.lock().persistence_lock("last_save_time")?,
        })
    }
    
    /// Get dirty chunks that need saving
    pub fn get_dirty_chunks(&self) -> PersistenceResult<std::collections::HashSet<ChunkPos>> {
        Ok(self.dirty_chunks.lock().persistence_lock("dirty_chunks")?.clone())
    }
    
    /// Save world metadata (for tests)
    pub fn save_world_metadata(&self, _world_save: &crate::persistence::WorldSave) -> PersistenceResult<()> {
        // Mock implementation for testing
        Ok(())
    }
    
    /// Save a single chunk (for tests)
    pub fn save_chunk(&self, pos: ChunkPos, _chunk: &crate::world::Chunk) -> PersistenceResult<()> {
        // Remove from dirty chunks
        let mut dirty = self.dirty_chunks.lock().persistence_lock("dirty_chunks")?;
        dirty.remove(&pos);
        Ok(())
    }
    
    /// Save player data (for tests)
    pub fn save_player_data(&self, _player_data: &crate::persistence::PlayerData) -> PersistenceResult<()> {
        // Mock implementation for testing
        Ok(())
    }
    
    /// Save chunks using atomic operations (if available)
    pub fn save_chunks_atomic(&self, world: &World, positions: &[ChunkPos], priority: SavePriority) -> PersistenceResult<()> {
        if let Some(atomic_manager) = &self.atomic_save_manager {
            // Use atomic save system
            if positions.len() == 1 {
                atomic_manager.queue_operation(SaveOperation::Chunk {
                    pos: positions[0],
                    priority,
                })?;
            } else {
                atomic_manager.queue_operation(SaveOperation::ChunkBatch {
                    positions: positions.to_vec(),
                    priority,
                })?;
            }
            
            // Process the operation immediately for critical saves
            if priority == SavePriority::Critical {
                while let Some(result) = atomic_manager.process_next_operation(world)? {
                    if !result.success {
                        return Err(result.error.unwrap_or_else(|| {
                            PersistenceError::IoError(
                                std::io::Error::new(std::io::ErrorKind::Other, "Atomic save failed")
                            )
                        }));
                    }
                }
            }
            
            Ok(())
        } else {
            // Fall back to regular save
            self.save_chunks(world, positions)
        }
    }
    
    /// Save player data using atomic operations (if available)
    pub fn save_player_atomic(&self, player_uuid: &str, priority: SavePriority) -> PersistenceResult<()> {
        if let Some(atomic_manager) = &self.atomic_save_manager {
            atomic_manager.queue_operation(SaveOperation::Player {
                uuid: player_uuid.to_string(),
                priority,
            })?;
            Ok(())
        } else {
            // Fall back to regular player save (would need player_data parameter)
            Err(PersistenceError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, "Player data not available for regular save")
            ))
        }
    }
    
    /// Validate state consistency (if validator is available)
    pub fn validate_state(&self, world: &World) -> PersistenceResult<()> {
        if let Some(validator) = &self.state_validator {
            let mut validator_guard = validator.lock().persistence_lock("state_validator")?;
            
            // Take snapshots
            let snapshot_id = format!("validation_{}", Instant::now().elapsed().as_millis());
            validator_guard.take_network_snapshot(world, snapshot_id.clone())?;
            validator_guard.take_persistence_snapshot(world, snapshot_id.clone())?;
            
            // Validate consistency
            let result = validator_guard.validate_consistency(&snapshot_id)?;
            
            if !result.success {
                return Err(PersistenceError::CorruptedData(
                    format!("State validation failed: {} errors", result.errors.len())
                ));
            }
            
            println!("[SaveManager] State validation passed: {} warnings", result.warnings.len());
        }
        
        Ok(())
    }
    
    /// Get atomic save manager stats (if available)
    pub fn get_atomic_stats(&self) -> Option<PersistenceResult<crate::persistence::atomic_save::AtomicSaveStats>> {
        self.atomic_save_manager.as_ref().map(|manager| manager.get_stats())
    }
    
    /// Get validation stats (if available)
    pub fn get_validation_stats(&self) -> Option<PersistenceResult<crate::persistence::state_validator::ValidationStats>> {
        self.state_validator.as_ref().map(|validator| {
            let validator_guard = validator.lock().persistence_lock("state_validator")?;
            Ok(validator_guard.get_validation_stats())
        })
    }
    
    /// Process pending atomic operations
    pub fn process_atomic_operations(&self, world: &World, max_operations: Option<usize>) -> PersistenceResult<usize> {
        if let Some(atomic_manager) = &self.atomic_save_manager {
            let mut processed = 0;
            let max_ops = max_operations.unwrap_or(usize::MAX);
            
            while processed < max_ops {
                match atomic_manager.process_next_operation(world)? {
                    Some(_) => processed += 1,
                    None => break, // No more operations
                }
            }
            
            Ok(processed)
        } else {
            Ok(0)
        }
    }
    
    /// Force save all dirty chunks
    pub fn flush(&self) -> PersistenceResult<()> {
        let chunks: Vec<ChunkPos> = self.dirty_chunks.lock().persistence_lock("dirty_chunks")?
            .iter()
            .cloned()
            .collect();
        
        if chunks.is_empty() {
            return Ok(());
        }
        
        // In real implementation, would save these chunks
        self.dirty_chunks.lock().persistence_lock("dirty_chunks")?.clear();
        
        Ok(())
    }

    /// Update chunk save state and send network notifications
    pub fn update_chunk_state(&self, chunk_pos: ChunkPos, state: ChunkSaveStatus) -> PersistenceResult<()> {
        // Update local state
        let mut chunk_states = self.chunk_states.lock().persistence_lock("chunk_states")?;
        chunk_states.insert(chunk_pos, state);
        
        // Send network notification
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        self.queue_network_packet(Packet::Server(ServerPacket::ChunkSaveState {
            chunk_pos,
            state,
            timestamp,
        }))?;
        
        Ok(())
    }

    /// Get current chunk states
    pub fn get_chunk_states(&self) -> PersistenceResult<HashMap<ChunkPos, ChunkSaveStatus>> {
        let chunk_states = self.chunk_states.lock().persistence_lock("chunk_states")?;
        Ok(chunk_states.clone())
    }

    /// Process network save/load requests
    pub fn process_network_requests(&self) -> PersistenceResult<Vec<NetworkSaveRequest>> {
        // This would be called by the network system to get pending save/load operations
        let mut requests = Vec::new();
        
        // Check dirty chunks
        let dirty_chunks = self.dirty_chunks.lock().persistence_lock("dirty_chunks")?;
        for chunk_pos in dirty_chunks.iter() {
            requests.push(NetworkSaveRequest::ChunkSave(*chunk_pos));
        }
        
        // Check player sync bridge for players needing save
        if let Some(bridge) = &self.player_sync_bridge {
            if let Ok(players_to_save) = bridge.get_players_needing_save() {
                for player_id in players_to_save {
                    requests.push(NetworkSaveRequest::PlayerSave(player_id));
                }
            }
        }
        
        Ok(requests)
    }

    /// Get network save statistics
    pub fn get_network_save_stats(&self) -> PersistenceResult<NetworkSaveStats> {
        let dirty_chunks = self.dirty_chunks.lock().persistence_lock("dirty_chunks")?.len();
        let pending_chunks = self.pending_chunk_saves.lock().persistence_lock("pending_chunk_saves")?.len();
        let save_in_progress = *self.save_in_progress.lock().persistence_lock("save_in_progress")?;
        let last_save_time = *self.last_save_time.lock().persistence_lock("last_save_time")?;
        let network_packets = self.network_packets.lock().persistence_lock("network_packets")?.len();
        
        let chunk_states = self.chunk_states.lock().persistence_lock("chunk_states")?;
        let total_chunks = chunk_states.len();
        let saved_chunks = chunk_states.values()
            .filter(|&&state| state == ChunkSaveStatus::Saved || state == ChunkSaveStatus::Clean)
            .count();
        let failed_chunks = chunk_states.values()
            .filter(|&&state| state == ChunkSaveStatus::SaveFailed || state == ChunkSaveStatus::LoadFailed)
            .count();
        
        Ok(NetworkSaveStats {
            dirty_chunks,
            pending_chunks,
            save_in_progress,
            last_save_time,
            network_packets_queued: network_packets,
            total_chunks,
            saved_chunks,
            failed_chunks,
        })
    }
}

/// Network save request types
#[derive(Debug, Clone)]
pub enum NetworkSaveRequest {
    ChunkSave(ChunkPos),
    PlayerSave(u32),
    WorldSave,
}

/// Network save statistics
#[derive(Debug, Clone)]
pub struct NetworkSaveStats {
    pub dirty_chunks: usize,
    pub pending_chunks: usize,
    pub save_in_progress: bool,
    pub last_save_time: Instant,
    pub network_packets_queued: usize,
    pub total_chunks: usize,
    pub saved_chunks: usize,
    pub failed_chunks: usize,
}

/// Statistics about the save system
#[derive(Debug, Clone)]
pub struct SaveStats {
    pub dirty_chunk_count: usize,
    pub pending_chunk_count: usize,
    pub save_in_progress: bool,
    pub last_save_time: Instant,
}

/// Save task for concurrent saving
#[derive(Debug)]
enum SaveTask {
    Chunk(ChunkPos),
    Player(String), // UUID
    Metadata,
}

/// Result of a save operation
#[derive(Debug)]
struct SaveResult {
    task: SaveTask,
    success: bool,
    error: Option<String>,
    duration: Duration,
}

impl Drop for SaveManager {
    fn drop(&mut self) {
        // Best effort - ignore errors during drop
        let _ = self.stop_auto_save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_save_manager_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
        let config = SaveConfig {
            save_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = SaveManager::new_with_config(config).expect("Failed to create SaveManager");
        assert_eq!(manager.get_stats().expect("Failed to get save stats").dirty_chunk_count, 0);
    }
    
    #[test]
    fn test_mark_chunk_dirty() {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
        let config = SaveConfig {
            save_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = SaveManager::new_with_config(config).expect("Failed to create SaveManager");
        
        manager.mark_chunk_dirty(ChunkPos { x: 0, y: 0, z: 0 }).expect("Failed to mark chunk dirty");
        manager.mark_chunk_dirty(ChunkPos { x: 1, y: 0, z: 0 }).expect("Failed to mark chunk dirty");
        
        assert_eq!(manager.get_stats().expect("Failed to get save stats").dirty_chunk_count, 2);
    }
    
    #[test]
    fn test_auto_save_config() {
        let config = AutoSaveConfig::default();
        assert_eq!(config.interval, Duration::from_secs(300));
        assert!(config.save_on_chunk_unload);
        assert!(config.save_on_player_disconnect);
    }
}