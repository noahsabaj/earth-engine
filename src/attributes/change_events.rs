/// Attribute Change Event System
/// 
/// Tracks and dispatches events when attributes change.
/// Supports multiple listeners and event filtering.

use crate::instance::InstanceId;
use crate::attributes::{AttributeKey, AttributeValue};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

/// Event types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// Attribute value changed
    Changed = 0,
    /// Attribute added
    Added = 1,
    /// Attribute removed
    Removed = 2,
    /// Modifier added
    ModifierAdded = 3,
    /// Modifier removed
    ModifierRemoved = 4,
    /// Inheritance changed
    InheritanceChanged = 5,
    /// Computed value invalidated
    Invalidated = 6,
}

/// Attribute change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeEvent {
    /// Affected instance
    pub instance: InstanceId,
    
    /// Affected attribute
    pub key: AttributeKey,
    
    /// Event type
    pub event_type: EventType,
    
    /// Old value (if applicable)
    pub old_value: Option<AttributeValue>,
    
    /// New value (if applicable)
    pub new_value: Option<AttributeValue>,
    
    /// Event timestamp
    #[serde(skip, default = "std::time::Instant::now")]
    pub timestamp: std::time::Instant,
}

impl AttributeEvent {
    pub fn new(
        instance: InstanceId,
        key: AttributeKey,
        event_type: EventType,
    ) -> Self {
        Self {
            instance,
            key,
            event_type,
            old_value: None,
            new_value: None,
            timestamp: std::time::Instant::now(),
        }
    }
    
    pub fn with_values(
        mut self,
        old_value: Option<AttributeValue>,
        new_value: Option<AttributeValue>,
    ) -> Self {
        self.old_value = old_value;
        self.new_value = new_value;
        self
    }
}

/// Event listener trait
pub trait ChangeListener: Send + Sync {
    /// Called when event occurs
    fn on_event(&self, event: &AttributeEvent);
    
    /// Filter for events (return true to receive)
    fn filter(&self, event: &AttributeEvent) -> bool {
        true
    }
    
    /// Priority (higher = called first)
    fn priority(&self) -> i32 {
        0
    }
}

/// Event dispatcher
pub struct EventDispatcher {
    /// Registered listeners
    listeners: Vec<ListenerEntry>,
    
    /// Event queue for async processing
    event_queue: Arc<Mutex<VecDeque<AttributeEvent>>>,
    
    /// Event history
    history: EventHistory,
    
    /// Dispatching enabled
    enabled: bool,
}

struct ListenerEntry {
    listener: Box<dyn ChangeListener>,
    id: u64,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            history: EventHistory::new(1000),
            enabled: true,
        }
    }
    
    /// Register a listener
    pub fn register(&mut self, listener: Box<dyn ChangeListener>) -> u64 {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        self.listeners.push(ListenerEntry { listener, id });
        
        // Sort by priority
        self.listeners.sort_by_key(|e| -e.listener.priority());
        
        id
    }
    
    /// Unregister a listener
    pub fn unregister(&mut self, id: u64) -> bool {
        let len_before = self.listeners.len();
        self.listeners.retain(|e| e.id != id);
        self.listeners.len() != len_before
    }
    
    /// Dispatch an event
    pub fn dispatch(&mut self, event: AttributeEvent) {
        if !self.enabled {
            return;
        }
        
        // Add to history
        self.history.add(event.clone());
        
        // Dispatch to listeners
        for entry in &self.listeners {
            if entry.listener.filter(&event) {
                entry.listener.on_event(&event);
            }
        }
        
        // Add to async queue
        if let Ok(mut queue) = self.event_queue.lock() {
            queue.push_back(event);
        }
    }
    
    /// Process queued events
    pub fn process_queue(&self) -> Vec<AttributeEvent> {
        if let Ok(mut queue) = self.event_queue.lock() {
            queue.drain(..).collect()
        } else {
            Vec::new()
        }
    }
    
    /// Enable/disable dispatching
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Get event history
    pub fn history(&self) -> &EventHistory {
        &self.history
    }
    
    /// Clear all listeners
    pub fn clear_listeners(&mut self) {
        self.listeners.clear();
    }
}

/// Event history tracker
pub struct EventHistory {
    /// Ring buffer of events
    events: VecDeque<AttributeEvent>,
    
    /// Maximum history size
    max_size: usize,
    
    /// Statistics
    stats: EventStats,
}

/// Event statistics
#[derive(Default)]
pub struct EventStats {
    /// Total events by type
    pub by_type: HashMap<EventType, u64>,
    
    /// Events per attribute
    pub by_attribute: HashMap<AttributeKey, u64>,
    
    /// Events per instance
    pub by_instance: HashMap<InstanceId, u64>,
    
    /// Total events
    pub total: u64,
}

