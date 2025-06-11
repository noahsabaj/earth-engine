/// Attribute Storage Backend
/// 
/// Efficient storage for dynamic attributes using columnar layout.
/// Supports string keys with fast lookups and type-safe access.

use crate::instance::InstanceId;
use crate::attributes::AttributeValue;
use crate::error::{EngineError, EngineResult};
use std::collections::{HashMap, BTreeMap};
use serde::{Serialize, Deserialize};

/// Attribute key type (string for flexibility)
pub type AttributeKey = String;

/// Attribute index for fast access
pub type AttributeIndex = u32;

/// Columnar attribute storage
pub struct AttributeStorage {
    /// Key to index mapping
    key_index: HashMap<AttributeKey, AttributeIndex>,
    
    /// Index to key mapping (for iteration)
    index_key: Vec<AttributeKey>,
    
    /// Instance attribute data (sparse)
    instance_data: HashMap<InstanceId, InstanceAttributes>,
    
    /// Global attribute columns (for bulk operations)
    columns: HashMap<AttributeIndex, AttributeColumn>,
    
    /// Next available index
    next_index: AttributeIndex,
}

/// Attributes for a single instance
#[derive(Default)]
struct InstanceAttributes {
    /// Sparse map of attribute indices to values
    values: BTreeMap<AttributeIndex, AttributeValue>,
    
    /// Dirty flags for change tracking
    dirty: Vec<AttributeIndex>,
    
    /// Version for optimistic locking
    version: u32,
}

/// Column storage for a single attribute type
struct AttributeColumn {
    /// All instances with this attribute
    instances: Vec<InstanceId>,
    
    /// Values in same order as instances
    values: Vec<AttributeValue>,
    
    /// Index lookup for O(1) access
    instance_index: HashMap<InstanceId, usize>,
}

impl AttributeStorage {
    pub fn new() -> Self {
        Self {
            key_index: HashMap::new(),
            index_key: Vec::new(),
            instance_data: HashMap::new(),
            columns: HashMap::new(),
            next_index: 0,
        }
    }
    
    /// Register a new attribute key
    pub fn register_key(&mut self, key: AttributeKey) -> AttributeIndex {
        if let Some(&index) = self.key_index.get(&key) {
            return index;
        }
        
        let index = self.next_index;
        self.next_index += 1;
        
        self.key_index.insert(key.clone(), index);
        self.index_key.push(key);
        self.columns.insert(index, AttributeColumn {
            instances: Vec::new(),
            values: Vec::new(),
            instance_index: HashMap::new(),
        });
        
        index
    }
    
    /// Get attribute index from key
    pub fn get_index(&self, key: &AttributeKey) -> Option<AttributeIndex> {
        self.key_index.get(key).copied()
    }
    
    /// Get key from index
    pub fn get_key(&self, index: AttributeIndex) -> Option<&AttributeKey> {
        self.index_key.get(index as usize)
    }
    
    /// Set attribute value
    pub fn set(
        &mut self,
        instance: InstanceId,
        key: AttributeKey,
        value: AttributeValue,
    ) -> Result<(), String> {
        // Get or create attribute index
        let index = if let Some(idx) = self.get_index(&key) {
            idx
        } else {
            self.register_key(key)
        };
        
        // Update instance data
        let inst_attrs = self.instance_data
            .entry(instance)
            .or_insert_with(InstanceAttributes::default);
            
        let old_value = inst_attrs.values.insert(index, value.clone());
        inst_attrs.version += 1;
        
        // Mark dirty if changed
        if old_value.as_ref() != Some(&value) {
            if !inst_attrs.dirty.contains(&index) {
                inst_attrs.dirty.push(index);
            }
        }
        
        // Update column storage
        self.update_column(instance, index, value);
        
        Ok(())
    }
    
    /// Get attribute value
    pub fn get(&self, instance: InstanceId, key: &AttributeKey) -> Option<&AttributeValue> {
        let index = self.get_index(key)?;
        let attrs = self.instance_data.get(&instance)?;
        attrs.values.get(&index)
    }
    
