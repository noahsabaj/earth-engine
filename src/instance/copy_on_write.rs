/// Copy-on-Write Metadata System
/// 
/// Optimizes memory usage by sharing immutable metadata.
/// Only copies data when modifications are made.
/// Perfect for instances that share common base properties.

use crate::instance::{InstanceId, MetadataValue, MetadataKey};
use std::sync::Arc;
use std::collections::HashMap;

/// Reference-counted metadata storage
pub type SharedMetadata = Arc<HashMap<MetadataKey, MetadataValue>>;

/// Copy-on-write handle for instance metadata
pub struct CowHandle {
    /// Instance this handle is for
    instance_id: InstanceId,
    /// Base metadata (shared, immutable)
    base: Option<SharedMetadata>,
    /// Local overrides (owned, mutable)
    overrides: HashMap<MetadataKey, Option<MetadataValue>>,
    /// Version for optimistic locking
    version: u32,
}

impl CowHandle {
    /// Create new handle with optional base
    pub fn new(instance_id: InstanceId, base: Option<SharedMetadata>) -> Self {
        Self {
            instance_id,
            base,
            overrides: HashMap::new(),
            version: 0,
        }
    }
    
    /// Get metadata value (checks overrides first)
    pub fn get(&self, key: MetadataKey) -> Option<MetadataValue> {
        // Check overrides first
        if let Some(override_value) = self.overrides.get(key) {
            return override_value.clone();
        }
        
        // Fall back to base
        self.base.as_ref()?.get(key).cloned()
    }
    
    /// Set metadata value (creates override)
    pub fn set(&mut self, key: MetadataKey, value: MetadataValue) {
        self.overrides.insert(key, Some(value));
        self.version += 1;
    }
    
    /// Remove metadata value
    pub fn remove(&mut self, key: MetadataKey) {
        self.overrides.insert(key, None);
        self.version += 1;
    }
    
    /// Check if has local modifications
    pub fn is_modified(&self) -> bool {
        !self.overrides.is_empty()
    }
    
    /// Materialize all metadata (base + overrides)
    pub fn materialize(&self) -> HashMap<MetadataKey, MetadataValue> {
        let mut result = HashMap::new();
        
        // Start with base
        if let Some(ref base) = self.base {
            result.extend(base.iter().map(|(k, v)| (*k, v.clone())));
        }
        
        // Apply overrides
        for (key, value) in &self.overrides {
            match value {
                Some(v) => { result.insert(*key, v.clone()); },
                None => { result.remove(key); },
            }
        }
        
        result
    }
    
    /// Create independent copy
    pub fn fork(&self) -> Self {
        Self {
            instance_id: InstanceId::new(), // New instance
            base: Some(Arc::new(self.materialize())),
            overrides: HashMap::new(),
            version: 0,
        }
    }
}

/// Copy-on-write metadata storage
pub struct CowMetadata {
    /// Templates for common instance types
    templates: HashMap<&'static str, SharedMetadata>,
    /// Instance handles
    handles: HashMap<InstanceId, CowHandle>,
    /// Memory usage stats
    stats: CowStats,
}

/// Memory usage statistics
#[derive(Default)]
pub struct CowStats {
    /// Number of shared templates
    pub template_count: usize,
    /// Number of instances sharing templates
    pub shared_instances: usize,
    /// Number of instances with overrides
    pub modified_instances: usize,
    /// Estimated memory saved (bytes)
    pub memory_saved: usize,
}

impl CowMetadata {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            handles: HashMap::new(),
            stats: CowStats::default(),
        }
    }
    
    /// Register a template
    pub fn register_template(&mut self, name: &'static str, metadata: HashMap<MetadataKey, MetadataValue>) {
        self.templates.insert(name, Arc::new(metadata));
        self.stats.template_count = self.templates.len();
    }
    
    /// Create instance from template
    pub fn create_from_template(&mut self, id: InstanceId, template: &'static str) -> Result<(), &'static str> {
        let base = self.templates.get(template)
            .ok_or("Template not found")?
            .clone();
            
        let handle = CowHandle::new(id, Some(base));
        self.handles.insert(id, handle);
        self.update_stats();
        
        Ok(())
    }
    
    /// Create instance without template
    pub fn create_empty(&mut self, id: InstanceId) {
        let handle = CowHandle::new(id, None);
        self.handles.insert(id, handle);
        self.update_stats();
    }
    
    /// Get metadata value
    pub fn get(&self, id: &InstanceId, key: MetadataKey) -> Option<MetadataValue> {
        self.handles.get(id)?.get(key)
    }
    
    /// Set metadata value
    pub fn set(&mut self, id: InstanceId, key: MetadataKey, value: MetadataValue) -> Result<(), &'static str> {
        let handle = self.handles.get_mut(&id).ok_or("Instance not found")?;
        handle.set(key, value);
        self.update_stats();
        Ok(())
    }
    
    /// Fork an instance (create copy)
    pub fn fork(&mut self, source_id: &InstanceId) -> Result<InstanceId, &'static str> {
        let source = self.handles.get(source_id).ok_or("Instance not found")?;
        let forked = source.fork();
        let new_id = forked.instance_id;
        
        self.handles.insert(new_id, forked);
        self.update_stats();
        
        Ok(new_id)
    }
    
    /// Update statistics
    fn update_stats(&mut self) {
        self.stats.shared_instances = 0;
        self.stats.modified_instances = 0;
        self.stats.memory_saved = 0;
        
        for handle in self.handles.values() {
            if handle.base.is_some() {
                self.stats.shared_instances += 1;
                
                // Estimate memory saved
                if let Some(ref base) = handle.base {
                    let base_size = base.len() * std::mem::size_of::<(MetadataKey, MetadataValue)>();
                    let override_size = handle.overrides.len() * std::mem::size_of::<(MetadataKey, Option<MetadataValue>)>();
                    
                    if override_size < base_size {
                        self.stats.memory_saved += base_size - override_size;
                    }
                }
            }
            
            if handle.is_modified() {
                self.stats.modified_instances += 1;
            }
        }
    }
    
    /// Get memory statistics
    pub fn stats(&self) -> &CowStats {
        &self.stats
    }
}

