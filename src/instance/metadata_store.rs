/// Metadata Storage System
/// 
/// Stores arbitrary key-value metadata for instances.
/// Uses column-based storage for efficient memory usage.
/// Supports different value types without boxing.

use crate::instance::InstanceId;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Metadata key type
pub type MetadataKey = &'static str;

/// Metadata value variants (unboxed for performance)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetadataValue {
    /// Boolean flag
    Bool(bool),
    /// 32-bit integer
    I32(i32),
    /// 64-bit integer
    I64(i64),
    /// 32-bit float
    F32(f32),
    /// 64-bit float
    F64(f64),
    /// Short string (up to 64 bytes)
    String(String),
    /// Binary data
    Bytes(Vec<u8>),
    /// Reference to another instance
    InstanceRef(InstanceId),
    /// Position in world
    Position([f32; 3]),
    /// Rotation quaternion
    Rotation([f32; 4]),
}

/// Column-based metadata storage for a specific key
pub struct MetadataColumn {
    /// The key this column stores
    key: MetadataKey,
    /// Instance ID to array index mapping
    indices: HashMap<InstanceId, usize>,
    /// Actual values (same type per column)
    values: MetadataColumnData,
}

/// Type-specific column storage
pub enum MetadataColumnData {
    Bool(Vec<bool>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    String(Vec<String>),
    Bytes(Vec<Vec<u8>>),
    InstanceRef(Vec<InstanceId>),
    Position(Vec<[f32; 3]>),
    Rotation(Vec<[f32; 4]>),
}

impl MetadataColumn {
    /// Create a new column for a specific value type
    pub fn new(key: MetadataKey, value_type: MetadataValue) -> Self {
        let values = match value_type {
            MetadataValue::Bool(_) => MetadataColumnData::Bool(Vec::new()),
            MetadataValue::I32(_) => MetadataColumnData::I32(Vec::new()),
            MetadataValue::I64(_) => MetadataColumnData::I64(Vec::new()),
            MetadataValue::F32(_) => MetadataColumnData::F32(Vec::new()),
            MetadataValue::F64(_) => MetadataColumnData::F64(Vec::new()),
            MetadataValue::String(_) => MetadataColumnData::String(Vec::new()),
            MetadataValue::Bytes(_) => MetadataColumnData::Bytes(Vec::new()),
            MetadataValue::InstanceRef(_) => MetadataColumnData::InstanceRef(Vec::new()),
            MetadataValue::Position(_) => MetadataColumnData::Position(Vec::new()),
            MetadataValue::Rotation(_) => MetadataColumnData::Rotation(Vec::new()),
        };
        
        Self {
            key,
            indices: HashMap::new(),
            values,
        }
    }
    
