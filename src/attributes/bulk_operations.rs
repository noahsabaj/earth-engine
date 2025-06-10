/// Bulk Attribute Operations
/// 
/// Efficient operations on multiple attributes or instances at once.
/// Optimized for cache-friendly access patterns.

use crate::instance::InstanceId;
use crate::attributes::{
    AttributeKey, AttributeValue, AttributeManager,
    AttributeCategory, Modifier, ModifierType
};
use rayon::prelude::*;
use std::collections::HashMap;

/// Bulk update operation
#[derive(Debug, Clone)]
pub struct BulkUpdate {
    /// Target instances
    pub targets: TargetSelection,
    
    /// Operations to perform
    pub operations: Vec<BulkOperation>,
    
    /// Whether to trigger events
    pub silent: bool,
    
    /// Transaction mode
    pub atomic: bool,
}

/// Target selection for bulk operations
#[derive(Debug, Clone)]
pub enum TargetSelection {
    /// Specific instances
    Instances(Vec<InstanceId>),
    
    /// All instances with attribute
    WithAttribute(AttributeKey),
    
    /// All instances of category
    Category(AttributeCategory),
    
    /// All instances matching predicate
    Predicate(AttributePredicate),
    
    /// All instances
    All,
}

/// Attribute predicate for filtering
#[derive(Debug, Clone)]
pub enum AttributePredicate {
    /// Attribute equals value
    Equals(AttributeKey, AttributeValue),
    
    /// Attribute greater than value
    GreaterThan(AttributeKey, AttributeValue),
    
    /// Attribute less than value
    LessThan(AttributeKey, AttributeValue),
    
    /// Attribute in range
    InRange(AttributeKey, AttributeValue, AttributeValue),
    
    /// Has attribute
    HasAttribute(AttributeKey),
    
    /// Combine with AND
    And(Box<AttributePredicate>, Box<AttributePredicate>),
    
    /// Combine with OR
    Or(Box<AttributePredicate>, Box<AttributePredicate>),
    
    /// Negate
    Not(Box<AttributePredicate>),
}

/// Bulk operation types
#[derive(Debug, Clone)]
pub enum BulkOperation {
    /// Set attribute value
    Set(AttributeKey, AttributeValue),
    
    /// Add to numeric attribute
    Add(AttributeKey, AttributeValue),
    
    /// Multiply numeric attribute
    Multiply(AttributeKey, f64),
    
    /// Remove attribute
    Remove(AttributeKey),
    
    /// Apply modifier
    ApplyModifier(AttributeKey, Modifier),
    
    /// Remove modifiers of type
    RemoveModifiers(AttributeKey, ModifierType),
    
    /// Copy attribute from another
    Copy(AttributeKey, AttributeKey),
    
    /// Swap two attributes
    Swap(AttributeKey, AttributeKey),
}

/// Bulk query operation
#[derive(Debug)]
pub struct BulkQuery {
    /// Target selection
    pub targets: TargetSelection,
    
    /// Attributes to retrieve
    pub attributes: AttributeSelection,
    
    /// Sorting
    pub sort_by: Option<(AttributeKey, SortOrder)>,
    
    /// Limit results
    pub limit: Option<usize>,
    
    /// Parallel execution
    pub parallel: bool,
}

/// Attribute selection for queries
#[derive(Debug, Clone)]
pub enum AttributeSelection {
    /// Specific attributes
    Keys(Vec<AttributeKey>),
    
    /// All attributes
    All,
    
    /// Attributes of category
    Category(AttributeCategory),
}

/// Sort order
#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Result of bulk operation
#[derive(Debug)]
pub struct BulkResult {
    /// Number of instances affected
    pub affected_count: usize,
    
    /// Number of attributes modified
    pub modified_attributes: usize,
    
    /// Errors encountered
    pub errors: Vec<(InstanceId, String)>,
    
    /// Execution time in microseconds
    pub execution_time_us: u64,
}

/// Bulk operation executor
pub struct BulkExecutor;

