/// UUID-based Instance Identification
/// 
/// Provides unique identifiers for every instance in the game.
/// Uses 128-bit UUIDs for globally unique identification.
/// Supports efficient serialization and network transmission.

use serde::{Serialize, Deserialize};
use std::fmt;
use crate::instance::error::{InstanceResult, timestamp_error};

/// 128-bit unique instance identifier
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InstanceId {
    /// High 64 bits
    pub high: u64,
    /// Low 64 bits  
    pub low: u64,
}

impl InstanceId {
    /// Create a new random instance ID
    pub fn new() -> Self {
        Self {
            high: rand::random(),
            low: rand::random(),
        }
    }
    
    /// Create a nil/empty instance ID
    pub const fn nil() -> Self {
        Self { high: 0, low: 0 }
    }
    
    /// Check if this is a nil ID
    pub fn is_nil(&self) -> bool {
        self.high == 0 && self.low == 0
    }
    
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        let high = u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let low = u64::from_be_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11],
            bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        Self { high, low }
    }
    
    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&self.high.to_be_bytes());
        bytes[8..16].copy_from_slice(&self.low.to_be_bytes());
        bytes
    }
    
    /// Create from string (hex format)
    pub fn from_string(s: &str) -> Option<Self> {
        if s.len() != 32 {
            return None;
        }
        
        let high = u64::from_str_radix(&s[0..16], 16).ok()?;
        let low = u64::from_str_radix(&s[16..32], 16).ok()?;
        
        Some(Self { high, low })
    }
}

impl fmt::Display for InstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}{:016x}", self.high, self.low)
    }
}

impl Default for InstanceId {
    fn default() -> Self {
        Self::nil()
    }
}

/// Thread-safe instance ID generator
pub struct InstanceIdGenerator {
    /// Node ID for distributed systems (prevents collisions)
    node_id: u16,
    /// Counter for sequential IDs
    counter: std::sync::atomic::AtomicU64,
}

impl InstanceIdGenerator {
    /// Create a new generator with node ID
    pub fn new(node_id: u16) -> Self {
        Self {
            node_id,
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    /// Generate a new instance ID
    /// Format: [timestamp:48][node:16][sequence:64]
    pub fn generate(&self) -> InstanceResult<InstanceId> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| timestamp_error("instance ID generation"))?
            .as_millis() as u64;
            
        let sequence = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        // High 64 bits: timestamp (48) + node_id (16)
        let high = (timestamp << 16) | (self.node_id as u64);
        
        Ok(InstanceId {
            high,
            low: sequence,
        })
    }
}

/// Efficient instance ID set for fast lookups
pub struct InstanceIdSet {
    /// Bloom filter for fast negative lookups
    bloom: Vec<u64>,
    /// Actual set for positive confirmation
    set: std::collections::HashSet<InstanceId>,
}

impl InstanceIdSet {
    pub fn new() -> Self {
        Self {
            bloom: vec![0; 1024], // 64KB bloom filter
            set: std::collections::HashSet::new(),
        }
    }
    
    pub fn insert(&mut self, id: InstanceId) {
        // Update bloom filter
        let hash = id.low ^ id.high;
        let index = (hash as usize) % (self.bloom.len() * 64);
        let word = index / 64;
        let bit = index % 64;
        if let Some(bloom_word) = self.bloom.get_mut(word) {
            *bloom_word |= 1u64 << bit;
        }
        
        // Add to set
        self.set.insert(id);
    }
    
    pub fn contains(&self, id: &InstanceId) -> bool {
        // Check bloom filter first (fast negative)
        let hash = id.low ^ id.high;
        let index = (hash as usize) % (self.bloom.len() * 64);
        let word = index / 64;
        let bit = index % 64;
        
        if self.bloom.get(word).map_or(true, |&bloom_word| (bloom_word & (1u64 << bit)) == 0) {
            return false; // Definitely not in set
        }
        
        // Confirm with actual set
        self.set.contains(id)
    }
    
    pub fn len(&self) -> usize {
        self.set.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_instance_id_creation() {
        let id1 = InstanceId::new();
        let id2 = InstanceId::new();
        
        assert_ne!(id1, id2); // Should be unique
        assert!(!id1.is_nil());
    }
    
    #[test]
    fn test_instance_id_serialization() {
        let id = InstanceId::new();
        let bytes = id.to_bytes();
        let restored = InstanceId::from_bytes(bytes);
        
        assert_eq!(id, restored);
    }
    
    #[test]
    fn test_instance_id_string() {
        let id = InstanceId { high: 0x1234567890ABCDEF, low: 0xFEDCBA0987654321 };
        let s = id.to_string();
        let restored = InstanceId::from_string(&s).unwrap();
        
        assert_eq!(id, restored);
        assert_eq!(s, "1234567890abcdeffedcba0987654321");
    }
    
    #[test]
    fn test_id_generator() {
        let gen = InstanceIdGenerator::new(42);
        let id1 = gen.generate().unwrap();
        let id2 = gen.generate().unwrap();
        
        assert_ne!(id1, id2);
        // Check node ID is embedded
        assert_eq!((id1.high & 0xFFFF) as u16, 42);
        assert_eq!((id2.high & 0xFFFF) as u16, 42);
    }
}