    /// Get mutable attribute value
    pub fn get_mut(
        &mut self,
        instance: InstanceId,
        key: &AttributeKey,
    ) -> Option<&mut AttributeValue> {
        let index = self.get_index(key)?;
        let attrs = self.instance_data.get_mut(&instance)?;
        attrs.values.get_mut(&index)
    }
    
    /// Remove attribute
    pub fn remove(&mut self, instance: InstanceId, key: &AttributeKey) -> Option<AttributeValue> {
        let index = self.get_index(key)?;
        let attrs = self.instance_data.get_mut(&instance)?;
        
        let value = attrs.values.remove(&index);
        if value.is_some() {
            attrs.version += 1;
            self.remove_from_column(instance, index);
        }
        
        value
    }
    
    /// Get all attributes for an instance
    pub fn get_all(&self, instance: InstanceId) -> HashMap<AttributeKey, AttributeValue> {
        let mut result = HashMap::new();
        
        if let Some(attrs) = self.instance_data.get(&instance) {
            for (&index, value) in &attrs.values {
                if let Some(key) = self.get_key(index) {
                    result.insert(key.clone(), value.clone());
                }
            }
        }
        
        result
    }
    
    /// Get instances with a specific attribute
    pub fn get_instances_with(&self, key: &AttributeKey) -> Vec<InstanceId> {
        if let Some(index) = self.get_index(key) {
            if let Some(column) = self.columns.get(&index) {
                return column.instances.clone();
            }
        }
        Vec::new()
    }
    
    /// Update column storage
    fn update_column(&mut self, instance: InstanceId, index: AttributeIndex, value: AttributeValue) {
        let column = match self.columns.get_mut(&index) {
            Some(col) => col,
            None => return, // Silently ignore if column not found
        };
        
        if let Some(&pos) = column.instance_index.get(&instance) {
            // Update existing
            column.values[pos] = value;
        } else {
            // Add new
            let pos = column.instances.len();
            column.instances.push(instance);
            column.values.push(value);
            column.instance_index.insert(instance, pos);
        }
    }
    
    /// Remove from column storage
    fn remove_from_column(&mut self, instance: InstanceId, index: AttributeIndex) {
        if let Some(column) = self.columns.get_mut(&index) {
            if let Some(&pos) = column.instance_index.get(&instance) {
                // Swap remove for efficiency
                column.instances.swap_remove(pos);
                column.values.swap_remove(pos);
                column.instance_index.remove(&instance);
                
                // Update moved instance's index
                if pos < column.instances.len() {
                    let moved_instance = column.instances[pos];
                    column.instance_index.insert(moved_instance, pos);
                }
            }
        }
    }
    
    /// Clear all attributes for an instance
    pub fn clear_instance(&mut self, instance: InstanceId) {
        if let Some(attrs) = self.instance_data.remove(&instance) {
            // Remove from all columns
            for &index in attrs.values.keys() {
                self.remove_from_column(instance, index);
            }
        }
    }
    
    /// Get dirty attributes for an instance
    pub fn get_dirty(&mut self, instance: InstanceId) -> Vec<(AttributeKey, AttributeValue)> {
        let mut result = Vec::new();
        
        if let Some(attrs) = self.instance_data.get_mut(&instance) {
            for &index in &attrs.dirty {
                if let Some(key) = self.get_key(index) {
                    if let Some(value) = attrs.values.get(&index) {
                        result.push((key.clone(), value.clone()));
                    }
                }
            }
            
            // Clear dirty flags
            attrs.dirty.clear();
        }
        
        result
    }
    
    /// Get version for optimistic locking
    pub fn get_version(&self, instance: InstanceId) -> u32 {
        self.instance_data.get(&instance)
            .map(|attrs| attrs.version)
            .unwrap_or(0)
    }
}

/// Serializable attribute snapshot
#[derive(Serialize, Deserialize)]
pub struct AttributeSnapshot {
    pub instance: InstanceId,
    pub attributes: HashMap<AttributeKey, AttributeValue>,
    pub version: u32,
}

