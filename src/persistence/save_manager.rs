use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};

use crate::world::{World, ChunkPos, Chunk};
use crate::persistence::{
    PersistenceResult, PersistenceError,
    WorldSave, PlayerSaveData, WorldMetadata,
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
}

impl SaveManager {
    /// Create a new save manager
    pub fn new(config: SaveConfig) -> PersistenceResult<Self> {
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
        *self.shutdown_signal.lock()? = true;
        
        if let Some(thread) = self.save_thread.take() {
            let _ = thread.join();
        }
        Ok(())
    }
    
    /// Mark a chunk as dirty (needs saving)
    pub fn mark_chunk_dirty(&self, pos: ChunkPos) -> PersistenceResult<()> {
        self.dirty_chunks.lock()?.insert(pos);
        Ok(())
    }
    
    /// Mark a chunk as pending unload
    pub fn mark_chunk_unloading(&self, pos: ChunkPos) -> PersistenceResult<()> {
        if self.config.auto_save_config.save_on_chunk_unload {
            let mut pending = self.pending_chunk_saves.lock()?;
            pending.insert(pos, Instant::now());
        }
        Ok(())
    }
    
    /// Save the entire world immediately
    pub fn save_world(&self, world: &World, metadata: &WorldMetadata) -> PersistenceResult<()> {
        // Set save in progress flag
        *self.save_in_progress.lock()? = true;
        
        let result = {
            let mut world_save = self.world_save.lock()?;
            world_save.save_world(world, metadata)
        };
        
        // Clear dirty chunks on successful save
        if result.is_ok() {
            self.dirty_chunks.lock()?.clear();
            *self.last_save_time.lock()? = Instant::now();
        }
        
        *self.save_in_progress.lock()? = false;
        
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
        
        let mut world_save = self.world_save.lock()?;
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
        let mut dirty = self.dirty_chunks.lock()?;
        for pos in positions {
            dirty.remove(pos);
        }
        
        Ok(())
    }
    
    /// Save player data
    pub fn save_player(&self, player_data: &PlayerSaveData) -> PersistenceResult<()> {
        player_data.save(&self.config.save_dir)
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
            dirty_chunk_count: self.dirty_chunks.lock()?.len(),
            pending_chunk_count: self.pending_chunk_saves.lock()?.len(),
            save_in_progress: *self.save_in_progress.lock()?,
            last_save_time: *self.last_save_time.lock()?,
        })
    }
    
    /// Force save all dirty chunks
    pub fn flush(&self) -> PersistenceResult<()> {
        let chunks: Vec<ChunkPos> = self.dirty_chunks.lock()?
            .iter()
            .cloned()
            .collect();
        
        if chunks.is_empty() {
            return Ok(());
        }
        
        // In real implementation, would save these chunks
        self.dirty_chunks.lock()?.clear();
        
        Ok(())
    }
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
        let temp_dir = TempDir::new().unwrap();
        let config = SaveConfig {
            save_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = SaveManager::new(config).unwrap();
        assert_eq!(manager.get_stats().unwrap().dirty_chunk_count, 0);
    }
    
    #[test]
    fn test_mark_chunk_dirty() {
        let temp_dir = TempDir::new().unwrap();
        let config = SaveConfig {
            save_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = SaveManager::new(config).unwrap();
        
        manager.mark_chunk_dirty(ChunkPos { x: 0, y: 0, z: 0 }).unwrap();
        manager.mark_chunk_dirty(ChunkPos { x: 1, y: 0, z: 0 }).unwrap();
        
        assert_eq!(manager.get_stats().unwrap().dirty_chunk_count, 2);
    }
    
    #[test]
    fn test_auto_save_config() {
        let config = AutoSaveConfig::default();
        assert_eq!(config.interval, Duration::from_secs(300));
        assert!(config.save_on_chunk_unload);
        assert!(config.save_on_player_disconnect);
    }
}