    /// Set value for an instance
    pub fn set(&mut self, id: InstanceId, value: MetadataValue) -> Result<(), &'static str> {
        match (&mut self.values, value) {
            (MetadataColumnData::Bool(vec), MetadataValue::Bool(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            (MetadataColumnData::I32(vec), MetadataValue::I32(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            (MetadataColumnData::I64(vec), MetadataValue::I64(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            (MetadataColumnData::F32(vec), MetadataValue::F32(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            (MetadataColumnData::F64(vec), MetadataValue::F64(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            (MetadataColumnData::String(vec), MetadataValue::String(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            (MetadataColumnData::Bytes(vec), MetadataValue::Bytes(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            (MetadataColumnData::InstanceRef(vec), MetadataValue::InstanceRef(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            (MetadataColumnData::Position(vec), MetadataValue::Position(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            (MetadataColumnData::Rotation(vec), MetadataValue::Rotation(v)) => {
                if let Some(&index) = self.indices.get(&id) {
                    vec[index] = v;
                } else {
                    let index = vec.len();
                    vec.push(v);
                    self.indices.insert(id, index);
                }
                Ok(())
            }
            _ => Err("Type mismatch"),
        }
    }
    
    /// Get value for an instance
    pub fn get(&self, id: &InstanceId) -> Option<MetadataValue> {
        let index = *self.indices.get(id)?;
        
        match &self.values {
            MetadataColumnData::Bool(vec) => vec.get(index).map(|&v| MetadataValue::Bool(v)),
            MetadataColumnData::I32(vec) => vec.get(index).map(|&v| MetadataValue::I32(v)),
            MetadataColumnData::I64(vec) => vec.get(index).map(|&v| MetadataValue::I64(v)),
            MetadataColumnData::F32(vec) => vec.get(index).map(|&v| MetadataValue::F32(v)),
            MetadataColumnData::F64(vec) => vec.get(index).map(|&v| MetadataValue::F64(v)),
            MetadataColumnData::String(vec) => vec.get(index).map(|v| MetadataValue::String(v.clone())),
            MetadataColumnData::Bytes(vec) => vec.get(index).map(|v| MetadataValue::Bytes(v.clone())),
            MetadataColumnData::InstanceRef(vec) => vec.get(index).map(|&v| MetadataValue::InstanceRef(v)),
            MetadataColumnData::Position(vec) => vec.get(index).map(|&v| MetadataValue::Position(v)),
            MetadataColumnData::Rotation(vec) => vec.get(index).map(|&v| MetadataValue::Rotation(v)),
        }
    }
}

/// Main metadata storage system
pub struct MetadataStore {
    /// Columns indexed by key
    columns: HashMap<MetadataKey, MetadataColumn>,
    /// Commonly used keys for quick access
    common_keys: CommonMetadataKeys,
}

/// Pre-defined common metadata keys
pub struct CommonMetadataKeys {
    pub name: MetadataKey,
    pub description: MetadataKey,
    pub owner: MetadataKey,
    pub durability: MetadataKey,
    pub stack_size: MetadataKey,
    pub custom_model: MetadataKey,
    pub tags: MetadataKey,
}

impl Default for CommonMetadataKeys {
    fn default() -> Self {
        Self {
            name: "name",
            description: "description",
            owner: "owner",
            durability: "durability",
            stack_size: "stack_size",
            custom_model: "custom_model",
            tags: "tags",
        }
    }
}

impl MetadataStore {
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
            common_keys: CommonMetadataKeys::default(),
        }
    }
    
    /// Set metadata value
    pub fn set(&mut self, id: InstanceId, key: MetadataKey, value: MetadataValue) -> Result<(), &'static str> {
        // Get or create column
        if !self.columns.contains_key(key) {
            let column = MetadataColumn::new(key, value.clone());
            self.columns.insert(key, column);
        }
        
        self.columns.get_mut(key)
            .ok_or("Failed to get metadata column")?
            .set(id, value)
    }
    
    /// Get metadata value
    pub fn get(&self, id: &InstanceId, key: MetadataKey) -> Option<MetadataValue> {
        self.columns.get(key)?.get(id)
    }
    
    /// Get all metadata for an instance
    pub fn get_all(&self, id: &InstanceId) -> HashMap<MetadataKey, MetadataValue> {
        let mut result = HashMap::new();
        
        for (key, column) in &self.columns {
            if let Some(value) = column.get(id) {
                result.insert(*key, value);
            }
        }
        
        result
    }
    
    /// Remove all metadata for an instance
    pub fn remove_instance(&mut self, id: &InstanceId) {
        for column in self.columns.values_mut() {
            column.indices.remove(id);
            // Note: We don't actually remove from arrays to maintain indices
            // This could be optimized with periodic compaction
        }
    }
    
    /// Get instances with specific metadata value
    pub fn find_by_metadata(&self, key: MetadataKey, value: &MetadataValue) -> Vec<InstanceId> {
        let mut results = Vec::new();
        
        if let Some(column) = self.columns.get(key) {
            for (id, &index) in &column.indices {
                if let Some(stored_value) = column.get(id) {
                    if &stored_value == value {
                        results.push(*id);
                    }
                }
            }
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metadata_storage() {
        let mut store = MetadataStore::new();
        let id = InstanceId::new();
        
        // Test different value types
        store.set(id, "name", MetadataValue::String("Test Item".to_string())).unwrap();
        store.set(id, "durability", MetadataValue::I32(100)).unwrap();
        store.set(id, "position", MetadataValue::Position([1.0, 2.0, 3.0])).unwrap();
        
        assert_eq!(
            store.get(&id, "name"),
            Some(MetadataValue::String("Test Item".to_string()))
        );
        assert_eq!(
            store.get(&id, "durability"),
            Some(MetadataValue::I32(100))
        );
        assert_eq!(
            store.get(&id, "position"),
            Some(MetadataValue::Position([1.0, 2.0, 3.0]))
        );
    }
    
    #[test]
    fn test_metadata_search() {
        let mut store = MetadataStore::new();
        let id1 = InstanceId::new();
        let id2 = InstanceId::new();
        let id3 = InstanceId::new();
        
        store.set(id1, "type", MetadataValue::String("sword".to_string())).unwrap();
        store.set(id2, "type", MetadataValue::String("sword".to_string())).unwrap();
        store.set(id3, "type", MetadataValue::String("shield".to_string())).unwrap();
        
        let swords = store.find_by_metadata("type", &MetadataValue::String("sword".to_string()));
        assert_eq!(swords.len(), 2);
        assert!(swords.contains(&id1));
        assert!(swords.contains(&id2));
    }
}