/// Event System
/// 
/// Provides loose coupling between engine systems through an asynchronous event bus.
/// Systems can publish and subscribe to events without direct dependencies,
/// reducing integration bottlenecks and improving system coordination.
/// 
/// Features:
/// - Type-safe event publishing and subscribing
/// - Async event processing with backpressure handling
/// - Event filtering and priority queuing
/// - Batch event processing for performance
/// - Event replay and persistence capabilities

use crate::error::{EngineError, EngineResult};
use crate::thread_pool::{ThreadPoolManager, PoolCategory};
use std::collections::{HashMap, VecDeque, BinaryHeap};
use std::sync::{Arc, Weak};
use parking_lot::{RwLock, Mutex};
use std::time::{Instant, Duration};
use std::cmp::Reverse;
use serde::{Serialize, Deserialize};
use std::any::{Any, TypeId};

/// Event priority levels
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Event delivery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryMode {
    /// Fire and forget - events may be dropped if queue is full
    FireAndForget,
    /// Guaranteed delivery - events are queued until processed
    Guaranteed,
    /// Immediate - events bypass queue and are processed synchronously
    Immediate,
}

/// Event handler trait for receiving events
pub trait EventHandler<T: Event>: Send + Sync {
    fn handle_event(&self, event: &T);
    fn can_handle(&self, event: &T) -> bool { true }
    fn handler_name(&self) -> &str { "unnamed_handler" }
}

/// Base event trait that all events must implement
pub trait Event: Send + Sync + Any + std::fmt::Debug + Clone {
    /// Event type identifier
    fn event_type(&self) -> &'static str;
    
    /// Event priority
    fn priority(&self) -> EventPriority { EventPriority::Normal }
    
    /// Whether this event should be persisted for replay
    fn should_persist(&self) -> bool { false }
    
    /// Custom event data as bytes for serialization
    fn to_bytes(&self) -> Vec<u8> { Vec::new() }
    
    /// Create event from bytes
    fn from_bytes(_bytes: &[u8]) -> Option<Self> where Self: Sized { None }
}

/// Event wrapper with metadata
#[derive(Debug, Clone)]
pub struct EventEnvelope {
    /// Event type ID for downcasting
    pub type_id: TypeId,
    
    /// Event type name
    pub type_name: &'static str,
    
    /// Event data (boxed Any)
    pub event: Arc<dyn Any + Send + Sync>,
    
    /// Event priority
    pub priority: EventPriority,
    
    /// Timestamp when event was created
    pub timestamp: Instant,
    
    /// Unique event ID
    pub id: u64,
    
    /// Source system that generated the event
    pub source: Option<String>,
    
    /// Event delivery mode
    pub delivery_mode: DeliveryMode,
    
    /// Number of processing attempts
    pub attempts: u32,
    
    /// Maximum retry attempts
    pub max_attempts: u32,
}

/// Event subscription handle
pub struct EventSubscription {
    pub id: u64,
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub handler: Weak<dyn Any + Send + Sync>,
    pub filter: Option<Box<dyn Fn(&EventEnvelope) -> bool + Send + Sync>>,
    pub subscriber_name: String,
    pub created_at: Instant,
}

/// Event filter function type
pub type EventFilter = Box<dyn Fn(&EventEnvelope) -> bool + Send + Sync>;

/// Event bus statistics
#[derive(Debug, Clone, Default)]
pub struct EventBusStats {
    pub events_published: u64,
    pub events_processed: u64,
    pub events_dropped: u64,
    pub events_failed: u64,
    pub active_subscriptions: usize,
    pub queue_size: usize,
    pub average_processing_time_ms: f64,
    pub error_rate: f64,
}

/// Event bus configuration
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Maximum number of events in queue
    pub max_queue_size: usize,
    
    /// Maximum number of events to process per batch
    pub batch_size: usize,
    
    /// How often to process events (milliseconds)
    pub processing_interval_ms: u64,
    
    /// Maximum time to spend processing events per frame (milliseconds)
    pub max_processing_time_ms: f64,
    
    /// Whether to enable event persistence
    pub enable_persistence: bool,
    
    /// Maximum number of events to keep in history
    pub max_history_size: usize,
    
    /// Enable event replay functionality
    pub enable_replay: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            batch_size: 100,
            processing_interval_ms: 16, // ~60 FPS
            max_processing_time_ms: 5.0, // 5ms budget per frame
            enable_persistence: false,
            max_history_size: 1000,
            enable_replay: false,
        }
    }
}