/// Batch operations for efficiency
pub struct CowBatch<'a> {
    cow: &'a mut CowMetadata,
    operations: Vec<BatchOp>,
}

enum BatchOp {
    Set(InstanceId, MetadataKey, MetadataValue),
    Remove(InstanceId, MetadataKey),
    CreateFromTemplate(InstanceId, &'static str),
}

impl<'a> CowBatch<'a> {
    pub fn new(cow: &'a mut CowMetadata) -> Self {
        Self {
            cow,
            operations: Vec::new(),
        }
    }
    
    pub fn set(mut self, id: InstanceId, key: MetadataKey, value: MetadataValue) -> Self {
        self.operations.push(BatchOp::Set(id, key, value));
        self
    }
    
    pub fn remove(mut self, id: InstanceId, key: MetadataKey) -> Self {
        self.operations.push(BatchOp::Remove(id, key));
        self
    }
    
    pub fn create_from_template(mut self, id: InstanceId, template: &'static str) -> Self {
        self.operations.push(BatchOp::CreateFromTemplate(id, template));
        self
    }
    
    pub fn commit(self) -> Result<usize, &'static str> {
        let count = self.operations.len();
        
        for op in self.operations {
            match op {
                BatchOp::Set(id, key, value) => {
                    self.cow.set(id, key, value)?;
                }
                BatchOp::Remove(id, key) => {
                    if let Some(handle) = self.cow.handles.get_mut(&id) {
                        handle.remove(key);
                    }
                }
                BatchOp::CreateFromTemplate(id, template) => {
                    self.cow.create_from_template(id, template)?;
                }
            }
        }
        
        self.cow.update_stats();
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cow_handle() {
        let mut base = HashMap::new();
        base.insert("name", MetadataValue::String("Sword".to_string()));
        base.insert("damage", MetadataValue::I32(10));
        
        let mut handle = CowHandle::new(InstanceId::new(), Some(Arc::new(base)));
        
        // Should get base values
        assert_eq!(
            handle.get("name"),
            Some(MetadataValue::String("Sword".to_string()))
        );
        
        // Override a value
        handle.set("damage", MetadataValue::I32(15));
        assert_eq!(handle.get("damage"), Some(MetadataValue::I32(15)));
        
        // Base is unchanged
        assert!(handle.base.as_ref().expect("No base metadata found").get("damage") == Some(&MetadataValue::I32(10)));
    }
    
    #[test]
    fn test_template_system() {
        let mut cow = CowMetadata::new();
        
        // Register sword template
        let mut sword_template = HashMap::new();
        sword_template.insert("type", MetadataValue::String("weapon".to_string()));
        sword_template.insert("damage", MetadataValue::I32(10));
        sword_template.insert("durability", MetadataValue::I32(100));
        
        cow.register_template("sword", sword_template);
        
        // Create instances
        let id1 = InstanceId::new();
        let id2 = InstanceId::new();
        
        cow.create_from_template(id1, "sword").expect("Failed to create instance from sword template");
        cow.create_from_template(id2, "sword").expect("Failed to create second instance from sword template");
        
        // Both share template
        assert_eq!(
            cow.get(&id1, "damage"),
            Some(MetadataValue::I32(10))
        );
        assert_eq!(
            cow.get(&id2, "damage"),
            Some(MetadataValue::I32(10))
        );
        
        // Modify one
        cow.set(id1, "damage", MetadataValue::I32(15)).expect("Failed to set damage metadata");
        
        // Only one changed
        assert_eq!(cow.get(&id1, "damage"), Some(MetadataValue::I32(15)));
        assert_eq!(cow.get(&id2, "damage"), Some(MetadataValue::I32(10)));
        
        // Check stats
        assert_eq!(cow.stats().shared_instances, 2);
        assert_eq!(cow.stats().modified_instances, 1);
    }
}