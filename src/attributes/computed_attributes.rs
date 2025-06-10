/// Computed Attributes System
/// 
/// Attributes that are calculated from other attributes.
/// Supports dependency tracking and automatic invalidation.

use crate::instance::InstanceId;
use crate::attributes::{AttributeKey, AttributeValue, AttributeManager};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// Compute function signature
pub type ComputeFunction = Arc<dyn Fn(InstanceId, &AttributeManager) -> Option<AttributeValue> + Send + Sync>;

/// Computed attribute definition
pub struct ComputedAttribute {
    /// Attribute key
    pub key: AttributeKey,
    
    /// Display name
    pub name: String,
    
    /// Dependencies (attributes this depends on)
    pub dependencies: Vec<AttributeKey>,
    
    /// Compute function
    pub compute_fn: ComputeFunction,
    
    /// Cache policy
    pub cache_policy: CachePolicy,
    
    /// Description
    pub description: String,
}

impl ComputedAttribute {
    pub fn new(
        key: AttributeKey,
        dependencies: Vec<AttributeKey>,
        compute_fn: ComputeFunction,
    ) -> Self {
        Self {
            key: key.clone(),
            name: key,
            dependencies,
            compute_fn,
            cache_policy: CachePolicy::default(),
            description: String::new(),
        }
    }
    
    /// Compute the value
    pub fn compute(&self, instance: InstanceId, manager: &AttributeManager) -> Option<AttributeValue> {
        (self.compute_fn)(instance, manager)
    }
    
    /// Set cache policy
    pub fn with_cache_policy(mut self, policy: CachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }
    
    /// Set description
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }
}

/// Cache policy for computed attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicy {
    /// Enable caching
    pub enabled: bool,
    
    /// Cache duration in ticks
    pub duration: Option<u64>,
    
    /// Invalidate on dependency change
    pub invalidate_on_change: bool,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            duration: None,
            invalidate_on_change: true,
        }
    }
}

/// Dependency graph for computed attributes
pub struct DependencyGraph {
    /// Forward dependencies (A depends on B, C)
    dependencies: HashMap<AttributeKey, HashSet<AttributeKey>>,
    
    /// Reverse dependencies (B is depended on by A)
    dependents: HashMap<AttributeKey, HashSet<AttributeKey>>,
    
    /// Cached values
    cache: HashMap<(InstanceId, AttributeKey), CachedValue>,
    
    /// Topological order (for evaluation)
    topo_order: Vec<AttributeKey>,
    
    /// Needs recomputation
    dirty: bool,
}