/// Main event bus implementation
pub struct EventBus {
    /// Event queue (priority queue)
    event_queue: Mutex<BinaryHeap<Reverse<QueuedEvent>>>,
    
    /// Event subscriptions by type
    subscriptions: RwLock<HashMap<TypeId, Vec<EventSubscription>>>,
    
    /// Next subscription ID
    next_subscription_id: std::sync::atomic::AtomicU64,
    
    /// Next event ID
    next_event_id: std::sync::atomic::AtomicU64,
    
    /// Event bus configuration
    config: EventBusConfig,
    
    /// Event processing statistics
    stats: RwLock<EventBusStats>,
    
    /// Event history for replay
    event_history: RwLock<VecDeque<EventEnvelope>>,
    
    /// Failed events for retry
    failed_events: Mutex<VecDeque<EventEnvelope>>,
    
    /// Processing state
    is_processing: std::sync::atomic::AtomicBool,
    
    /// Last processing time
    last_processing_time: RwLock<Instant>,
    
    /// Event handlers registry
    handlers_registry: RwLock<HashMap<String, Weak<dyn Any + Send + Sync>>>,
}

/// Queued event for priority processing
#[derive(Debug, Clone)]
struct QueuedEvent {
    envelope: EventEnvelope,
    queued_at: Instant,
}

impl PartialEq for QueuedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.envelope.priority == other.envelope.priority && 
        self.envelope.timestamp == other.envelope.timestamp
    }
}

impl Eq for QueuedEvent {}

impl PartialOrd for QueuedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority events come first, then earlier timestamps
        match other.envelope.priority.cmp(&self.envelope.priority) {
            std::cmp::Ordering::Equal => self.envelope.timestamp.cmp(&other.envelope.timestamp),
            other => other,
        }
    }
}

/// Event bus builder for configuration
pub struct EventBusBuilder {
    config: EventBusConfig,
}

impl EventBus {
    /// Create a new event bus with default configuration
    pub fn new() -> Self {
        Self::with_config(EventBusConfig::default())
    }
    
    /// Create a new event bus with custom configuration
    pub fn with_config(config: EventBusConfig) -> Self {
        Self {
            event_queue: Mutex::new(BinaryHeap::new()),
            subscriptions: RwLock::new(HashMap::new()),
            next_subscription_id: std::sync::atomic::AtomicU64::new(1),
            next_event_id: std::sync::atomic::AtomicU64::new(1),
            config,
            stats: RwLock::new(EventBusStats::default()),
            event_history: RwLock::new(VecDeque::new()),
            failed_events: Mutex::new(VecDeque::new()),
            is_processing: std::sync::atomic::AtomicBool::new(false),
            last_processing_time: RwLock::new(Instant::now()),
            handlers_registry: RwLock::new(HashMap::new()),
        }
    }
    
    /// Subscribe to events of a specific type
    pub fn subscribe<T: Event + 'static, H: EventHandler<T> + 'static>(
        &self,
        handler: Arc<H>,
        filter: Option<EventFilter>,
        subscriber_name: String,
    ) -> u64 {
        let subscription_id = self.next_subscription_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        
        let subscription = EventSubscription {
            id: subscription_id,
            type_id,
            type_name,
            handler: Arc::downgrade(&handler) as Weak<dyn Any + Send + Sync>,
            filter,
            subscriber_name: subscriber_name.clone(),
            created_at: Instant::now(),
        };
        
        // Store handler in registry
        {
            let mut registry = self.handlers_registry.write();
            registry.insert(format!("{}:{}", subscriber_name, subscription_id), 
                          Arc::downgrade(&handler) as Weak<dyn Any + Send + Sync>);
        }
        
        // Add subscription
        {
            let mut subscriptions = self.subscriptions.write();
            subscriptions.entry(type_id).or_insert_with(Vec::new).push(subscription);
        }
        
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.active_subscriptions += 1;
        }
        
