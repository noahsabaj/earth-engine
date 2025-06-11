/// Attribute Inheritance System
/// 
/// Handles inheritance of attributes from parent entities, templates, or classes.
/// Supports multiple inheritance chains and conflict resolution.

use crate::instance::InstanceId;
use crate::attributes::{AttributeKey, AttributeValue, AttributeManager};
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

/// Source of inherited attribute
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeSource {
    /// From parent instance
    Parent(InstanceId),
    /// From template
    Template(String),
    /// From class/type
    Class(String),
    /// From tag
    Tag(String),
    /// Default value
    Default,
}

/// Inheritance rule for attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritanceRule {
    /// Attribute to inherit
    pub attribute: AttributeKey,
    /// Source priority (higher wins)
    pub priority: u8,
    /// Merge strategy
    pub strategy: MergeStrategy,
    /// Optional condition
    pub condition: Option<InheritanceCondition>,
}

/// How to merge inherited values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Use first value found
    First,
    /// Use last value found
    Last,
    /// Add all values
    Sum,
    /// Multiply all values
    Product,
    /// Take minimum
    Min,
    /// Take maximum
    Max,
    /// Average all values
    Average,
    /// Override (no inheritance)
    Override,
}

/// Conditions for inheritance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InheritanceCondition {
    /// Only inherit if attribute not set
    NotSet,
    /// Only inherit if value matches
    ValueEquals(AttributeValue),
    /// Only inherit if source has attribute
    SourceHasAttribute(AttributeKey),
    /// Custom condition
    Custom(String),
}

/// Inheritance chain for an instance
#[derive(Debug, Clone)]
pub struct InheritanceChain {
    /// Instance ID
    pub instance: InstanceId,
    /// Sources in priority order
    pub sources: Vec<AttributeSource>,
    /// Cached inherited values
    pub cache: HashMap<AttributeKey, AttributeValue>,
    /// Cache version
    pub cache_version: u32,
}

impl InheritanceChain {
    pub fn new(instance: InstanceId) -> Self {
        Self {
            instance,
            sources: Vec::new(),
            cache: HashMap::new(),
            cache_version: 0,
        }
    }
    
    /// Add inheritance source
    pub fn add_source(&mut self, source: AttributeSource) {
        if !self.sources.contains(&source) {
            self.sources.push(source);
            self.invalidate_cache();
        }
    }
    
    /// Remove inheritance source
    pub fn remove_source(&mut self, source: &AttributeSource) -> bool {
        let len_before = self.sources.len();
        self.sources.retain(|s| s != source);
        
        if self.sources.len() != len_before {
            self.invalidate_cache();
            true
        } else {
            false
        }
    }
    
    /// Clear all sources
    pub fn clear_sources(&mut self) {
        self.sources.clear();
        self.invalidate_cache();
    }
    
    /// Invalidate cache
    pub fn invalidate_cache(&mut self) {
        self.cache.clear();
        self.cache_version += 1;
    }
    
    /// Check if has source
    pub fn has_source(&self, source: &AttributeSource) -> bool {
        self.sources.contains(source)
    }
}

/// Inheritance resolver
pub struct InheritanceResolver {
    /// Inheritance chains per instance
    chains: HashMap<InstanceId, InheritanceChain>,
    
    /// Global inheritance rules
    rules: HashMap<AttributeKey, Vec<InheritanceRule>>,
    
    /// Template definitions
    templates: HashMap<String, HashMap<AttributeKey, AttributeValue>>,
    
    /// Class definitions
    classes: HashMap<String, HashMap<AttributeKey, AttributeValue>>,
    
    /// Tag attributes
    tags: HashMap<String, HashMap<AttributeKey, AttributeValue>>,
}

impl InheritanceResolver {
    pub fn new() -> Self {
        Self {
            chains: HashMap::new(),
            rules: HashMap::new(),
            templates: HashMap::new(),
            classes: HashMap::new(),
            tags: HashMap::new(),
        }
    }
    
    /// Set inheritance chain for instance
    pub fn set_chain(&mut self, instance: InstanceId, chain: InheritanceChain) {
        self.chains.insert(instance, chain);
    }
    