impl BulkExecutor {
    /// Execute bulk update
    pub fn execute_update(
        update: &BulkUpdate,
        manager: &mut AttributeManager,
    ) -> BulkResult {
        let start = std::time::Instant::now();
        let mut result = BulkResult {
            affected_count: 0,
            modified_attributes: 0,
            errors: Vec::new(),
            execution_time_us: 0,
        };
        
        // Get target instances
        let instances = Self::resolve_targets(&update.targets, manager);
        result.affected_count = instances.len();
        
        if update.atomic {
            // Atomic mode - all or nothing
            let mut changes = Vec::new();
            
            for instance in &instances {
                for operation in &update.operations {
                    match Self::prepare_operation(*instance, operation, manager) {
                        Ok(change) => changes.push((*instance, change)),
                        Err(e) => {
                            result.errors.push((*instance, e));
                            result.execution_time_us = start.elapsed().as_micros() as u64;
                            return result; // Rollback
                        }
                    }
                }
            }
            
            // Apply all changes
            for (instance, change) in changes {
                Self::apply_change(instance, change, manager, update.silent);
                result.modified_attributes += 1;
            }
        } else {
            // Non-atomic mode - best effort
            for instance in instances {
                for operation in &update.operations {
                    match Self::prepare_operation(instance, operation, manager) {
                        Ok(change) => {
                            Self::apply_change(instance, change, manager, update.silent);
                            result.modified_attributes += 1;
                        }
                        Err(e) => {
                            result.errors.push((instance, e));
                        }
                    }
                }
            }
        }
        
        result.execution_time_us = start.elapsed().as_micros() as u64;
        result
    }
    
    /// Execute bulk query
    pub fn execute_query(
        query: &BulkQuery,
        manager: &AttributeManager,
    ) -> Vec<(InstanceId, HashMap<AttributeKey, AttributeValue>)> {
        let instances = Self::resolve_targets(&query.targets, manager);
        
        let mut results: Vec<_> = if query.parallel && instances.len() > 1000 {
            // Parallel execution for large queries
            instances.par_iter()
                .map(|&instance| {
                    let attrs = Self::get_attributes(instance, &query.attributes, manager);
                    (instance, attrs)
                })
                .collect()
        } else {
            // Sequential for small queries
            instances.iter()
                .map(|&instance| {
                    let attrs = Self::get_attributes(instance, &query.attributes, manager);
                    (instance, attrs)
                })
                .collect()
        };
        
        // Sort if requested
        if let Some((ref key, order)) = query.sort_by {
            results.sort_by(|(_, attrs1), (_, attrs2)| {
                let v1 = attrs1.get(key);
                let v2 = attrs2.get(key);
                
                let ordering = match (v1, v2) {
                    (Some(a), Some(b)) => a.compare(b).unwrap_or(std::cmp::Ordering::Equal),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                };
                
                match order {
                    SortOrder::Ascending => ordering,
                    SortOrder::Descending => ordering.reverse(),
                }
            });
        }
        
        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        
        results
    }
    
    /// Resolve target instances
    fn resolve_targets(
        targets: &TargetSelection,
        manager: &AttributeManager,
    ) -> Vec<InstanceId> {
        match targets {
            TargetSelection::Instances(ids) => ids.clone(),
            
            TargetSelection::WithAttribute(key) => {
                manager.storage.get_instances_with(key)
            }
            
            TargetSelection::Category(category) => {
                let mut instances = Vec::new();
                if let Some(keys) = manager.metadata.by_category.get(category) {
                    for key in keys {
                        instances.extend(manager.storage.get_instances_with(key));
                    }
                }
                instances.sort();
                instances.dedup();
                instances
            }
            
            TargetSelection::Predicate(pred) => {
                // Would iterate all instances and filter
                // For now, return empty
                Vec::new()
            }
            
            TargetSelection::All => {
                // Would return all instances
                // For now, return empty
                Vec::new()
            }
        }
    }
    
    /// Prepare operation (validate and compute change)
    fn prepare_operation(
        instance: InstanceId,
        operation: &BulkOperation,
        manager: &AttributeManager,
    ) -> Result<PreparedChange, String> {
        match operation {
            BulkOperation::Set(key, value) => {
                Ok(PreparedChange::Set(key.clone(), value.clone()))
            }
            
            BulkOperation::Add(key, value) => {
                if let Some(current) = manager.get_attribute(instance, key) {
                    if let Some(new_value) = current.add(value) {
                        Ok(PreparedChange::Set(key.clone(), new_value))
                    } else {
                        Err("Cannot add values".to_string())
                    }
                } else {
                    Ok(PreparedChange::Set(key.clone(), value.clone()))
                }
            }
            
            BulkOperation::Multiply(key, scalar) => {
                if let Some(current) = manager.get_attribute(instance, key) {
                    if let Some(new_value) = current.multiply(*scalar) {
                        Ok(PreparedChange::Set(key.clone(), new_value))
                    } else {
                        Err("Cannot multiply value".to_string())
                    }
                } else {
                    Err("Attribute not found".to_string())
                }
            }
            
            BulkOperation::Remove(key) => {
                Ok(PreparedChange::Remove(key.clone()))
            }
            
            BulkOperation::ApplyModifier(key, modifier) => {
                Ok(PreparedChange::AddModifier(key.clone(), modifier.clone()))
            }
            
            BulkOperation::RemoveModifiers(key, mod_type) => {
                Ok(PreparedChange::RemoveModifiers(key.clone(), *mod_type))
            }
            
            BulkOperation::Copy(from_key, to_key) => {
                if let Some(value) = manager.get_attribute(instance, from_key) {
                    Ok(PreparedChange::Set(to_key.clone(), value))
                } else {
                    Err("Source attribute not found".to_string())
                }
            }
            
            BulkOperation::Swap(key1, key2) => {
                let val1 = manager.get_attribute(instance, key1);
                let val2 = manager.get_attribute(instance, key2);
                
                Ok(PreparedChange::Swap(
                    key1.clone(),
                    val1,
                    key2.clone(),
                    val2,
                ))
            }
        }
    }
    