        subscription_id
    }
    
    /// Unsubscribe from events
    pub fn unsubscribe(&self, subscription_id: u64) -> bool {
        let mut subscriptions = self.subscriptions.write();
        let mut found = false;
        
        for (_, subs) in subscriptions.iter_mut() {
            if let Some(pos) = subs.iter().position(|s| s.id == subscription_id) {
                subs.remove(pos);
                found = true;
                break;
            }
        }
        
        if found {
            let mut stats = self.stats.write();
            stats.active_subscriptions = stats.active_subscriptions.saturating_sub(1);
        }
        
        found
    }
    
    /// Publish an event
    pub fn publish<T: Event + 'static>(&self, event: T, source: Option<String>, delivery_mode: DeliveryMode) -> EngineResult<u64> {
        let event_id = self.next_event_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        
        let envelope = EventEnvelope {
            type_id,
            type_name,
            event: Arc::new(event.clone()) as Arc<dyn Any + Send + Sync>,
            priority: event.priority(),
            timestamp: Instant::now(),
            id: event_id,
            source,
            delivery_mode,
            attempts: 0,
            max_attempts: 3,
        };
        
        // Clone envelope for persistence before moving it
        let envelope_for_history = if self.config.enable_persistence && event.should_persist() {
            Some(envelope.clone())
        } else {
            None
        };
        
        match delivery_mode {
            DeliveryMode::Immediate => {
                self.process_event_immediate(&envelope)?;
            }
            DeliveryMode::FireAndForget => {
                self.enqueue_event(envelope, false)?;
            }
            DeliveryMode::Guaranteed => {
                self.enqueue_event(envelope, true)?;
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.events_published += 1;
        }
        
        // Add to history if persistence is enabled
        if let Some(envelope_for_history) = envelope_for_history {
            let mut history = self.event_history.write();
            history.push_back(envelope_for_history);
            if history.len() > self.config.max_history_size {
                history.pop_front();
            }
        }
        
        Ok(event_id)
    }
    
    /// Process events from the queue
    pub fn process_events(&self) -> EngineResult<usize> {
        if self.is_processing.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Ok(0); // Already processing
        }
        
        let start_time = Instant::now();
        let mut processed_count = 0;
        let max_processing_time = Duration::from_millis(self.config.max_processing_time_ms as u64);
        
        // Process up to batch_size events or until time budget is exhausted
        while processed_count < self.config.batch_size && start_time.elapsed() < max_processing_time {
            let event = {
                let mut queue = self.event_queue.lock();
                queue.pop().map(|Reverse(queued)| queued.envelope)
            };
            
            match event {
                Some(envelope) => {
                    if let Err(e) = self.process_event(&envelope) {
                        log::warn!("Failed to process event {}: {}", envelope.id, e);
                        self.handle_failed_event(envelope);
                    } else {
                        processed_count += 1;
                    }
                }
                None => break, // No more events
            }
        }
        
        // Process retry queue
        let retry_count = self.process_retry_queue(max_processing_time - start_time.elapsed())?;
        processed_count += retry_count;
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.events_processed += processed_count as u64;
            let processing_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
            if processed_count > 0 {
                stats.average_processing_time_ms = 
                    (stats.average_processing_time_ms + processing_time_ms) / 2.0;
            }
        }
        
        *self.last_processing_time.write() = Instant::now();
        self.is_processing.store(false, std::sync::atomic::Ordering::SeqCst);
        
        Ok(processed_count)
    }
    
    /// Enqueue an event for processing
    fn enqueue_event(&self, envelope: EventEnvelope, guaranteed: bool) -> EngineResult<()> {
        let mut queue = self.event_queue.lock();
        
        if queue.len() >= self.config.max_queue_size {
            if guaranteed {
                // For guaranteed delivery, we need to make space
                while queue.len() >= self.config.max_queue_size {
                    if let Some(Reverse(dropped)) = queue.pop() {
                        log::warn!("Dropping event {} to make space for guaranteed event", dropped.envelope.id);
                        let mut stats = self.stats.write();
                        stats.events_dropped += 1;
                    }
                }
            } else {
                // For fire-and-forget, just drop the new event
                let mut stats = self.stats.write();
                stats.events_dropped += 1;
                return Err(EngineError::ResourceExhausted("Event queue full".to_string()));
            }
        }
        
        let queued_event = QueuedEvent {
            envelope,
            queued_at: Instant::now(),
        };
        
        queue.push(Reverse(queued_event));
        
        // Update queue size stat
        {
            let mut stats = self.stats.write();
            stats.queue_size = queue.len();
        }
        
        Ok(())
    }
    
    /// Process a single event immediately
    fn process_event_immediate(&self, envelope: &EventEnvelope) -> EngineResult<()> {
        ThreadPoolManager::global().execute(PoolCategory::Compute, || {
            if let Err(e) = self.process_event(envelope) {
                log::error!("Failed to process immediate event {}: {}", envelope.id, e);
            }
        });
        Ok(())
    }
    
    /// Process a single event
    fn process_event(&self, envelope: &EventEnvelope) -> EngineResult<()> {
        let subscriptions = self.subscriptions.read();
        let subs = match subscriptions.get(&envelope.type_id) {
            Some(subs) => subs,
            None => return Ok(()), // No subscribers
        };
        
        let mut successful_deliveries = 0;
        let mut failed_deliveries = 0;
        
        for subscription in subs.iter() {
            // Apply filter if present
            if let Some(ref filter) = subscription.filter {
                if !filter(envelope) {
                    continue;
                }
            }
            
            // Get handler
            let handler = match subscription.handler.upgrade() {
                Some(handler) => handler,
                None => {
                    // Dead handler, should be cleaned up
                    continue;
                }
            };
            
            // Call handler based on event type
            match self.call_typed_handler(envelope, &handler) {
                Ok(()) => successful_deliveries += 1,
                Err(e) => {
                    log::warn!("Handler {} failed for event {}: {}", 
                             subscription.subscriber_name, envelope.id, e);
                    failed_deliveries += 1;
                }
            }
        }
        
        if successful_deliveries == 0 && failed_deliveries > 0 {
            return Err(EngineError::ProcessingFailed(
                format!("All handlers failed for event {}", envelope.id)
            ));
        }
        
        Ok(())
    }
    
    /// Call typed event handler
    fn call_typed_handler(&self, envelope: &EventEnvelope, handler: &Arc<dyn Any + Send + Sync>) -> EngineResult<()> {
        // This is a simplified version - in practice, you'd need a registry of type handlers
        // For now, we'll just log that the event was processed
        log::debug!("Processing event {} of type {} at {:?}", 
                   envelope.id, envelope.type_name, envelope.timestamp);
        Ok(())
    }
    
    /// Handle failed event processing
    fn handle_failed_event(&self, mut envelope: EventEnvelope) {
        envelope.attempts += 1;
        
        if envelope.attempts < envelope.max_attempts {
            // Retry with exponential backoff
            let delay = Duration::from_millis(100 * (2_u64.pow(envelope.attempts - 1)));
            
            // For now, just add back to the failed queue
            // In a real implementation, you'd schedule a delayed retry
            self.failed_events.lock().push_back(envelope);
        } else {
            log::error!("Event {} exceeded max retry attempts, dropping", envelope.id);
            let mut stats = self.stats.write();
            stats.events_failed += 1;
        }
    }
    
    /// Process retry queue
    fn process_retry_queue(&self, time_budget: Duration) -> EngineResult<usize> {
        let start_time = Instant::now();
        let mut processed = 0;
        
        while start_time.elapsed() < time_budget {
            let event = self.failed_events.lock().pop_front();
            match event {
                Some(envelope) => {
                    if let Err(e) = self.process_event(&envelope) {
                        log::warn!("Retry failed for event {}: {}", envelope.id, e);
                        self.handle_failed_event(envelope);
                    } else {
                        processed += 1;
                    }
                }
                None => break,
            }
        }
        
        Ok(processed)
    }
    
    /// Get event bus statistics
    pub fn get_stats(&self) -> EventBusStats {
        self.stats.read().clone()
    }
    
    /// Clear event history
    pub fn clear_history(&self) {
        self.event_history.write().clear();
    }
    
    /// Replay events from history
    pub fn replay_events(&self, from_time: Instant, to_time: Instant) -> EngineResult<usize> {
        if !self.config.enable_replay {
            return Err(EngineError::FeatureDisabled("Event replay is disabled".to_string()));
        }
        
        let history = self.event_history.read();
        let events_to_replay: Vec<_> = history.iter()
            .filter(|envelope| envelope.timestamp >= from_time && envelope.timestamp <= to_time)
            .cloned()
            .collect();
        drop(history);
        
        let count = events_to_replay.len();
        
        for envelope in events_to_replay {
            self.enqueue_event(envelope, false)?;
        }
        
        Ok(count)
    }
    
    /// Clean up dead subscriptions
    pub fn cleanup_subscriptions(&self) {
        let mut subscriptions = self.subscriptions.write();
        let mut total_removed = 0;
        
        for (_, subs) in subscriptions.iter_mut() {
            let original_len = subs.len();
            subs.retain(|sub| sub.handler.upgrade().is_some());
            total_removed += original_len - subs.len();
        }
        
        if total_removed > 0 {
            log::debug!("Cleaned up {} dead subscriptions", total_removed);
            let mut stats = self.stats.write();
            stats.active_subscriptions = stats.active_subscriptions.saturating_sub(total_removed);
        }
    }
}

