use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use bincode;

/// Trait for serializable game state
pub trait SerializableState: Send + Sync {
    /// Get state identifier
    fn state_id(&self) -> &str;
    
    /// Serialize state to bytes
    fn serialize(&self) -> Result<Vec<u8>, StateError>;
    
    /// Deserialize state from bytes
    fn deserialize(&mut self, data: &[u8]) -> Result<(), StateError>;
    
    /// Get state version (for compatibility checking)
    fn version(&self) -> u32 {
        1
    }
}

/// State snapshot
#[derive(Clone)]
pub struct StateSnapshot {
    /// State identifier
    pub id: String,
    
    /// Serialized data
    pub data: Vec<u8>,
    
    /// State version
    pub version: u32,
    
    /// Timestamp
    pub timestamp: std::time::SystemTime,
}

/// State preservation manager
pub struct StatePreserver {
    /// Registered states
    states: Arc<RwLock<HashMap<String, Box<dyn SerializableState>>>>,
    
    /// State snapshots
    snapshots: Arc<RwLock<HashMap<String, StateSnapshot>>>,
    
    /// Snapshot history
    history: Arc<RwLock<Vec<StateSnapshot>>>,
    
    /// Maximum history size
    max_history: usize,
}

impl StatePreserver {
    /// Create new state preserver
    pub fn new(max_history: usize) -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history,
        }
    }
    
    /// Register state for preservation
    pub fn register_state(&self, state: Box<dyn SerializableState>) {
        let id = state.state_id().to_string();
        self.states.write().unwrap().insert(id, state);
    }
    
    /// Create snapshot of all states
    pub fn create_snapshot(&self) -> Result<Vec<StateSnapshot>, StateError> {
        let mut snapshots = Vec::new();
        let states = self.states.read().unwrap();
        
        for (id, state) in states.iter() {
            let data = state.serialize()?;
            let snapshot = StateSnapshot {
                id: id.clone(),
                data,
                version: state.version(),
                timestamp: std::time::SystemTime::now(),
            };
            
            snapshots.push(snapshot);
        }
        
        // Store snapshots
        let mut snapshot_map = self.snapshots.write().unwrap();
        let mut history = self.history.write().unwrap();
        
        for snapshot in &snapshots {
            snapshot_map.insert(snapshot.id.clone(), snapshot.clone());
            history.push(snapshot.clone());
        }
        
        // Trim history
        if history.len() > self.max_history {
            history.drain(0..history.len() - self.max_history);
        }
        
        Ok(snapshots)
    }
    
    /// Restore states from snapshots
    pub fn restore_snapshot(&self) -> Result<(), StateError> {
        let snapshots = self.snapshots.read().unwrap();
        let mut states = self.states.write().unwrap();
        
        for (id, state) in states.iter_mut() {
            if let Some(snapshot) = snapshots.get(id) {
                // Check version compatibility
                if snapshot.version != state.version() {
                    log::warn!(
                        "State version mismatch for {}: snapshot v{}, current v{}",
                        id, snapshot.version, state.version()
                    );
                    continue;
                }
                
                state.deserialize(&snapshot.data)?;
                log::info!("Restored state: {}", id);
            }
        }
        
        Ok(())
    }
    
    /// Save snapshots to disk
    pub fn save_to_disk(&self, path: impl AsRef<std::path::Path>) -> Result<(), StateError> {
        let snapshots: Vec<StateSnapshot> = self.snapshots.read().unwrap()
            .values()
            .cloned()
            .collect();
        
        let data = bincode::serialize(&snapshots)
            .map_err(|e| StateError::SerializationError(e.to_string()))?;
        
        std::fs::write(path, data)
            .map_err(|e| StateError::IoError(e))?;
        
        Ok(())
    }
    
    /// Load snapshots from disk
    pub fn load_from_disk(&self, path: impl AsRef<std::path::Path>) -> Result<(), StateError> {
        let data = std::fs::read(path)
            .map_err(|e| StateError::IoError(e))?;
        
        let snapshots: Vec<StateSnapshot> = bincode::deserialize(&data)
            .map_err(|e| StateError::SerializationError(e.to_string()))?;
        
        let mut snapshot_map = self.snapshots.write().unwrap();
        snapshot_map.clear();
        
        for snapshot in snapshots {
            snapshot_map.insert(snapshot.id.clone(), snapshot);
        }
        
        Ok(())
    }
    
    /// Get state by ID
    pub fn get_state(&self, id: &str) -> Option<&dyn SerializableState> {
        self.states.read().unwrap().get(id).map(|s| s.as_ref())
    }
    
    /// Clear all snapshots
    pub fn clear_snapshots(&self) {
        self.snapshots.write().unwrap().clear();
        self.history.write().unwrap().clear();
    }
}