impl AttributeStorage {
    /// Create snapshot for serialization
    pub fn create_snapshot(&self, instance: InstanceId) -> Option<AttributeSnapshot> {
        let attrs = self.instance_data.get(&instance)?;
        let mut attributes = HashMap::new();
        
        for (&index, value) in &attrs.values {
            if let Some(key) = self.get_key(index) {
                attributes.insert(key.clone(), value.clone());
            }
        }
        
        Some(AttributeSnapshot {
            instance,
            attributes,
            version: attrs.version,
        })
    }
    
    /// Restore from snapshot
    pub fn restore_snapshot(&mut self, snapshot: AttributeSnapshot) -> Result<(), String> {
        // Clear existing
        self.clear_instance(snapshot.instance);
        
        // Restore attributes
        for (key, value) in snapshot.attributes {
            self.set(snapshot.instance, key, value)?;
        }
        
        // Set version
        if let Some(attrs) = self.instance_data.get_mut(&snapshot.instance) {
            attrs.version = snapshot.version;
        }
        
        Ok(())
    }
}

/// Storage statistics
pub struct StorageStats {
    pub total_instances: usize,
    pub total_attributes: usize,
    pub unique_keys: usize,
    pub memory_usage: usize,
}

impl AttributeStorage {
    /// Get storage statistics
    pub fn stats(&self) -> StorageStats {
        let mut total_attributes = 0;
        
        for attrs in self.instance_data.values() {
            total_attributes += attrs.values.len();
        }
        
        StorageStats {
            total_instances: self.instance_data.len(),
            total_attributes,
            unique_keys: self.key_index.len(),
            memory_usage: self.estimate_memory_usage(),
        }
    }
    
    /// Estimate memory usage in bytes
    fn estimate_memory_usage(&self) -> usize {
        let mut total = 0;
        
        // Key mappings
        total += self.key_index.len() * (32 + 4); // Assume 32 byte keys
        total += self.index_key.len() * 32;
        
        // Instance data
        for attrs in self.instance_data.values() {
            total += attrs.values.len() * (4 + 32); // Index + value size
        }
        
        // Column storage
        for column in self.columns.values() {
            total += column.instances.len() * 16; // InstanceId size
            total += column.values.len() * 32; // AttributeValue size
        }
        
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_attribute_storage() {
        let mut storage = AttributeStorage::new();
        let instance = InstanceId::new();
        
        // Set attributes
        storage.set(instance, "health".to_string(), AttributeValue::Float(100.0)).unwrap();
        storage.set(instance, "name".to_string(), AttributeValue::String("Player".to_string())).unwrap();
        
        // Get attributes
        assert_eq!(
            storage.get(instance, &"health".to_string()),
            Some(&AttributeValue::Float(100.0))
        );
        
        // Get all
        let all = storage.get_all(instance);
        assert_eq!(all.len(), 2);
    }
    
    #[test]
    fn test_column_storage() {
        let mut storage = AttributeStorage::new();
        let id1 = InstanceId::new();
        let id2 = InstanceId::new();
        
        storage.set(id1, "level".to_string(), AttributeValue::Integer(10)).unwrap();
        storage.set(id2, "level".to_string(), AttributeValue::Integer(15)).unwrap();
        
        let instances = storage.get_instances_with(&"level".to_string());
        assert_eq!(instances.len(), 2);
    }
    
    #[test]
    fn test_dirty_tracking() {
        let mut storage = AttributeStorage::new();
        let instance = InstanceId::new();
        
        storage.set(instance, "health".to_string(), AttributeValue::Float(100.0)).unwrap();
        storage.set(instance, "mana".to_string(), AttributeValue::Float(50.0)).unwrap();
        
        let dirty = storage.get_dirty(instance);
        assert_eq!(dirty.len(), 2);
        
        // Should be empty after getting
        let dirty2 = storage.get_dirty(instance);
        assert_eq!(dirty2.len(), 0);
    }
}