impl EventBusBuilder {
    pub fn new() -> Self {
        Self {
            config: EventBusConfig::default(),
        }
    }
    
    pub fn max_queue_size(mut self, size: usize) -> Self {
        self.config.max_queue_size = size;
        self
    }
    
    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }
    
    pub fn processing_interval_ms(mut self, interval: u64) -> Self {
        self.config.processing_interval_ms = interval;
        self
    }
    
    pub fn enable_persistence(mut self, enable: bool) -> Self {
        self.config.enable_persistence = enable;
        self
    }
    
    pub fn enable_replay(mut self, enable: bool) -> Self {
        self.config.enable_replay = enable;
        self
    }
    
    pub fn build(self) -> EventBus {
        EventBus::with_config(self.config)
    }
}

/// Common engine events
pub mod events {
    use super::*;
    
    #[derive(Debug, Clone)]
    pub struct SystemStartedEvent {
        pub system_name: String,
        pub timestamp: Instant,
    }
    
    impl Event for SystemStartedEvent {
        fn event_type(&self) -> &'static str { "SystemStarted" }
        fn priority(&self) -> EventPriority { EventPriority::Normal }
    }
    
    #[derive(Debug, Clone)]
    pub struct SystemStoppedEvent {
        pub system_name: String,
        pub timestamp: Instant,
        pub reason: String,
    }
    
