use crate::camera::{calculate_forward_vector, CameraData};
use crate::input::InputState;
use crate::{cast_ray, BlockId, BlockRegistry, Ray, RaycastHit, VoxelPos, WorldInterface};
use crate::world::functional_wrapper;
use cgmath::Point3;

// Gateway modules (new DOP system)
pub mod gateway_data;
pub mod gateway_operations;

// Legacy callback module (to be removed)
pub mod callbacks;

// Re-export gateway types
pub use gateway_data::{
    GameEvent, GameCommand, GameOperations, GameDataAccess, GameDataHandle,
    InteractionType, MessageType, BlockRegistration, BlockProperties,
    EngineStateView, InputStateView, WorldInfoView, PlayerInfo,
    GameGatewayData, GatewayConfig, GatewayMetrics,
};

pub use gateway_operations::{
    init_gateway, shutdown_gateway, queue_event, queue_events,
    process_update, register_blocks, get_active_block,
    save_game_state, load_game_state, get_metrics, reset_metrics,
    is_gateway_initialized, get_gateway_config, update_gateway_config,
};

// Legacy exports for compatibility
pub use callbacks::{get_game_callbacks, register_game_callbacks, GameCallbacks};

/// Game data structure (DOP - no methods)
/// Pure data structure for game state
pub trait GameData: Send + Sync + 'static {}

/// Register blocks in the registry
/// Function - transforms registry data by registering game blocks
pub fn register_game_blocks<T: GameData + 'static>(game: &mut T, registry: &mut BlockRegistry) {
    let _ = game; // Avoid unused warning
    
    // Try new gateway first
    if is_gateway_initialized() {
        register_blocks(registry);
    } else {
        // Fall back to legacy callbacks
        callbacks::execute_register_blocks(registry);
    }
}

/// Update game state
/// Function - transforms game data based on context and time
pub fn update_game<T: GameData + 'static>(game: &mut T, ctx: &mut GameContext, delta_time: f32) {
    // Try new gateway first
    if is_gateway_initialized() {
        // Convert context to engine buffers and process
        // For now, we'll need to create a compatibility layer
        // This will be handled by the game implementation
    } else {
        // Fall back to legacy callbacks
        let game_any = game as &mut dyn std::any::Any;
        callbacks::execute_update_game(game_any, ctx, delta_time);
    }
}

/// Handle block break event
/// Function - processes block break for game data
pub fn handle_block_break<T: GameData + 'static>(game: &mut T, pos: VoxelPos, block: BlockId) {
    // Queue event to new gateway if available
    if is_gateway_initialized() {
        queue_event(GameEvent::BlockBreak {
            position: pos,
            block_id: block,
            player_id: None, // TODO: Get from context
        });
    } else {
        // Fall back to legacy callbacks
        let game_any = game as &mut dyn std::any::Any;
        callbacks::execute_on_block_break(game_any, pos, block);
    }
}

/// Handle block place event
/// Function - processes block place for game data
pub fn handle_block_place<T: GameData + 'static>(game: &mut T, pos: VoxelPos, block: BlockId) {
    // Queue event to new gateway if available
    if is_gateway_initialized() {
        queue_event(GameEvent::BlockPlace {
            position: pos,
            block_id: block,
            player_id: None, // TODO: Get from context
        });
    } else {
        // Fall back to legacy callbacks
        let game_any = game as &mut dyn std::any::Any;
        callbacks::execute_on_block_place(game_any, pos, block);
    }
}

/// Get the active block for placement
/// Pure function - reads active block from game data
pub fn get_active_block_from_game<T: GameData + 'static>(game: &T) -> BlockId {
    // Try new gateway first
    if is_gateway_initialized() {
        get_active_block()
    } else {
        // Fall back to legacy callbacks
        let game_any = game as &dyn std::any::Any;
        callbacks::execute_get_active_block(game_any)
    }
}

/// Context passed to game update functions
pub struct GameContext<'a> {
    pub world: &'a mut dyn WorldInterface,
    pub registry: &'a BlockRegistry,
    pub camera: &'a CameraData,
    pub input: &'a InputState,
    pub selected_block: Option<RaycastHit>,
}

/// DOP version of game context that uses engine buffers
pub struct GameContextDOP<'a> {
    pub buffers: &'a mut crate::EngineBuffers,
    pub registry: &'a BlockRegistry,
    pub selected_block: Option<RaycastHit>,
    pub chunk_size: u32,
}

/// Cast a ray from the camera and find what block is being looked at
/// Pure function - calculates raycast from camera data
pub fn cast_camera_ray_from_context(ctx: &GameContext, max_distance: f32) -> Option<RaycastHit> {
    let position = Point3::new(
        ctx.camera.position[0],
        ctx.camera.position[1],
        ctx.camera.position[2],
    );
    let forward = calculate_forward_vector(ctx.camera);
    let ray = Ray::new(position, forward);
    functional_wrapper::raycast(&*ctx.world, ray, max_distance)
}

/// Break a block at the given position
/// Function - transforms world data by breaking block
pub fn break_block_in_context(ctx: &mut GameContext, pos: VoxelPos) -> bool {
    let block = functional_wrapper::get_block(&*ctx.world, pos);
    if block != BlockId::AIR {
        match functional_wrapper::set_block(ctx.world, pos, BlockId::AIR) {
            Ok(_) => true,
            Err(e) => {
                log::error!("[Game] Failed to break block at {:?}: {}", pos, e);
                false
            }
        }
    } else {
        false
    }
}

