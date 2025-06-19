//! Request dispatch - pure functions for handling gateway requests
//! No methods, no traits, just functions that transform data

use crate::gateway::{
    GatewayState, EngineRequest, EngineResponse, EngineEvent,
    types::*, handlers
};
use cgmath::InnerSpace;

/// Handle a single request and return response
pub fn handle_request(state: &GatewayState, request: EngineRequest) -> EngineResponse {
    match request {
        // World operations
        EngineRequest::GetBlock { pos } => {
            handlers::world::handle_get_block(state, pos)
        }
        
        EngineRequest::SetBlock { pos, block_id } => {
            handlers::world::handle_set_block(state, pos, block_id)
        }
        
        EngineRequest::BatchSetBlocks { changes } => {
            handlers::world::handle_batch_set_blocks(state, changes)
        }
        
        EngineRequest::Raycast { origin, direction, max_distance } => {
            handlers::world::handle_raycast(state, origin, direction, max_distance)
        }
        
        // Entity operations
        EngineRequest::SpawnEntity { descriptor } => {
            handlers::entity::handle_spawn_entity(state, descriptor)
        }
        
        EngineRequest::DespawnEntity { entity_id } => {
            handlers::entity::handle_despawn_entity(state, entity_id)
        }
        
        EngineRequest::GetEntityTransform { entity_id } => {
            handlers::entity::handle_get_entity_transform(state, entity_id)
        }
        
        EngineRequest::SetEntityTransform { entity_id, transform } => {
            handlers::entity::handle_set_entity_transform(state, entity_id, transform)
        }
        
        EngineRequest::QueryEntities { filter } => {
            handlers::entity::handle_query_entities(state, filter)
        }
        
        // Physics operations
        EngineRequest::ApplyImpulse { entity_id, impulse } => {
            handlers::physics::handle_apply_impulse(state, entity_id, impulse)
        }
        
        EngineRequest::SetVelocity { entity_id, velocity } => {
            handlers::physics::handle_set_velocity(state, entity_id, velocity)
        }
        
        EngineRequest::ApplyForce { entity_id, force } => {
            handlers::physics::handle_apply_force(state, entity_id, force)
        }
        
        // Rendering operations
        EngineRequest::SetCamera { camera } => {
            handlers::rendering::handle_set_camera(state, camera)
        }
        
        EngineRequest::SetRenderSettings { settings } => {
            handlers::rendering::handle_set_render_settings(state, settings)
        }
        
        EngineRequest::QueueParticleEffect { effect } => {
            handlers::rendering::handle_queue_particle_effect(state, effect)
        }
        
        EngineRequest::CaptureScreenshot { path } => {
            handlers::rendering::handle_capture_screenshot(state, path)
        }
        
        // Game state operations
        EngineRequest::SaveGame { slot } => {
            handlers::game::handle_save_game(state, slot)
        }
        
        EngineRequest::LoadGame { slot } => {
            handlers::game::handle_load_game(state, slot)
        }
        
        EngineRequest::GetGameState => {
            handlers::game::handle_get_game_state(state)
        }
    }
}

/// Batch process multiple requests
pub fn handle_batch_requests(
    state: &GatewayState, 
    requests: Vec<(u64, EngineRequest)>
) -> Vec<(u64, EngineResponse)> {
    requests.into_iter()
        .map(|(id, request)| (id, handle_request(state, request)))
        .collect()
}

/// Filter requests by type
pub fn filter_requests_by_type(
    requests: &[(u64, EngineRequest)]
) -> RequestsByType {
    let mut world_requests = Vec::new();
    let mut entity_requests = Vec::new();
    let mut physics_requests = Vec::new();
    let mut render_requests = Vec::new();
    let mut game_requests = Vec::new();
    
    for (id, request) in requests {
        match request {
            EngineRequest::GetBlock { .. } |
            EngineRequest::SetBlock { .. } |
            EngineRequest::BatchSetBlocks { .. } |
            EngineRequest::Raycast { .. } => {
                world_requests.push((*id, request.clone()));
            }
            
            EngineRequest::SpawnEntity { .. } |
            EngineRequest::DespawnEntity { .. } |
            EngineRequest::GetEntityTransform { .. } |
            EngineRequest::SetEntityTransform { .. } |
            EngineRequest::QueryEntities { .. } => {
                entity_requests.push((*id, request.clone()));
            }
            
            EngineRequest::ApplyImpulse { .. } |
            EngineRequest::SetVelocity { .. } |
            EngineRequest::ApplyForce { .. } => {
                physics_requests.push((*id, request.clone()));
            }
            
            EngineRequest::SetCamera { .. } |
            EngineRequest::SetRenderSettings { .. } |
            EngineRequest::QueueParticleEffect { .. } |
            EngineRequest::CaptureScreenshot { .. } => {
                render_requests.push((*id, request.clone()));
            }
            
            EngineRequest::SaveGame { .. } |
            EngineRequest::LoadGame { .. } |
            EngineRequest::GetGameState => {
                game_requests.push((*id, request.clone()));
            }
        }
    }
    
    RequestsByType {
        world: world_requests,
        entity: entity_requests,
        physics: physics_requests,
        render: render_requests,
        game: game_requests,
    }
}

/// Categorized requests data
pub struct RequestsByType {
    pub world: Vec<(u64, EngineRequest)>,
    pub entity: Vec<(u64, EngineRequest)>,
    pub physics: Vec<(u64, EngineRequest)>,
    pub render: Vec<(u64, EngineRequest)>,
    pub game: Vec<(u64, EngineRequest)>,
}

/// Validate request before processing
pub fn validate_request(request: &EngineRequest) -> Result<(), String> {
    match request {
        EngineRequest::Raycast { max_distance, .. } => {
            if *max_distance <= 0.0 {
                return Err("Raycast max_distance must be positive".to_string());
            }
        }
        
        EngineRequest::SaveGame { slot } |
        EngineRequest::LoadGame { slot } => {
            if *slot > 99 {
                return Err("Save slot must be between 0-99".to_string());
            }
        }
        
        EngineRequest::ApplyImpulse { impulse, .. } |
        EngineRequest::SetVelocity { velocity: impulse, .. } |
        EngineRequest::ApplyForce { force: impulse, .. } => {
            if impulse.magnitude() > 10000.0 {
                return Err("Physics force/impulse too large".to_string());
            }
        }
        
        _ => {}
    }
    
    Ok(())
}

/// Priority for request types
pub fn get_request_priority(request: &EngineRequest) -> u32 {
    match request {
        // High priority - immediate gameplay impact
        EngineRequest::GetBlock { .. } => 9,
        EngineRequest::Raycast { .. } => 9,
        EngineRequest::GetEntityTransform { .. } => 8,
        
        // Medium priority - state changes
        EngineRequest::SetBlock { .. } => 7,
        EngineRequest::SpawnEntity { .. } => 7,
        EngineRequest::ApplyImpulse { .. } => 6,
        
        // Low priority - bulk operations
        EngineRequest::BatchSetBlocks { .. } => 5,
        EngineRequest::QueryEntities { .. } => 4,
        
        // Background priority
        EngineRequest::SaveGame { .. } => 2,
        EngineRequest::CaptureScreenshot { .. } => 1,
        
        _ => 5, // Default medium priority
    }
}

/// Sort requests by priority
pub fn sort_requests_by_priority(
    requests: &mut Vec<(u64, EngineRequest)>
) {
    requests.sort_by_key(|(_, req)| std::cmp::Reverse(get_request_priority(req)));
}