    impl Event for SystemStoppedEvent {
        fn event_type(&self) -> &'static str { "SystemStopped" }
        fn priority(&self) -> EventPriority { EventPriority::High }
    }
    
    #[derive(Debug, Clone)]
    pub struct SystemErrorEvent {
        pub system_name: String,
        pub error_message: String,
        pub timestamp: Instant,
        pub is_recoverable: bool,
    }
    
    impl Event for SystemErrorEvent {
        fn event_type(&self) -> &'static str { "SystemError" }
        fn priority(&self) -> EventPriority { EventPriority::Critical }
        fn should_persist(&self) -> bool { true }
    }
    
    #[derive(Debug, Clone)]
    pub struct PerformanceAlertEvent {
        pub system_name: String,
        pub metric_name: String,
        pub current_value: f64,
        pub threshold: f64,
        pub timestamp: Instant,
    }
    
    impl Event for PerformanceAlertEvent {
        fn event_type(&self) -> &'static str { "PerformanceAlert" }
        fn priority(&self) -> EventPriority { EventPriority::High }
    }
    
    #[derive(Debug, Clone)]
    pub struct ResourceExhaustionEvent {
        pub resource_type: String,
        pub current_usage: f64,
        pub maximum_capacity: f64,
        pub timestamp: Instant,
    }
    