impl EventHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(max_size),
            max_size,
            stats: EventStats::default(),
        }
    }
    
    /// Add event to history
    pub fn add(&mut self, event: AttributeEvent) {
        // Update stats
        *self.stats.by_type.entry(event.event_type).or_insert(0) += 1;
        *self.stats.by_attribute.entry(event.key.clone()).or_insert(0) += 1;
        *self.stats.by_instance.entry(event.instance).or_insert(0) += 1;
        self.stats.total += 1;
        
        // Add to history
        if self.events.len() >= self.max_size {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
    
    /// Get recent events
    pub fn recent(&self, count: usize) -> Vec<&AttributeEvent> {
        self.events.iter()
            .rev()
            .take(count)
            .collect()
    }
    
    /// Find events by instance
    pub fn by_instance(&self, instance: InstanceId) -> Vec<&AttributeEvent> {
        self.events.iter()
            .filter(|e| e.instance == instance)
            .collect()
    }
    
    /// Find events by attribute
    pub fn by_attribute(&self, key: &AttributeKey) -> Vec<&AttributeEvent> {
        self.events.iter()
            .filter(|e| e.key == *key)
            .collect()
    }
    
    /// Get statistics
    pub fn stats(&self) -> &EventStats {
        &self.stats
    }
    
    /// Clear history
    pub fn clear(&mut self) {
        self.events.clear();
        self.stats = EventStats::default();
    }
}

/// Common listener implementations

/// Console logger listener
pub struct ConsoleLogger {
    pub prefix: String,
}

impl ChangeListener for ConsoleLogger {
    fn on_event(&self, event: &AttributeEvent) {
        println!(
            "{} [{:?}] Instance {:?}: {} {:?}",
            self.prefix,
            event.event_type,
            event.instance,
            event.key,
            event.new_value
        );
    }
}

/// Attribute watcher
pub struct AttributeWatcher {
    pub watched_keys: Vec<AttributeKey>,
    pub callback: Arc<dyn Fn(&AttributeEvent) + Send + Sync>,
}

impl ChangeListener for AttributeWatcher {
    fn on_event(&self, event: &AttributeEvent) {
        if self.watched_keys.contains(&event.key) {
            (self.callback)(event);
        }
    }
    
    fn filter(&self, event: &AttributeEvent) -> bool {
        self.watched_keys.contains(&event.key)
    }
}

/// Event aggregator
pub struct EventAggregator {
    pub events: Arc<Mutex<Vec<AttributeEvent>>>,
    pub max_events: usize,
}

impl EventAggregator {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            max_events,
        }
    }
    
    pub fn get_events(&self) -> Vec<AttributeEvent> {
        if let Ok(events) = self.events.lock() {
            events.clone()
        } else {
            Vec::new()
        }
    }
    
    pub fn clear(&self) {
        if let Ok(mut events) = self.events.lock() {
            events.clear();
        }
    }
}

impl ChangeListener for EventAggregator {
    fn on_event(&self, event: &AttributeEvent) {
        if let Ok(mut events) = self.events.lock() {
            if events.len() >= self.max_events {
                events.remove(0);
            }
            events.push(event.clone());
        }
    }
}

/// Event filter builder
pub struct EventFilterBuilder {
    instance_filter: Option<InstanceId>,
    attribute_filter: Option<Vec<AttributeKey>>,
    type_filter: Option<Vec<EventType>>,
}

impl EventFilterBuilder {
    pub fn new() -> Self {
        Self {
            instance_filter: None,
            attribute_filter: None,
            type_filter: None,
        }
    }
    
    pub fn instance(mut self, instance: InstanceId) -> Self {
        self.instance_filter = Some(instance);
        self
    }
    
    pub fn attributes(mut self, keys: Vec<AttributeKey>) -> Self {
        self.attribute_filter = Some(keys);
        self
    }
    
    pub fn types(mut self, types: Vec<EventType>) -> Self {
        self.type_filter = Some(types);
        self
    }
    
    pub fn build(self) -> Box<dyn ChangeListener> {
        Box::new(FilteredListener {
            instance_filter: self.instance_filter,
            attribute_filter: self.attribute_filter,
            type_filter: self.type_filter,
            callback: Arc::new(|_| {}),
        })
    }
}

struct FilteredListener {
    instance_filter: Option<InstanceId>,
    attribute_filter: Option<Vec<AttributeKey>>,
    type_filter: Option<Vec<EventType>>,
    callback: Arc<dyn Fn(&AttributeEvent) + Send + Sync>,
}

impl ChangeListener for FilteredListener {
    fn on_event(&self, event: &AttributeEvent) {
        (self.callback)(event);
    }
    
    fn filter(&self, event: &AttributeEvent) -> bool {
        if let Some(instance) = self.instance_filter {
            if event.instance != instance {
                return false;
            }
        }
        
        if let Some(ref attributes) = self.attribute_filter {
            if !attributes.contains(&event.key) {
                return false;
            }
        }
        
        if let Some(ref types) = self.type_filter {
            if !types.contains(&event.event_type) {
                return false;
            }
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_dispatcher() {
        let mut dispatcher = EventDispatcher::new();
        
        // Add aggregator listener
        let aggregator = Arc::new(EventAggregator::new(10));
        dispatcher.register(Box::new(aggregator.clone()));
        
        // Dispatch event
        let event = AttributeEvent::new(
            InstanceId::new(),
            "health".to_string(),
            EventType::Changed,
        );
        
        dispatcher.dispatch(event);
        
        // Check aggregator received event
        let events = aggregator.get_events();
        assert_eq!(events.len(), 1);
    }
    
    #[test]
    fn test_event_history() {
        let mut history = EventHistory::new(5);
        
        // Add events
        for i in 0..10 {
            let event = AttributeEvent::new(
                InstanceId::new(),
                format!("attr{}", i),
                EventType::Changed,
            );
            history.add(event);
        }
        
        // Should only keep last 5
        assert_eq!(history.events.len(), 5);
        assert_eq!(history.stats().total, 10);
    }
}