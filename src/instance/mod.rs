/// Instance & Metadata System
/// 
/// Provides unique identification and metadata storage for all game entities.
/// Every item, block, entity can have a unique UUID with associated metadata.
/// Purely data-oriented - no instance "objects", just tables of data.
/// 
/// Part of Sprint 30: Instance & Metadata System

pub mod instance_id;
pub mod metadata_store;
pub mod history;
pub mod query;
pub mod copy_on_write;
pub mod network_sync;
pub mod error;

pub use instance_id::{InstanceId, InstanceIdGenerator};
pub use metadata_store::{MetadataStore, MetadataValue, MetadataKey};
pub use history::{HistoryLog, HistoryEntry, HistoryEvent};
pub use query::{InstanceQuery, QueryResult, QueryFilter};
pub use copy_on_write::{CowMetadata, CowHandle};
pub use network_sync::{InstanceSync, SyncPacket, SyncState};
pub use error::{InstanceResult, InstanceErrorContext, timestamp_error};

/// Maximum instances supported (16 million)
pub const MAX_INSTANCES: usize = 1 << 24;

use serde::{Serialize, Deserialize};

/// Instance type categories for efficient filtering
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstanceType {
    Block = 0,
    Item = 1,
    Entity = 2,
    Player = 3,
    Structure = 4,
    Container = 5,
    Tool = 6,
    Custom = 255,
}

/// Core instance data (Structure of Arrays)
pub struct InstanceData {
    /// Instance IDs (sparse, some may be unused)
    pub ids: Vec<InstanceId>,
    
    /// Instance types for filtering
    pub types: Vec<InstanceType>,
    
    /// Creation timestamps (unix epoch)
    pub created_at: Vec<u64>,
    
    /// Creator IDs (player who created)
    pub created_by: Vec<InstanceId>,
    
    /// Last modified timestamps
    pub modified_at: Vec<u64>,
    
    /// Version numbers for optimistic locking
    pub versions: Vec<u32>,
    
    /// Active flags (false = deleted but retained for history)
    pub active: Vec<bool>,
}

impl InstanceData {
    pub fn new() -> Self {
        Self {
            ids: Vec::with_capacity(MAX_INSTANCES),
            types: Vec::with_capacity(MAX_INSTANCES),
            created_at: Vec::with_capacity(MAX_INSTANCES),
            created_by: Vec::with_capacity(MAX_INSTANCES),
            modified_at: Vec::with_capacity(MAX_INSTANCES),
            versions: Vec::with_capacity(MAX_INSTANCES),
            active: Vec::with_capacity(MAX_INSTANCES),
        }
    }
    
    /// Add a new instance
    pub fn add(&mut self, id: InstanceId, instance_type: InstanceType, creator: InstanceId) -> InstanceResult<usize> {
        let index = self.ids.len();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| timestamp_error("instance creation"))?
            .as_secs();
            
        self.ids.push(id);
        self.types.push(instance_type);
        self.created_at.push(now);
        self.created_by.push(creator);
        self.modified_at.push(now);
        self.versions.push(0);
        self.active.push(true);
        
        Ok(index)
    }
    
    /// Mark instance as deleted (soft delete for history)
    pub fn delete(&mut self, index: usize) -> InstanceResult<()> {
        if index < self.active.len() {
            self.active[index] = false;
            self.versions[index] += 1;
            self.modified_at[index] = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| timestamp_error("instance deletion"))?
                .as_secs();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_instance_creation() {
        let mut data = InstanceData::new();
        let id = InstanceId::new();
        let creator = InstanceId::new();
        
        let index = data.add(id, InstanceType::Item, creator).unwrap();
        
        assert_eq!(data.ids[index], id);
        assert_eq!(data.types[index], InstanceType::Item);
        assert_eq!(data.created_by[index], creator);
        assert!(data.active[index]);
    }
}