/// State error types
#[derive(Debug)]
pub enum StateError {
    SerializationError(String),
    DeserializationError(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            StateError::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
            StateError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for StateError {}

/// Example implementations for common game state

/// Player state
#[derive(Serialize, Deserialize)]
pub struct PlayerState {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub health: f32,
    pub inventory: Vec<u32>,
}

impl SerializableState for PlayerState {
    fn state_id(&self) -> &str {
        "player"
    }
    
    fn serialize(&self) -> Result<Vec<u8>, StateError> {
        bincode::serialize(self)
            .map_err(|e| StateError::SerializationError(e.to_string()))
    }
    
    fn deserialize(&mut self, data: &[u8]) -> Result<(), StateError> {
        let state: PlayerState = bincode::deserialize(data)
            .map_err(|e| StateError::DeserializationError(e.to_string()))?;
        
        *self = state;
        Ok(())
    }
}

/// Camera state
#[derive(Serialize, Deserialize)]
pub struct CameraState {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub fov: f32,
}

impl SerializableState for CameraState {
    fn state_id(&self) -> &str {
        "camera"
    }
    
    fn serialize(&self) -> Result<Vec<u8>, StateError> {
        bincode::serialize(self)
            .map_err(|e| StateError::SerializationError(e.to_string()))
    }
    
    fn deserialize(&mut self, data: &[u8]) -> Result<(), StateError> {
        let state: CameraState = bincode::deserialize(data)
            .map_err(|e| StateError::DeserializationError(e.to_string()))?;
        
        *self = state;
        Ok(())
    }
}

/// State preservation scope guard
pub struct StateScope<'a> {
    preserver: &'a StatePreserver,
    restored: bool,
}

impl<'a> StateScope<'a> {
    /// Create new state scope
    pub fn new(preserver: &'a StatePreserver) -> Result<Self, StateError> {
        // Create snapshot on entry
        preserver.create_snapshot()?;
        
        Ok(Self {
            preserver,
            restored: false,
        })
    }
    
    /// Restore state (call if reload succeeded)
    pub fn restore(mut self) -> Result<(), StateError> {
        self.preserver.restore_snapshot()?;
        self.restored = true;
        Ok(())
    }
}

impl<'a> Drop for StateScope<'a> {
    fn drop(&mut self) {
        if !self.restored {
            // Restore automatically if not done manually
            if let Err(e) = self.preserver.restore_snapshot() {
                log::error!("Failed to restore state in scope guard: {}", e);
            }
        }
    }
}

/// Macro for implementing SerializableState
#[macro_export]
macro_rules! impl_serializable_state {
    ($type:ty, $id:literal) => {
        impl SerializableState for $type {
            fn state_id(&self) -> &str {
                $id
            }
            
            fn serialize(&self) -> Result<Vec<u8>, StateError> {
                bincode::serialize(self)
                    .map_err(|e| StateError::SerializationError(e.to_string()))
            }
            
            fn deserialize(&mut self, data: &[u8]) -> Result<(), StateError> {
                let state: Self = bincode::deserialize(data)
                    .map_err(|e| StateError::DeserializationError(e.to_string()))?;
                
                *self = state;
                Ok(())
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_state_preservation() {
        let preserver = StatePreserver::new(10);
        
        // Register states
        let player = PlayerState {
            position: [10.0, 20.0, 30.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            health: 100.0,
            inventory: vec![1, 2, 3],
        };
        
        preserver.register_state(Box::new(player));
        
        // Create snapshot
        let snapshots = preserver.create_snapshot().unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, "player");
        
        // Modify state would happen here...
        
        // Restore snapshot
        preserver.restore_snapshot().unwrap();
    }
}