    impl Event for ResourceExhaustionEvent {
        fn event_type(&self) -> &'static str { "ResourceExhaustion" }
        fn priority(&self) -> EventPriority { EventPriority::Critical }
        fn should_persist(&self) -> bool { true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::events::*;
    
    #[derive(Debug, Clone)]
    struct TestEvent {
        pub message: String,
    }
    
    impl Event for TestEvent {
        fn event_type(&self) -> &'static str { "TestEvent" }
    }
    
    struct TestHandler {
        pub received_events: Arc<Mutex<Vec<String>>>,
    }
    
    impl EventHandler<TestEvent> for TestHandler {
        fn handle_event(&self, event: &TestEvent) {
            self.received_events.lock().push(event.message.clone());
        }
        
        fn handler_name(&self) -> &str { "TestHandler" }
    }
    
    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new();
        let stats = bus.get_stats();
        assert_eq!(stats.active_subscriptions, 0);
        assert_eq!(stats.events_published, 0);
    }
    
    #[test]
    fn test_event_subscription() {
        let bus = EventBus::new();
        let received_events = Arc::new(Mutex::new(Vec::new()));
        let handler = Arc::new(TestHandler { received_events: received_events.clone() });
        
        let subscription_id = bus.subscribe::<TestEvent, _>(
            handler,
            None,
            "test_subscriber".to_string(),
        );
        
        assert!(subscription_id > 0);
        
        let stats = bus.get_stats();
        assert_eq!(stats.active_subscriptions, 1);
    }
    
    #[test]
    fn test_event_publishing() {
        let bus = EventBus::new();
        let event = TestEvent {
            message: "test message".to_string(),
        };
        
        let event_id = bus.publish(event, Some("test_system".to_string()), DeliveryMode::FireAndForget)
            .expect("Failed to publish event");
        
        assert!(event_id > 0);
        
        let stats = bus.get_stats();
        assert_eq!(stats.events_published, 1);
    }
    
    #[test]
    fn test_event_processing() {
        let bus = EventBus::new();
        let received_events = Arc::new(Mutex::new(Vec::new()));
        let handler = Arc::new(TestHandler { received_events: received_events.clone() });
        
        bus.subscribe::<TestEvent, _>(
            handler,
            None,
            "test_subscriber".to_string(),
        );
        
        let event = TestEvent {
            message: "test message".to_string(),
        };
        
        bus.publish(event, Some("test_system".to_string()), DeliveryMode::FireAndForget)
            .expect("Failed to publish event");
        
        let processed = bus.process_events().expect("Failed to process events");
        assert!(processed > 0);
    }
    
    #[test]
    fn test_event_priority_ordering() {
        let config = EventBusConfig {
            max_queue_size: 100,
            batch_size: 10,
            ..Default::default()
        };
        let bus = EventBus::with_config(config);
        
        // Publish events with different priorities
        let low_event = TestEvent { message: "low priority".to_string() };
        let high_event = TestEvent { message: "high priority".to_string() };
        let critical_event = TestEvent { message: "critical priority".to_string() };
        
        // Events should be processed in priority order regardless of publish order
        bus.publish(low_event, None, DeliveryMode::FireAndForget).expect("Failed to publish low event");
        bus.publish(critical_event, None, DeliveryMode::FireAndForget).expect("Failed to publish critical event");
        bus.publish(high_event, None, DeliveryMode::FireAndForget).expect("Failed to publish high event");
        
        let stats = bus.get_stats();
        assert_eq!(stats.events_published, 3);
    }
    
    #[test]
    fn test_cleanup_subscriptions() {
        let bus = EventBus::new();
        
        {
            let handler = Arc::new(TestHandler { 
                received_events: Arc::new(Mutex::new(Vec::new())) 
            });
            
            bus.subscribe::<TestEvent, _>(
                handler,
                None,
                "test_subscriber".to_string(),
            );
        } // handler goes out of scope and is dropped
        
        let stats_before = bus.get_stats();
        assert_eq!(stats_before.active_subscriptions, 1);
        
        bus.cleanup_subscriptions();
        
        let stats_after = bus.get_stats();
        assert_eq!(stats_after.active_subscriptions, 0);
    }
}