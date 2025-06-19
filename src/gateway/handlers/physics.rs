//! Physics operation handlers - pure functions only

use crate::gateway::{
    GatewayState, EngineResponse,
    types::*,
};
use cgmath::Vector3;

/// Handle apply impulse request
pub fn handle_apply_impulse(
    state: &GatewayState,
    entity_id: EntityId,
    impulse: Vector3<f32>,
) -> EngineResponse {
    let mut physics = state.physics.write();
    
    // Convert to physics entity ID (assuming 1:1 mapping for now)
    let physics_entity = crate::physics::EntityId(entity_id.0 as u32);
    
    // Apply the impulse through physics system
    physics.apply_impulse(physics_entity, impulse);
    
    EngineResponse::Success
}

/// Handle set velocity request
pub fn handle_set_velocity(
    state: &GatewayState,
    entity_id: EntityId,
    velocity: Vector3<f32>,
) -> EngineResponse {
    let mut physics = state.physics.write();
    
    // Convert to physics entity ID
    let physics_entity = crate::physics::EntityId(entity_id.0 as u32);
    
    // Set velocity through physics system
    physics.set_velocity(physics_entity, velocity);
    
    EngineResponse::Success
}

/// Handle apply force request
pub fn handle_apply_force(
    state: &GatewayState,
    entity_id: EntityId,
    force: Vector3<f32>,
) -> EngineResponse {
    let mut physics = state.physics.write();
    
    // Convert to physics entity ID
    let physics_entity = crate::physics::EntityId(entity_id.0 as u32);
    
    // Apply force (continuous, unlike impulse which is instant)
    physics.apply_force(physics_entity, force);
    
    EngineResponse::Success
}