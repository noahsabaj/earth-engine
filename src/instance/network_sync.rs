/// Network-Friendly Instance Syncing
/// 
/// Efficient synchronization of instances across network.
/// Uses delta compression and batching for minimal bandwidth.
/// Supports both reliable and unreliable transport.

use crate::instance::{InstanceId, InstanceType, MetadataValue, MetadataKey};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Sync packet types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncPacket {
    /// Full instance snapshot
    Snapshot(InstanceSnapshot),
    /// Delta update
    Delta(InstanceDelta),
    /// Batch of updates
    Batch(Vec<SyncPacket>),
    /// Request specific instances
    Request(Vec<InstanceId>),
    /// Acknowledge receipt
    Ack(u64), // sequence number
}

/// Complete instance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSnapshot {
    pub id: InstanceId,
    pub instance_type: InstanceType,
    pub version: u32,
    pub metadata: HashMap<String, MetadataValue>,
    pub created_at: u64,
    pub created_by: InstanceId,
}

/// Delta update for instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceDelta {
    pub id: InstanceId,
    pub from_version: u32,
    pub to_version: u32,
    pub changes: Vec<DeltaChange>,
}

/// Individual change in delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaChange {
    MetadataSet(String, MetadataValue),
    MetadataRemove(String),
    TypeChanged(InstanceType),
    Deleted,
}

/// Network sync state for a peer
pub struct SyncState {
    /// Last known versions for instances
    peer_versions: HashMap<InstanceId, u32>,
    /// Pending acknowledgments
    pending_acks: HashMap<u64, Vec<InstanceId>>,
    /// Next sequence number
    next_seq: u64,
    /// Last received sequence
    last_received: u64,
    /// Statistics
    stats: SyncStats,
}

/// Sync statistics
#[derive(Default)]
pub struct SyncStats {
    pub snapshots_sent: u64,
    pub deltas_sent: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub compression_ratio: f32,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            peer_versions: HashMap::new(),
            pending_acks: HashMap::new(),
            next_seq: 0,
            last_received: 0,
            stats: SyncStats::default(),
        }
    }
    
    /// Get next sequence number
    pub fn next_sequence(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }
    
    /// Record sent instances
    pub fn record_sent(&mut self, seq: u64, instances: Vec<InstanceId>) {
        self.pending_acks.insert(seq, instances);
    }
    
    /// Process acknowledgment
    pub fn process_ack(&mut self, seq: u64) {
        if let Some(instances) = self.pending_acks.remove(&seq) {
            // Update peer versions based on what was acknowledged
            for id in instances {
                // Would update versions here based on sent data
            }
        }
    }
    
    /// Check if peer needs update
    pub fn needs_update(&self, id: &InstanceId, current_version: u32) -> bool {
        self.peer_versions.get(id).map_or(true, |&v| v < current_version)
    }
}

/// Main instance sync system
pub struct InstanceSync {
    /// Sync states per peer
    peers: HashMap<String, SyncState>,
    /// Compression threshold (bytes)
    compression_threshold: usize,
    /// Max batch size
    max_batch_size: usize,
}

impl InstanceSync {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            compression_threshold: 1024,
            max_batch_size: 100,
        }
    }
    
    /// Add new peer
    pub fn add_peer(&mut self, peer_id: String) {
        self.peers.insert(peer_id, SyncState::new());
    }
    
    /// Remove peer
    pub fn remove_peer(&mut self, peer_id: &str) {
        self.peers.remove(peer_id);
    }
    
    /// Generate sync packet for peer
    pub fn generate_sync_packet(
        &mut self,
        peer_id: &str,
        instances: &[(InstanceId, InstanceSnapshot, u32)], // (id, snapshot, current_version)
    ) -> Option<SyncPacket> {
        let mut packets = Vec::new();
        let mut updates_needed = Vec::new();
        
        // First pass: determine what updates are needed
        if let Some(state) = self.peers.get(peer_id) {
            for (id, snapshot, current_version) in instances {
                if state.needs_update(id, *current_version) {
                    let peer_version = state.peer_versions.get(id).copied().unwrap_or(0);
                    updates_needed.push((id, snapshot, current_version, peer_version));
                }
            }
        } else {
            return None;
        }
        
        // Second pass: generate packets (no active borrows)
        for (id, snapshot, current_version, peer_version) in updates_needed {
            if peer_version == 0 {
                // Send full snapshot
                packets.push(SyncPacket::Snapshot(snapshot.clone()));
            } else {
                // Send delta if possible
                if let Some(delta) = self.generate_delta(id, peer_version, *current_version, snapshot) {
                    packets.push(SyncPacket::Delta(delta));
                } else {
                    // Fall back to snapshot
                    packets.push(SyncPacket::Snapshot(snapshot.clone()));
                }
            }
        }
        
        // Update stats
        if let Some(state) = self.peers.get_mut(peer_id) {
            for packet in &packets {
                match packet {
                    SyncPacket::Snapshot(_) => state.stats.snapshots_sent += 1,
                    SyncPacket::Delta(_) => state.stats.deltas_sent += 1,
                    _ => {}
                }
            }
        }
        
        if packets.is_empty() {
            None
        } else if packets.len() == 1 {
            packets.into_iter().next()
        } else {
            // Batch multiple updates
            Some(SyncPacket::Batch(packets))
        }
    }
    
    /// Generate delta between versions
    fn generate_delta(
        &self,
        id: &InstanceId,
        from_version: u32,
        to_version: u32,
        current: &InstanceSnapshot,
    ) -> Option<InstanceDelta> {
        // In real implementation, would diff against historical versions
        // For now, return None to force snapshot
        None
    }
    
    /// Process received sync packet
    pub fn process_packet(&mut self, peer_id: &str, packet: SyncPacket) -> Vec<InstanceUpdate> {
        let mut updates = Vec::new();
        
        match packet {
            SyncPacket::Snapshot(snapshot) => {
                updates.push(InstanceUpdate::Snapshot(snapshot));
            }
            
            SyncPacket::Delta(delta) => {
                updates.push(InstanceUpdate::Delta(delta));
            }
            
            SyncPacket::Batch(packets) => {
                for p in packets {
                    updates.extend(self.process_packet(peer_id, p));
                }
            }
            
            SyncPacket::Request(ids) => {
                updates.push(InstanceUpdate::RequestReceived(ids));
            }
            
            SyncPacket::Ack(seq) => {
                if let Some(state) = self.peers.get_mut(peer_id) {
                    state.process_ack(seq);
                }
            }
        }
        
        updates
    }
}

