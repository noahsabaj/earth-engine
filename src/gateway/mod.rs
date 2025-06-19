//! Engine Gateway API - DOP-compliant interface between games and engine
//! 
//! Pure data-oriented design: no traits, no methods, only data and functions

pub mod types;
pub mod dispatch;
pub mod handlers;

pub use types::*;
pub use dispatch::*;
pub use handlers::*;

use std::sync::Arc;
use parking_lot::RwLock;

/// Gateway state - pure data, no methods
pub struct GatewayState {
    /// Engine context data
    pub world_manager: Arc<RwLock<crate::world::management::UnifiedWorldManager>>,
    pub renderer: Arc<RwLock<crate::renderer::gpu_driven::GpuDrivenRenderer>>,
    pub physics: Arc<RwLock<crate::physics::GpuPhysicsWorld>>,
    
    /// Event queue for outgoing events
    pub event_queue: Arc<RwLock<Vec<EngineEvent>>>,
    
    /// Request processing queue
    pub request_queue: Arc<RwLock<Vec<(u64, EngineRequest)>>>,
    
    /// Response buffer
    pub response_buffer: Arc<RwLock<std::collections::HashMap<u64, EngineResponse>>>,
    
    /// Next request ID
    pub next_request_id: Arc<std::sync::atomic::AtomicU64>,
}

/// Initialize gateway state
pub fn create_gateway_state(
    world_manager: Arc<RwLock<crate::world::management::UnifiedWorldManager>>,
    renderer: Arc<RwLock<crate::renderer::gpu_driven::GpuDrivenRenderer>>,
    physics: Arc<RwLock<crate::physics::GpuPhysicsWorld>>,
) -> GatewayState {
    GatewayState {
        world_manager,
        renderer,
        physics,
        event_queue: Arc::new(RwLock::new(Vec::with_capacity(1024))),
        request_queue: Arc::new(RwLock::new(Vec::with_capacity(128))),
        response_buffer: Arc::new(RwLock::new(std::collections::HashMap::with_capacity(128))),
        next_request_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
    }
}

/// Submit a request to the gateway
pub fn submit_request(state: &GatewayState, request: EngineRequest) -> u64 {
    let request_id = state.next_request_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut queue = state.request_queue.write();
    queue.push((request_id, request));
    request_id
}

/// Process all pending requests
pub fn process_requests(state: &GatewayState) {
    let requests: Vec<(u64, EngineRequest)> = {
        let mut queue = state.request_queue.write();
        std::mem::take(&mut *queue)
    };
    
    for (request_id, request) in requests {
        let response = dispatch::handle_request(state, request);
        let mut buffer = state.response_buffer.write();
        buffer.insert(request_id, response);
    }
}

/// Get response for a request ID
pub fn get_response(state: &GatewayState, request_id: u64) -> Option<EngineResponse> {
    let mut buffer = state.response_buffer.write();
    buffer.remove(&request_id)
}

/// Poll for events
pub fn poll_events(state: &GatewayState, max_events: usize) -> Vec<EngineEvent> {
    let mut queue = state.event_queue.write();
    let drain_count = queue.len().min(max_events);
    queue.drain(..drain_count).collect()
}

/// Queue an event for delivery to game
pub fn queue_event(state: &GatewayState, event: EngineEvent) {
    let mut queue = state.event_queue.write();
    queue.push(event);
}