/// Cached computed value
struct CachedValue {
    value: AttributeValue,
    computed_at: std::time::Instant,
    version: u32,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
            cache: HashMap::new(),
            topo_order: Vec::new(),
            dirty: true,
        }
    }
    
    /// Register computed attribute
    pub fn register(&mut self, key: AttributeKey, deps: Vec<AttributeKey>) {
        // Add forward dependencies
        self.dependencies.insert(key.clone(), deps.iter().cloned().collect());
        
        // Add reverse dependencies
        for dep in deps {
            self.dependents
                .entry(dep)
                .or_insert_with(HashSet::new)
                .insert(key.clone());
        }
        
        self.dirty = true;
    }
    
    /// Unregister computed attribute
    pub fn unregister(&mut self, key: &AttributeKey) {
        if let Some(deps) = self.dependencies.remove(key) {
            // Remove from reverse dependencies
            for dep in deps {
                if let Some(dependents) = self.dependents.get_mut(&dep) {
                    dependents.remove(key);
                }
            }
        }
        
        // Remove as dependent
        self.dependents.remove(key);
        
        // Clear cache
        self.cache.retain(|(_, k), _| k != key);
        
        self.dirty = true;
    }
    
    /// Invalidate dependents of changed attribute
    pub fn invalidate_dependents(&mut self, changed_key: &AttributeKey) {
        let mut to_invalidate = VecDeque::new();
        to_invalidate.push_back(changed_key.clone());
        
        let mut invalidated = HashSet::new();
        
        while let Some(key) = to_invalidate.pop_front() {
            if !invalidated.insert(key.clone()) {
                continue; // Already processed
            }
            
            // Clear cache for this key
            self.cache.retain(|(_, k), _| k != &key);
            
            // Add dependents to queue
            if let Some(dependents) = self.dependents.get(&key) {
                for dep in dependents {
                    to_invalidate.push_back(dep.clone());
                }
            }
        }
    }
    
    /// Get cached value
    pub fn get_cached(
        &self,
        instance: InstanceId,
        key: &AttributeKey,
    ) -> Option<&AttributeValue> {
        self.cache.get(&(instance, key.clone()))
            .map(|cached| &cached.value)
    }
    
    /// Set cached value
    pub fn set_cached(
        &mut self,
        instance: InstanceId,
        key: AttributeKey,
        value: AttributeValue,
    ) {
        self.cache.insert(
            (instance, key),
            CachedValue {
                value,
                computed_at: std::time::Instant::now(),
                version: 0,
            },
        );
    }
    
    /// Compute topological order
    pub fn compute_order(&mut self) -> &[AttributeKey] {
        if !self.dirty {
            return &self.topo_order;
        }
        
        self.topo_order.clear();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        
        // DFS for topological sort
        for key in self.dependencies.keys() {
            if !visited.contains(key) {
                self.dfs_visit(key, &mut visited, &mut stack);
            }
        }
        
        // Reverse to get correct order
        self.topo_order = stack.into_iter().rev().collect();
        self.dirty = false;
        
        &self.topo_order
    }
    
    fn dfs_visit(
        &self,
        key: &AttributeKey,
        visited: &mut HashSet<AttributeKey>,
        stack: &mut Vec<AttributeKey>,
    ) {
        visited.insert(key.clone());
        
        if let Some(deps) = self.dependencies.get(key) {
            for dep in deps {
                if !visited.contains(dep) {
                    self.dfs_visit(dep, visited, stack);
                }
            }
        }
        
        stack.push(key.clone());
    }
    
    /// Check for cycles
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for key in self.dependencies.keys() {
            if !visited.contains(key) {
                if self.has_cycle_util(key, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        
        false
    }
    
    fn has_cycle_util(
        &self,
        key: &AttributeKey,
        visited: &mut HashSet<AttributeKey>,
        rec_stack: &mut HashSet<AttributeKey>,
    ) -> bool {
        visited.insert(key.clone());
        rec_stack.insert(key.clone());
        
        if let Some(deps) = self.dependencies.get(key) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_cycle_util(dep, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true; // Cycle detected
                }
            }
        }
        
        rec_stack.remove(key);
        false
    }
    
    /// Get computation statistics
    pub fn stats(&self) -> DependencyStats {
        DependencyStats {
            total_attributes: self.dependencies.len(),
            cached_values: self.cache.len(),
            max_dependency_depth: self.compute_max_depth(),
        }
    }
    
    fn compute_max_depth(&self) -> usize {
        let mut max_depth = 0;
        let mut depths = HashMap::new();
        
        for key in self.dependencies.keys() {
            let depth = self.compute_depth(key, &mut depths);
            max_depth = max_depth.max(depth);
        }
        
        max_depth
    }
    
    fn compute_depth(
        &self,
        key: &AttributeKey,
        depths: &mut HashMap<AttributeKey, usize>,
    ) -> usize {
        if let Some(&depth) = depths.get(key) {
            return depth;
        }
        
        let depth = if let Some(deps) = self.dependencies.get(key) {
            1 + deps.iter()
                .map(|dep| self.compute_depth(dep, depths))
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        
        depths.insert(key.clone(), depth);
        depth
    }
}

/// Dependency statistics
pub struct DependencyStats {
    pub total_attributes: usize,
    pub cached_values: usize,
    pub max_dependency_depth: usize,
}

/// Common computed attribute templates
pub struct ComputedTemplates;