/// Place a block at the given position
/// Function - transforms world data by placing block
pub fn place_block_in_context(ctx: &mut GameContext, pos: VoxelPos, block_id: BlockId) -> bool {
    let current = functional_wrapper::get_block(&*ctx.world, pos);
    if current == BlockId::AIR {
        match functional_wrapper::set_block(ctx.world, pos, block_id) {
            Ok(_) => true,
            Err(e) => {
                log::error!(
                    "[Game] Failed to place block {:?} at {:?}: {}",
                    block_id,
                    pos,
                    e
                );
                false
            }
        }
    } else {
        false
    }
}

// ============================================================================
// DOP Versions - Operating on EngineBuffers
// ============================================================================

/// Update game state using DOP buffers
/// Function - transforms game data using centralized buffers
pub fn update_game_dop<T: GameData + 'static>(
    game: &mut T,
    buffers: &mut crate::EngineBuffers,
    registry: &BlockRegistry,
    delta_time: f32,
) {
    // Convert buffers to a context for backwards compatibility
    // In future, callbacks should directly use buffers
    let mut ctx = GameContextDOP {
        buffers,
        registry,
        selected_block: None,
        chunk_size: 50, // TODO: Get from config
    };
    
    // Update game-specific data in game buffers
    let game_any = game as &mut dyn std::any::Any;
    // TODO: Update callbacks to use DOP context
    // callbacks::execute_update_game_dop(game_any, &mut ctx, delta_time);
}

/// Cast a ray from the camera using DOP buffers
/// Pure function - calculates raycast using buffer data
pub fn cast_camera_ray_dop(
    buffers: &crate::EngineBuffers,
    max_distance: f32,
    chunk_size: u32,
) -> Option<RaycastHit> {
    use crate::world::world_operations;
    
    let camera_pos = buffers.render.camera_position;
    let position = Point3::new(camera_pos[0], camera_pos[1], camera_pos[2]);
    
    // Calculate forward vector from view matrix
    let view_matrix = buffers.render.view_matrix;
    let forward = cgmath::Vector3::new(
        -view_matrix[2],
        -view_matrix[6],
        -view_matrix[10],
    ).normalize();
    
    let ray = Ray::new(position, forward);
    
    // Use DOP world operations
    world_operations::raycast(&buffers.world.chunks[0].into(), ray, max_distance, chunk_size)
}

/// Break a block using DOP buffers
/// Function - transforms world data in buffers by breaking block
pub fn break_block_dop(
    buffers: &mut crate::EngineBuffers,
    pos: VoxelPos,
    chunk_size: u32,
) -> bool {
    use crate::world::{world_operations, data_types::WorldData};
    
    // Convert buffer data to WorldData for operations
    // TODO: Update world_operations to work directly with WorldBuffers
    let mut world_data = WorldData {
        chunks: buffers.world.chunks.clone(),
        size_x: buffers.world.world_size[0],
        size_y: buffers.world.world_size[1],
        size_z: buffers.world.world_size[2],
        chunk_capacity: buffers.world.chunks.capacity(),
        active_chunks: buffers.world.active_chunks.clone(),
        seed: buffers.world.world_seed,
        tick: buffers.world.world_tick,
    };
    
    let block = world_operations::get_block(&world_data, pos, chunk_size);
    if block != BlockId::AIR {
        match world_operations::set_block(&mut world_data, pos, BlockId::AIR, chunk_size) {
            Ok(modification) => {
                // Update buffers with modified world data
                buffers.world.chunks = world_data.chunks;
                buffers.world.active_chunks = world_data.active_chunks;
                buffers.world.modifications.push_back(modification);
                true
            }
            Err(e) => {
                log::error!("[Game DOP] Failed to break block at {:?}: {:?}", pos, e);
                false
            }
        }
    } else {
        false
    }
}

/// Place a block using DOP buffers
/// Function - transforms world data in buffers by placing block
pub fn place_block_dop(
    buffers: &mut crate::EngineBuffers,
    pos: VoxelPos,
    block_id: BlockId,
    chunk_size: u32,
) -> bool {
    use crate::world::{world_operations, data_types::WorldData};
    
    // Convert buffer data to WorldData for operations
    let mut world_data = WorldData {
        chunks: buffers.world.chunks.clone(),
        size_x: buffers.world.world_size[0],
        size_y: buffers.world.world_size[1],
        size_z: buffers.world.world_size[2],
        chunk_capacity: buffers.world.chunks.capacity(),
        active_chunks: buffers.world.active_chunks.clone(),
        seed: buffers.world.world_seed,
        tick: buffers.world.world_tick,
    };
    
    let current = world_operations::get_block(&world_data, pos, chunk_size);
    if current == BlockId::AIR {
        match world_operations::set_block(&mut world_data, pos, block_id, chunk_size) {
            Ok(modification) => {
                // Update buffers with modified world data
                buffers.world.chunks = world_data.chunks;
                buffers.world.active_chunks = world_data.active_chunks;
                buffers.world.modifications.push_back(modification);
                true
            }
            Err(e) => {
                log::error!(
                    "[Game DOP] Failed to place block {:?} at {:?}: {:?}",
                    block_id,
                    pos,
                    e
                );
                false
            }
        }
    } else {
        false
    }
}
