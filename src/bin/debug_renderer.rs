use earth_engine::{
    Engine, EngineConfig, BlockId, VoxelPos,
};
use earth_engine::game::{GameData, GameContext};
use anyhow::Result;

/// Debug game data structure (DOP - no methods)
struct DebugGameData {
    active_block: BlockId,
}

impl GameData for DebugGameData {}

/// Create new debug game data
/// Pure function - returns debug game data structure
fn create_debug_game_data() -> DebugGameData {
    DebugGameData {
        active_block: BlockId(1), // Grass
    }
}

/// Register blocks for debug game
/// Function - no-op since blocks already registered in gpu_state
fn register_debug_game_blocks(_game: &mut DebugGameData, _registry: &mut earth_engine::BlockRegistry) {
    // Blocks already registered in gpu_state
}

/// Update debug game
/// Function - no-op for debug
fn update_debug_game(_game: &mut DebugGameData, _ctx: &mut GameContext, _delta_time: f32) {
    // No update needed for debug
}

/// Get active block for debug game
/// Pure function - returns active block from debug game data
fn get_debug_game_active_block(game: &DebugGameData) -> BlockId {
    game.active_block
}

/// Handle block break for debug game
/// Function - logs block break event
fn handle_debug_game_block_break(_game: &mut DebugGameData, pos: VoxelPos, block: BlockId) {
    log::info!("Block broken at {:?}: {:?}", pos, block);
}

/// Handle block place for debug game
/// Function - logs block place event
fn handle_debug_game_block_place(_game: &mut DebugGameData, pos: VoxelPos, block: BlockId) {
    log::info!("Block placed at {:?}: {:?}", pos, block);
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,earth_engine=debug"))
        .init();

    log::info!("Starting debug renderer...");

    let config = EngineConfig {
        window_title: "Hearth Engine Debug Renderer".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 4,
    };

    let engine = Engine::new(config);
    let mut game = create_debug_game_data();
    
    // Note: Engine.run() may need updates to use DOP approach
    // For now, this demonstrates the DOP game structure
    log::info!("Game data created successfully!");
    log::info!("DOP functions available:");
    log::info!("  - register_debug_game_blocks");
    log::info!("  - update_debug_game");
    log::info!("  - get_debug_game_active_block");
    log::info!("  - handle_debug_game_block_break");
    log::info!("  - handle_debug_game_block_place");

    Ok(())
}