impl ComputedTemplates {
    /// Max health from level and constitution
    pub fn max_health() -> ComputedAttribute {
        ComputedAttribute::new(
            "max_health".to_string(),
            vec!["level".to_string(), "constitution".to_string()],
            Arc::new(|instance, manager| {
                let level = manager.get_attribute(instance, &"level".to_string())?
                    .as_integer()?;
                let con = manager.get_attribute(instance, &"constitution".to_string())?
                    .as_integer()?;
                    
                Some(AttributeValue::Integer(100 + level * 10 + con * 5))
            }),
        )
        .with_description("Maximum health based on level and constitution".to_string())
    }
    
    /// Attack power from strength and weapon
    pub fn attack_power() -> ComputedAttribute {
        ComputedAttribute::new(
            "attack_power".to_string(),
            vec!["strength".to_string(), "weapon_damage".to_string()],
            Arc::new(|instance, manager| {
                let str = manager.get_attribute(instance, &"strength".to_string())?
                    .as_float()?;
                let weapon = manager.get_attribute(instance, &"weapon_damage".to_string())?
                    .as_float()?;
                    
                Some(AttributeValue::Float(str * 2.0 + weapon))
            }),
        )
    }
    
    /// Movement speed with encumbrance
    pub fn movement_speed() -> ComputedAttribute {
        ComputedAttribute::new(
            "movement_speed".to_string(),
            vec!["base_speed".to_string(), "carry_weight".to_string(), "max_carry_weight".to_string()],
            Arc::new(|instance, manager| {
                let base = manager.get_attribute(instance, &"base_speed".to_string())?
                    .as_float()?;
                let carry = manager.get_attribute(instance, &"carry_weight".to_string())?
                    .as_float()?;
                let max_carry = manager.get_attribute(instance, &"max_carry_weight".to_string())?
                    .as_float()?;
                    
                let encumbrance = (carry / max_carry).clamp(0.0, 1.0);
                let speed = base * (1.0 - encumbrance * 0.5);
                
                Some(AttributeValue::Float(speed))
            }),
        )
    }
    
    /// Critical hit chance
    pub fn crit_chance() -> ComputedAttribute {
        ComputedAttribute::new(
            "crit_chance".to_string(),
            vec!["dexterity".to_string(), "luck".to_string()],
            Arc::new(|instance, manager| {
                let dex = manager.get_attribute(instance, &"dexterity".to_string())?
                    .as_float()?;
                let luck = manager.get_attribute(instance, &"luck".to_string())?
                    .as_float()?;
                    
                let chance = (dex * 0.5 + luck * 2.0) / 100.0;
                
                Some(AttributeValue::Float(chance.clamp(0.0, 0.5)))
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        
        // Register computed attributes
        graph.register("max_health".to_string(), vec!["level".to_string(), "constitution".to_string()]);
        graph.register("defense".to_string(), vec!["armor".to_string(), "dexterity".to_string()]);
        
        // Check no cycles
        assert!(!graph.has_cycle());
        
        // Compute order
        let order = graph.compute_order();
        assert!(order.len() >= 2);
    }
    
    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        
        // Create cycle: A -> B -> C -> A
        graph.register("A".to_string(), vec!["B".to_string()]);
        graph.register("B".to_string(), vec!["C".to_string()]);
        graph.register("C".to_string(), vec!["A".to_string()]);
        
        assert!(graph.has_cycle());
    }
    
    #[test]
    fn test_invalidation() {
        let mut graph = DependencyGraph::new();
        
        graph.register("max_health".to_string(), vec!["level".to_string()]);
        graph.register("combat_rating".to_string(), vec!["max_health".to_string(), "attack".to_string()]);
        
        // Cache some values
        let instance = InstanceId::new();
        graph.set_cached(instance, "max_health".to_string(), AttributeValue::Integer(100));
        graph.set_cached(instance, "combat_rating".to_string(), AttributeValue::Integer(50));
        
        // Invalidate level -> should invalidate max_health and combat_rating
        graph.invalidate_dependents(&"level".to_string());
        
        assert!(graph.get_cached(instance, &"max_health".to_string()).is_none());
        assert!(graph.get_cached(instance, &"combat_rating".to_string()).is_none());
    }
}