    /// Get or create chain
    pub fn get_or_create_chain(&mut self, instance: InstanceId) -> &mut InheritanceChain {
        self.chains.entry(instance)
            .or_insert_with(|| InheritanceChain::new(instance))
    }
    
    /// Add inheritance rule
    pub fn add_rule(&mut self, rule: InheritanceRule) {
        self.rules
            .entry(rule.attribute.clone())
            .or_insert_with(Vec::new)
            .push(rule);
    }
    
    /// Register template
    pub fn register_template(
        &mut self,
        name: String,
        attributes: HashMap<AttributeKey, AttributeValue>,
    ) {
        self.templates.insert(name, attributes);
        self.invalidate_all_caches();
    }
    
    /// Register class
    pub fn register_class(
        &mut self,
        name: String,
        attributes: HashMap<AttributeKey, AttributeValue>,
    ) {
        self.classes.insert(name, attributes);
        self.invalidate_all_caches();
    }
    
    /// Register tag attributes
    pub fn register_tag(
        &mut self,
        tag: String,
        attributes: HashMap<AttributeKey, AttributeValue>,
    ) {
        self.tags.insert(tag, attributes);
        self.invalidate_all_caches();
    }
    
    /// Resolve inherited value
    pub fn resolve(
        &self,
        instance: InstanceId,
        key: &AttributeKey,
        manager: &AttributeManager,
    ) -> Option<AttributeValue> {
        let chain = self.chains.get(&instance)?;
        
        // Check cache first
        if let Some(cached) = chain.cache.get(key) {
            return Some(cached.clone());
        }
        
        // Get rules for this attribute
        let rules = self.rules.get(key)
            .map(|r| r.as_slice())
            .unwrap_or(&[]);
            
        // Default strategy if no rules
        let default_strategy = if rules.is_empty() {
            MergeStrategy::First
        } else {
            rules[0].strategy
        };
        
        // Collect values from sources
        let mut values = Vec::new();
        
        for source in &chain.sources {
            if let Some(value) = self.get_from_source(source, key, manager) {
                // Check conditions
                let mut allowed = true;
                for rule in rules {
                    if rule.attribute == *key {
                        if let Some(ref condition) = rule.condition {
                            allowed = self.check_condition(condition, instance, source, key, manager);
                        }
                    }
                }
                
                if allowed {
                    values.push(value);
                }
            }
        }
        
        // Apply merge strategy
        self.merge_values(values, default_strategy)
    }
    
    /// Get value from source
    fn get_from_source(
        &self,
        source: &AttributeSource,
        key: &AttributeKey,
        manager: &AttributeManager,
    ) -> Option<AttributeValue> {
        match source {
            AttributeSource::Parent(parent_id) => {
                manager.storage.get(*parent_id, key).cloned()
            }
            
            AttributeSource::Template(name) => {
                self.templates.get(name)?.get(key).cloned()
            }
            
            AttributeSource::Class(name) => {
                self.classes.get(name)?.get(key).cloned()
            }
            
            AttributeSource::Tag(tag) => {
                self.tags.get(tag)?.get(key).cloned()
            }
            
            AttributeSource::Default => {
                manager.metadata.defaults.get(key).cloned()
            }
        }
    }
    
    /// Check inheritance condition
    fn check_condition(
        &self,
        condition: &InheritanceCondition,
        instance: InstanceId,
        source: &AttributeSource,
        key: &AttributeKey,
        manager: &AttributeManager,
    ) -> bool {
        match condition {
            InheritanceCondition::NotSet => {
                manager.storage.get(instance, key).is_none()
            }
            
            InheritanceCondition::ValueEquals(value) => {
                manager.storage.get(instance, key)
                    .map(|v| v == value)
                    .unwrap_or(false)
            }
            
            InheritanceCondition::SourceHasAttribute(attr) => {
                self.get_from_source(source, attr, manager).is_some()
            }
            
            InheritanceCondition::Custom(_) => {
                // Would evaluate custom condition
                true
            }
        }
    }
    