    /// Apply prepared change
    fn apply_change(
        instance: InstanceId,
        change: PreparedChange,
        manager: &mut AttributeManager,
        silent: bool,
    ) {
        // Disable events if silent
        let old_observable = if silent {
            // Would temporarily disable events
            true
        } else {
            true
        };
        
        match change {
            PreparedChange::Set(key, value) => {
                let _ = manager.set_attribute(instance, key, value);
            }
            
            PreparedChange::Remove(key) => {
                manager.storage.remove(instance, &key);
            }
            
            PreparedChange::AddModifier(key, modifier) => {
                let _ = manager.add_modifier(instance, key, modifier);
            }
            
            PreparedChange::RemoveModifiers(key, mod_type) => {
                if let Some(stack) = manager.modifiers.get_mut(&(instance, key)) {
                    stack.clear_type(mod_type);
                }
            }
            
            PreparedChange::Swap(key1, val1, key2, val2) => {
                if let Some(v1) = val1 {
                    let _ = manager.set_attribute(instance, key2, v1);
                }
                if let Some(v2) = val2 {
                    let _ = manager.set_attribute(instance, key1, v2);
                }
            }
        }
    }
    
    /// Get attributes for instance
    fn get_attributes(
        instance: InstanceId,
        selection: &AttributeSelection,
        manager: &AttributeManager,
    ) -> HashMap<AttributeKey, AttributeValue> {
        match selection {
            AttributeSelection::Keys(keys) => {
                let mut attrs = HashMap::new();
                for key in keys {
                    if let Some(value) = manager.get_attribute(instance, key) {
                        attrs.insert(key.clone(), value);
                    }
                }
                attrs
            }
            
            AttributeSelection::All => {
                manager.storage.get_all(instance)
            }
            
            AttributeSelection::Category(category) => {
                let mut attrs = HashMap::new();
                if let Some(keys) = manager.metadata.by_category.get(category) {
                    for key in keys {
                        if let Some(value) = manager.get_attribute(instance, key) {
                            attrs.insert(key.clone(), value);
                        }
                    }
                }
                attrs
            }
        }
    }
}

/// Prepared change for atomic operations
enum PreparedChange {
    Set(AttributeKey, AttributeValue),
    Remove(AttributeKey),
    AddModifier(AttributeKey, Modifier),
    RemoveModifiers(AttributeKey, ModifierType),
    Swap(AttributeKey, Option<AttributeValue>, AttributeKey, Option<AttributeValue>),
}

/// Bulk operation builder
pub struct BulkUpdateBuilder {
    update: BulkUpdate,
}

impl BulkUpdateBuilder {
    pub fn new() -> Self {
        Self {
            update: BulkUpdate {
                targets: TargetSelection::All,
                operations: Vec::new(),
                silent: false,
                atomic: false,
            },
        }
    }
    
    pub fn targets(mut self, targets: TargetSelection) -> Self {
        self.update.targets = targets;
        self
    }
    
    pub fn set(mut self, key: AttributeKey, value: AttributeValue) -> Self {
        self.update.operations.push(BulkOperation::Set(key, value));
        self
    }
    
    pub fn add(mut self, key: AttributeKey, value: AttributeValue) -> Self {
        self.update.operations.push(BulkOperation::Add(key, value));
        self
    }
    
    pub fn multiply(mut self, key: AttributeKey, scalar: f64) -> Self {
        self.update.operations.push(BulkOperation::Multiply(key, scalar));
        self
    }
    
    pub fn silent(mut self) -> Self {
        self.update.silent = true;
        self
    }
    
    pub fn atomic(mut self) -> Self {
        self.update.atomic = true;
        self
    }
    
    pub fn build(self) -> BulkUpdate {
        self.update
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bulk_update_builder() {
        let update = BulkUpdateBuilder::new()
            .targets(TargetSelection::WithAttribute("health".to_string()))
            .add("health".to_string(), AttributeValue::Float(10.0))
            .multiply("damage".to_string(), 1.5)
            .atomic()
            .build();
            
        assert_eq!(update.operations.len(), 2);
        assert!(update.atomic);
    }
}