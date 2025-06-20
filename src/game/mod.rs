use crate::{BlockId, BlockRegistry, VoxelPos, WorldInterface, Ray, RaycastHit, cast_ray};
use crate::camera::{CameraData, calculate_forward_vector};
use crate::input::InputState;
use cgmath::Point3;

pub mod callbacks;
pub use callbacks::{GameCallbacks, register_game_callbacks, get_game_callbacks};

/// Game data structure (DOP - no methods)
/// Pure data structure for game state
pub trait GameData: Send + Sync {}

/// Register blocks in the registry
/// Function - transforms registry data by registering game blocks
pub fn register_game_blocks<T: GameData>(game: &mut T, registry: &mut BlockRegistry) {
    let _ = game; // Avoid unused warning
    callbacks::execute_register_blocks(registry);
}

/// Update game state
/// Function - transforms game data based on context and time
pub fn update_game<T: GameData>(game: &mut T, ctx: &mut GameContext, delta_time: f32) {
    // Use Any to allow game-specific data types
    let game_any = game as &mut dyn std::any::Any;
    callbacks::execute_update_game(game_any, ctx, delta_time);
}

/// Handle block break event
/// Function - processes block break for game data
pub fn handle_block_break<T: GameData>(game: &mut T, pos: VoxelPos, block: BlockId) {
    let game_any = game as &mut dyn std::any::Any;
    callbacks::execute_on_block_break(game_any, pos, block);
}

/// Handle block place event
/// Function - processes block place for game data
pub fn handle_block_place<T: GameData>(game: &mut T, pos: VoxelPos, block: BlockId) {
    let game_any = game as &mut dyn std::any::Any;
    callbacks::execute_on_block_place(game_any, pos, block);
}

/// Get the active block for placement
/// Pure function - reads active block from game data
pub fn get_active_block_from_game<T: GameData>(game: &T) -> BlockId {
    let game_any = game as &dyn std::any::Any;
    callbacks::execute_get_active_block(game_any)
}


/// Context passed to game update functions
pub struct GameContext<'a> {
    pub world: &'a mut dyn WorldInterface,
    pub registry: &'a BlockRegistry,
    pub camera: &'a CameraData,
    pub input: &'a InputState,
    pub selected_block: Option<RaycastHit>,
}

/// Cast a ray from the camera and find what block is being looked at
/// Pure function - calculates raycast from camera data
pub fn cast_camera_ray_from_context(ctx: &GameContext, max_distance: f32) -> Option<RaycastHit> {
    let position = Point3::new(
        ctx.camera.position[0], 
        ctx.camera.position[1], 
        ctx.camera.position[2]
    );
    let forward = calculate_forward_vector(ctx.camera);
    let ray = Ray::new(position, forward);
    cast_ray(&*ctx.world, ray, max_distance)
}

/// Break a block at the given position
/// Function - transforms world data by breaking block
pub fn break_block_in_context(ctx: &mut GameContext, pos: VoxelPos) -> bool {
    let block = ctx.world.get_block(pos);
    if block != BlockId::AIR {
        ctx.world.set_block(pos, BlockId::AIR);
        true
    } else {
        false
    }
}

/// Place a block at the given position
/// Function - transforms world data by placing block
pub fn place_block_in_context(ctx: &mut GameContext, pos: VoxelPos, block_id: BlockId) -> bool {
    let current = ctx.world.get_block(pos);
    if current == BlockId::AIR {
        ctx.world.set_block(pos, block_id);
        true
    } else {
        false
    }
}