/// Updates to apply locally
#[derive(Debug)]
pub enum InstanceUpdate {
    Snapshot(InstanceSnapshot),
    Delta(InstanceDelta),
    RequestReceived(Vec<InstanceId>),
}

/// Binary serialization for network
pub struct NetworkSerializer;

impl NetworkSerializer {
    /// Serialize packet to bytes
    pub fn serialize(packet: &SyncPacket) -> Result<Vec<u8>, &'static str> {
        bincode::serialize(packet).map_err(|_| "Serialization failed")
    }
    
    /// Deserialize packet from bytes
    pub fn deserialize(data: &[u8]) -> Result<SyncPacket, &'static str> {
        bincode::deserialize(data).map_err(|_| "Deserialization failed")
    }
    
    /// Compress if beneficial
    pub fn compress(data: &[u8]) -> Vec<u8> {
        // Would use lz4 or similar for real-time compression
        // For now, return as-is
        data.to_vec()
    }
    
    /// Decompress data
    pub fn decompress(data: &[u8]) -> Result<Vec<u8>, &'static str> {
        // Would decompress here
        Ok(data.to_vec())
    }
}

/// Priority queue for updates
pub struct UpdateQueue {
    /// High priority (player actions)
    high: Vec<(InstanceId, u32)>,
    /// Medium priority (nearby objects)
    medium: Vec<(InstanceId, u32)>,
    /// Low priority (distant objects)
    low: Vec<(InstanceId, u32)>,
}

impl UpdateQueue {
    pub fn new() -> Self {
        Self {
            high: Vec::new(),
            medium: Vec::new(),
            low: Vec::new(),
        }
    }
    
    pub fn push_high(&mut self, id: InstanceId, version: u32) {
        self.high.push((id, version));
    }
    
    pub fn push_medium(&mut self, id: InstanceId, version: u32) {
        self.medium.push((id, version));
    }
    
    pub fn push_low(&mut self, id: InstanceId, version: u32) {
        self.low.push((id, version));
    }
    
    /// Get next batch respecting priorities
    pub fn next_batch(&mut self, max_size: usize) -> Vec<(InstanceId, u32)> {
        let mut batch = Vec::new();
        
        // Take from high priority first
        while !self.high.is_empty() && batch.len() < max_size {
            if let Some(item) = self.high.pop() {
                batch.push(item);
            }
        }
        
        // Then medium
        while !self.medium.is_empty() && batch.len() < max_size {
            if let Some(item) = self.medium.pop() {
                batch.push(item);
            }
        }
        
        // Finally low
        while !self.low.is_empty() && batch.len() < max_size {
            if let Some(item) = self.low.pop() {
                batch.push(item);
            }
        }
        
        batch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sync_state() {
        let mut state = SyncState::new();
        let id = InstanceId::new();
        
        // Should need update for unknown instance
        assert!(state.needs_update(&id, 1));
        
        // Record version
        state.peer_versions.insert(id, 1);
        
        // Should not need update for same version
        assert!(!state.needs_update(&id, 1));
        
        // Should need update for newer version
        assert!(state.needs_update(&id, 2));
    }
    
    #[test]
    fn test_packet_serialization() {
        let snapshot = InstanceSnapshot {
            id: InstanceId::new(),
            instance_type: InstanceType::Item,
            version: 1,
            metadata: HashMap::new(),
            created_at: 12345,
            created_by: InstanceId::new(),
        };
        
        let packet = SyncPacket::Snapshot(snapshot);
        
        // Serialize
        let bytes = NetworkSerializer::serialize(&packet).unwrap();
        
        // Deserialize
        let restored = NetworkSerializer::deserialize(&bytes).unwrap();
        
        match restored {
            SyncPacket::Snapshot(s) => {
                assert_eq!(s.version, 1);
                assert_eq!(s.created_at, 12345);
            }
            _ => panic!("Wrong packet type"),
        }
    }
    
    #[test]
    fn test_update_queue() {
        let mut queue = UpdateQueue::new();
        
        let id1 = InstanceId::new();
        let id2 = InstanceId::new();
        let id3 = InstanceId::new();
        
        queue.push_low(id1, 1);
        queue.push_high(id2, 2);
        queue.push_medium(id3, 3);
        
        let batch = queue.next_batch(2);
        
        // Should get high priority first
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].0, id2); // high priority
        assert_eq!(batch[1].0, id3); // medium priority
    }
}