    /// Merge multiple values
    fn merge_values(&self, values: Vec<AttributeValue>, strategy: MergeStrategy) -> Option<AttributeValue> {
        if values.is_empty() {
            return None;
        }
        
        match strategy {
            MergeStrategy::First => values.into_iter().next(),
            
            MergeStrategy::Last => values.into_iter().last(),
            
            MergeStrategy::Sum => {
                values.into_iter().reduce(|a, b| a.add(&b).unwrap_or(a))
            }
            
            MergeStrategy::Product => {
                if let Some(first) = values.first() {
                    if let Some(mut product) = first.as_float() {
                        for value in values.iter().skip(1) {
                            if let Some(v) = value.as_float() {
                                product *= v;
                            }
                        }
                        return Some(AttributeValue::Float(product));
                    }
                }
                None
            }
            
            MergeStrategy::Min => {
                values.into_iter().min_by(|a, b| a.compare(b).unwrap_or(std::cmp::Ordering::Equal))
            }
            
            MergeStrategy::Max => {
                values.into_iter().max_by(|a, b| a.compare(b).unwrap_or(std::cmp::Ordering::Equal))
            }
            
            MergeStrategy::Average => {
                if values.is_empty() {
                    return None;
                }
                
                let sum: f64 = values.iter()
                    .filter_map(|v| v.as_float())
                    .sum();
                    
                Some(AttributeValue::Float(sum / values.len() as f64))
            }
            
            MergeStrategy::Override => None,
        }
    }
    
    /// Invalidate all caches
    fn invalidate_all_caches(&mut self) {
        for chain in self.chains.values_mut() {
            chain.invalidate_cache();
        }
    }
    
    /// Get inheritance graph
    pub fn get_inheritance_graph(&self, instance: InstanceId) -> InheritanceGraph {
        let mut graph = InheritanceGraph::new();
        let mut visited = HashSet::new();
        
        self.build_graph_recursive(instance, &mut graph, &mut visited);
        
        graph
    }
    
    fn build_graph_recursive(
        &self,
        instance: InstanceId,
        graph: &mut InheritanceGraph,
        visited: &mut HashSet<InstanceId>,
    ) {
        if !visited.insert(instance) {
            return; // Already visited
        }
        
        if let Some(chain) = self.chains.get(&instance) {
            for source in &chain.sources {
                graph.add_edge(instance, source.clone());
                
                // Recurse for parent instances
                if let AttributeSource::Parent(parent) = source {
                    self.build_graph_recursive(*parent, graph, visited);
                }
            }
        }
    }
}

/// Inheritance graph for visualization
pub struct InheritanceGraph {
    /// Nodes (instances)
    pub nodes: HashSet<InstanceId>,
    /// Edges (instance -> source)
    pub edges: Vec<(InstanceId, AttributeSource)>,
}

impl InheritanceGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: Vec::new(),
        }
    }
    
    pub fn add_edge(&mut self, from: InstanceId, to: AttributeSource) {
        self.nodes.insert(from);
        self.edges.push((from, to));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_inheritance_chain() {
        let mut chain = InheritanceChain::new(InstanceId::new());
        
        chain.add_source(AttributeSource::Template("warrior".to_string()));
        chain.add_source(AttributeSource::Class("humanoid".to_string()));
        
        assert_eq!(chain.sources.len(), 2);
        assert!(chain.has_source(&AttributeSource::Template("warrior".to_string())));
    }
    
    #[test]
    fn test_merge_strategies() {
        let resolver = InheritanceResolver::new();
        
        let values = vec![
            AttributeValue::Float(10.0),
            AttributeValue::Float(20.0),
            AttributeValue::Float(30.0),
        ];
        
        assert_eq!(
            resolver.merge_values(values.clone(), MergeStrategy::First),
            Some(AttributeValue::Float(10.0))
        );
        
        assert_eq!(
            resolver.merge_values(values.clone(), MergeStrategy::Sum),
            Some(AttributeValue::Float(60.0))
        );
        
        assert_eq!(
            resolver.merge_values(values.clone(), MergeStrategy::Average),
            Some(AttributeValue::Float(20.0))
        );
    }
}