//! Entity operation handlers - pure functions only

use crate::gateway::{
    GatewayState, EngineResponse, EngineEvent,
    types::*, queue_event
};
use cgmath::Point3;

// Entity storage will be added later - for now return placeholder responses
static mut NEXT_ENTITY_ID: u64 = 1;

/// Handle spawn entity request
pub fn handle_spawn_entity(
    state: &GatewayState,
    descriptor: EntityDescriptor,
) -> EngineResponse {
    // TODO: Implement actual entity spawning when entity system is ready
    let entity_id = unsafe {
        let id = NEXT_ENTITY_ID;
        NEXT_ENTITY_ID += 1;
        EntityId(id)
    };
    
    // Queue spawn event
    queue_event(state, EngineEvent::EntitySpawned {
        entity_id,
        descriptor: descriptor.clone(),
    });
    
    EngineResponse::Entity { entity_id }
}

/// Handle despawn entity request
pub fn handle_despawn_entity(
    state: &GatewayState,
    entity_id: EntityId,
) -> EngineResponse {
    // TODO: Implement actual entity despawning
    
    // Queue despawn event
    queue_event(state, EngineEvent::EntityDespawned { entity_id });
    
    EngineResponse::Success
}

/// Handle get entity transform request
pub fn handle_get_entity_transform(
    state: &GatewayState,
    entity_id: EntityId,
) -> EngineResponse {
    // TODO: Implement actual transform lookup
    EngineResponse::Error { 
        message: "Entity system not yet implemented".to_string() 
    }
}

/// Handle set entity transform request
pub fn handle_set_entity_transform(
    state: &GatewayState,
    entity_id: EntityId,
    transform: Transform,
) -> EngineResponse {
    // TODO: Implement actual transform setting
    
    // Queue movement event
    queue_event(state, EngineEvent::EntityMoved {
        entity_id,
        old_pos: Point3::new(0.0, 0.0, 0.0), // TODO: Get actual old position
        new_pos: transform.position,
    });
    
    EngineResponse::Success
}

/// Handle query entities request
pub fn handle_query_entities(
    state: &GatewayState,
    filter: EntityFilter,
) -> EngineResponse {
    // TODO: Implement actual entity querying
    EngineResponse::Entities { entities: Vec::new() }
}