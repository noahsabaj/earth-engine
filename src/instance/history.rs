/// Instance History Tracking
/// 
/// Tracks all changes to instances over time.
/// Stores who changed what, when, and previous values.
/// Uses ring buffer for efficient memory usage.

use crate::instance::{InstanceId, MetadataValue, MetadataKey};
use crate::instance::error::{InstanceResult, timestamp_error};
use serde::{Serialize, Deserialize};

/// History event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryEvent {
    Created,
    Modified,
    Deleted,
    MetadataSet,
    MetadataRemoved,
    TypeChanged,
    OwnerChanged,
}

/// Single history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// When this happened (unix timestamp)
    pub timestamp: u64,
    /// Who made the change
    pub actor: InstanceId,
    /// What type of change
    pub event: HistoryEvent,
    /// Instance version after this change
    pub version: u32,
    /// Optional metadata key affected
    pub metadata_key: Option<&'static str>,
    /// Previous value (for rollback)
    pub previous_value: Option<MetadataValue>,
    /// New value (for audit)
    pub new_value: Option<MetadataValue>,
}

/// Ring buffer for history storage
pub struct HistoryRingBuffer {
    /// Fixed-size buffer
    entries: Vec<Option<HistoryEntry>>,
    /// Current write position
    write_pos: usize,
    /// Total entries written (for ordering)
    total_written: u64,
}

impl HistoryRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: vec![None; capacity],
            write_pos: 0,
            total_written: 0,
        }
    }
    
    /// Add new history entry
    pub fn push(&mut self, entry: HistoryEntry) {
        if let Some(slot) = self.entries.get_mut(self.write_pos) {
            *slot = Some(entry);
            self.write_pos = (self.write_pos + 1) % self.entries.len();
            self.total_written += 1;
        }
    }
    
    /// Get last N entries
    pub fn recent(&self, count: usize) -> Vec<&HistoryEntry> {
        let mut result = Vec::new();
        let capacity = self.entries.len();
        let start = if self.total_written < capacity as u64 {
            0
        } else {
            self.write_pos
        };
        
        for i in 0..count.min(capacity) {
            let idx = (start + capacity - 1 - i) % capacity;
            if let Some(Some(ref entry)) = self.entries.get(idx) {
                result.push(entry);
            }
        }
        
        result
    }
}

/// Main history log for all instances
pub struct HistoryLog {
    /// History indexed by instance ID
    instance_histories: std::collections::HashMap<InstanceId, HistoryRingBuffer>,
    /// Global history buffer for system-wide events
    global_history: HistoryRingBuffer,
    /// History buffer size per instance
    buffer_size: usize,
}

impl HistoryLog {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            instance_histories: std::collections::HashMap::new(),
            global_history: HistoryRingBuffer::new(buffer_size * 10), // Larger for global
            buffer_size,
        }
    }
    
    /// Record a history event
    pub fn record(&mut self, instance_id: InstanceId, entry: HistoryEntry) {
        // Add to instance-specific history
        self.instance_histories
            .entry(instance_id)
            .or_insert_with(|| HistoryRingBuffer::new(self.buffer_size))
            .push(entry.clone());
            
        // Also add to global history
        self.global_history.push(entry);
    }
    
    /// Get history for specific instance
    pub fn get_instance_history(&self, id: &InstanceId, count: usize) -> Vec<&HistoryEntry> {
        self.instance_histories
            .get(id)
            .map(|buffer| buffer.recent(count))
            .unwrap_or_default()
    }
    
    /// Get recent global history
    pub fn get_global_history(&self, count: usize) -> Vec<&HistoryEntry> {
        self.global_history.recent(count)
    }
    
    /// Find history by actor
    pub fn find_by_actor(&self, actor: &InstanceId, count: usize) -> Vec<&HistoryEntry> {
        self.global_history
            .recent(count * 10) // Search more to find enough matches
            .into_iter()
            .filter(|entry| &entry.actor == actor)
            .take(count)
            .collect()
    }
    
    /// Clear history for deleted instance (save memory)
    pub fn clear_instance(&mut self, id: &InstanceId) {
        self.instance_histories.remove(id);
    }
}

/// Helper to create history entries
pub struct HistoryBuilder {
    timestamp: u64,
    actor: InstanceId,
}

impl HistoryBuilder {
    pub fn new(actor: InstanceId) -> InstanceResult<Self> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| timestamp_error("history builder"))?
            .as_secs();
            
        Ok(Self { timestamp, actor })
    }
    
    pub fn created(&self, version: u32) -> HistoryEntry {
        HistoryEntry {
            timestamp: self.timestamp,
            actor: self.actor,
            event: HistoryEvent::Created,
            version,
            metadata_key: None,
            previous_value: None,
            new_value: None,
        }
    }
    
    pub fn metadata_changed(
        &self,
        version: u32,
        key: &'static str,
        old_value: Option<MetadataValue>,
        new_value: Option<MetadataValue>,
    ) -> HistoryEntry {
        HistoryEntry {
            timestamp: self.timestamp,
            actor: self.actor,
            event: if new_value.is_none() {
                HistoryEvent::MetadataRemoved
            } else {
                HistoryEvent::MetadataSet
            },
            version,
            metadata_key: Some(key),
            previous_value: old_value,
            new_value,
        }
    }
    
    pub fn deleted(&self, version: u32) -> HistoryEntry {
        HistoryEntry {
            timestamp: self.timestamp,
            actor: self.actor,
            event: HistoryEvent::Deleted,
            version,
            metadata_key: None,
            previous_value: None,
            new_value: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ring_buffer() {
        let mut buffer = HistoryRingBuffer::new(3);
        let actor = InstanceId::new();
        let builder = HistoryBuilder::new(actor).unwrap();
        
        // Fill buffer
        buffer.push(builder.created(1));
        buffer.push(builder.deleted(2));
        buffer.push(builder.created(3));
        
        // Should have 3 entries
        assert_eq!(buffer.recent(10).len(), 3);
        
        // Add one more (wraps around)
        buffer.push(builder.deleted(4));
        
        // Still 3 entries, but oldest is gone
        let recent = buffer.recent(10);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].version, 4); // Most recent
        assert_eq!(recent[2].version, 2); // Oldest remaining
    }
    
    #[test]
    fn test_history_log() {
        let mut log = HistoryLog::new(10);
        let instance = InstanceId::new();
        let actor = InstanceId::new();
        let builder = HistoryBuilder::new(actor).unwrap();
        
        // Record some events
        log.record(instance, builder.created(1));
        log.record(instance, builder.metadata_changed(
            2,
            "name",
            None,
            Some(MetadataValue::String("Test".to_string()))
        ));
        
        // Check instance history
        let history = log.get_instance_history(&instance, 10);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].event, HistoryEvent::MetadataSet);
        assert_eq!(history[1].event, HistoryEvent::Created);
        
        // Check actor search
        let by_actor = log.find_by_actor(&actor, 10);
        assert_eq!(by_actor.len(), 2);
    }
}