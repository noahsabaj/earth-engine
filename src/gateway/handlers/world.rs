//! World operation handlers - pure functions only

use crate::gateway::{
    GatewayState, EngineResponse, EngineEvent, 
    types::*, queue_event
};
use crate::world::interfaces::WorldInterface;
use cgmath::{Point3, Vector3};

/// Handle get block request
pub fn handle_get_block(state: &GatewayState, pos: VoxelPos) -> EngineResponse {
    let world = state.world_manager.read();
    let engine_pos = voxel_to_engine(pos);
    let block_id = world.get_block(engine_pos);
    EngineResponse::Block { block_id: engine_to_block(block_id) }
}

/// Handle set block request
pub fn handle_set_block(state: &GatewayState, pos: VoxelPos, block_id: BlockId) -> EngineResponse {
    let mut world = state.world_manager.write();
    let engine_pos = voxel_to_engine(pos);
    let engine_block = block_to_engine(block_id);
    
    // Get old block for event
    let old_block = world.get_block(engine_pos);
    
    match world.set_block(engine_pos, engine_block) {
        Ok(()) => {
            // Queue block changed event
            queue_event(state, EngineEvent::BlockChanged {
                pos,
                old_block: engine_to_block(old_block),
                new_block: block_id,
            });
            
            // Queue chunk modified event
            let chunk_pos = ChunkPos {
                x: pos.x / 32,
                y: pos.y / 32,
                z: pos.z / 32,
            };
            queue_event(state, EngineEvent::ChunkModified { chunk_pos });
            
            EngineResponse::Success
        }
        Err(e) => EngineResponse::Error { 
            message: format!("Failed to set block: {}", e) 
        }
    }
}

/// Handle batch set blocks request
pub fn handle_batch_set_blocks(
    state: &GatewayState, 
    changes: Vec<(VoxelPos, BlockId)>
) -> EngineResponse {
    let mut world = state.world_manager.write();
    let mut modified_chunks = std::collections::HashSet::new();
    let mut errors = Vec::new();
    
    for (pos, block_id) in changes {
        let engine_pos = voxel_to_engine(pos);
        let engine_block = block_to_engine(block_id);
        let old_block = world.get_block(engine_pos);
        
        match world.set_block(engine_pos, engine_block) {
            Ok(()) => {
                // Queue event
                queue_event(state, EngineEvent::BlockChanged {
                    pos,
                    old_block: engine_to_block(old_block),
                    new_block: block_id,
                });
                
                // Track modified chunk
                let chunk_pos = ChunkPos {
                    x: pos.x / 32,
                    y: pos.y / 32,
                    z: pos.z / 32,
                };
                modified_chunks.insert(chunk_pos);
            }
            Err(e) => {
                errors.push(format!("Failed at {:?}: {}", pos, e));
            }
        }
    }
    
    // Queue chunk modified events
    for chunk_pos in modified_chunks {
        queue_event(state, EngineEvent::ChunkModified { chunk_pos });
    }
    
    if errors.is_empty() {
        EngineResponse::Success
    } else {
        EngineResponse::Error {
            message: format!("{} blocks failed: {}", errors.len(), errors.join("; "))
        }
    }
}

/// Handle raycast request
pub fn handle_raycast(
    state: &GatewayState,
    origin: Point3<f32>,
    direction: Vector3<f32>,
    max_distance: f32,
) -> EngineResponse {
    let world = state.world_manager.read();
    
    let ray = crate::world::core::Ray {
        origin,
        direction: direction.normalize(),
    };
    
    let hit = world.raycast(ray, max_distance);
    
    EngineResponse::RaycastHit {
        hit: hit.map(|h| RaycastHit {
            position: Point3::new(
                h.position.x as f32 + 0.5,
                h.position.y as f32 + 0.5,
                h.position.z as f32 + 0.5,
            ),
            normal: match h.face {
                crate::world::core::BlockFace::Top => Vector3::new(0.0, 1.0, 0.0),
                crate::world::core::BlockFace::Bottom => Vector3::new(0.0, -1.0, 0.0),
                crate::world::core::BlockFace::North => Vector3::new(0.0, 0.0, -1.0),
                crate::world::core::BlockFace::South => Vector3::new(0.0, 0.0, 1.0),
                crate::world::core::BlockFace::East => Vector3::new(1.0, 0.0, 0.0),
                crate::world::core::BlockFace::West => Vector3::new(-1.0, 0.0, 0.0),
            },
            distance: h.distance,
            entity_id: None,
            block_pos: Some(engine_to_voxel(h.position)),
        })
    }
}