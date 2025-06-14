use earth_engine::{
    Engine, EngineConfig, BlockId, VoxelPos,
    BlockRegistry, Block, RenderData, PhysicsProperties,
};
use earth_engine::game::{GameData, GameContext, register_game_blocks, update_game, handle_block_break, handle_block_place, get_active_block_from_game};

// Define some basic blocks for the test - DOP style
struct TestGrassBlockData;
struct TestDirtBlockData;
struct TestStoneBlockData;

/// Get grass block ID
/// Pure function - returns grass block identifier
fn get_grass_block_id(_block: &TestGrassBlockData) -> BlockId { BlockId::GRASS }

/// Get grass block render data
/// Pure function - returns grass rendering properties
fn get_grass_render_data(_block: &TestGrassBlockData) -> RenderData {
    RenderData {
        color: [0.3, 0.7, 0.2],
        texture_id: 0,
    }
}

/// Get grass block physics properties
/// Pure function - returns grass physics properties
fn get_grass_physics_properties(_block: &TestGrassBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 1200.0,
    }
}

/// Get grass block name
/// Pure function - returns grass block name
fn get_grass_name(_block: &TestGrassBlockData) -> &'static str { "Grass" }

/// Get dirt block ID
/// Pure function - returns dirt block identifier
fn get_dirt_block_id(_block: &TestDirtBlockData) -> BlockId { BlockId::DIRT }

/// Get dirt block render data
/// Pure function - returns dirt rendering properties
fn get_dirt_render_data(_block: &TestDirtBlockData) -> RenderData {
    RenderData {
        color: [0.5, 0.3, 0.1],
        texture_id: 0,
    }
}

/// Get dirt block physics properties
/// Pure function - returns dirt physics properties
fn get_dirt_physics_properties(_block: &TestDirtBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 1500.0,
    }
}

/// Get dirt block name
/// Pure function - returns dirt block name
fn get_dirt_name(_block: &TestDirtBlockData) -> &'static str { "Dirt" }

/// Get stone block ID
/// Pure function - returns stone block identifier
fn get_stone_block_id(_block: &TestStoneBlockData) -> BlockId { BlockId::STONE }

/// Get stone block render data
/// Pure function - returns stone rendering properties
fn get_stone_render_data(_block: &TestStoneBlockData) -> RenderData {
    RenderData {
        color: [0.6, 0.6, 0.6],
        texture_id: 0,
    }
}

/// Get stone block physics properties
/// Pure function - returns stone physics properties
fn get_stone_physics_properties(_block: &TestStoneBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 2500.0,
    }
}

/// Get stone block name
/// Pure function - returns stone block name
fn get_stone_name(_block: &TestStoneBlockData) -> &'static str { "Stone" }

// Legacy Block implementations for compatibility
impl Block for TestGrassBlockData {
    fn get_id(&self) -> BlockId { get_grass_block_id(self) }
    fn get_render_data(&self) -> RenderData { get_grass_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_grass_physics_properties(self) }
    fn get_name(&self) -> &str { get_grass_name(self) }
}

impl Block for TestDirtBlockData {
    fn get_id(&self) -> BlockId { get_dirt_block_id(self) }
    fn get_render_data(&self) -> RenderData { get_dirt_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_dirt_physics_properties(self) }
    fn get_name(&self) -> &str { get_dirt_name(self) }
}

impl Block for TestStoneBlockData {
    fn get_id(&self) -> BlockId { get_stone_block_id(self) }
    fn get_render_data(&self) -> RenderData { get_stone_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_stone_physics_properties(self) }
    fn get_name(&self) -> &str { get_stone_name(self) }
}

/// Test game data structure (DOP - no methods)
#[derive(Default)]
struct TestGameData {
    frame_counter: u32,
}

impl GameData for TestGameData {}

/// Register blocks for test game
/// Function - transforms registry by adding test blocks
fn register_test_game_blocks(game: &mut TestGameData, registry: &mut BlockRegistry) {
    let _ = game;
    // Register basic blocks for testing
    registry.register("test:grass", TestGrassBlockData);
    registry.register("test:dirt", TestDirtBlockData);
    registry.register("test:stone", TestStoneBlockData);
    
    log::info!("[TestGame] Registered {} blocks", 3);
}

/// Get active block for test game
/// Pure function - returns active block from test game data
fn get_test_game_active_block(_game: &TestGameData) -> BlockId {
    BlockId(1) // Grass block
}

/// Update test game state
/// Function - transforms test game data
fn update_test_game(game: &mut TestGameData, _ctx: &mut GameContext, _delta_time: f32) {
    // Log camera position periodically
    game.frame_counter += 1;
    if game.frame_counter % 60 == 0 {
        log::info!("[TestGame] Frame {}", game.frame_counter);
    }
}

/// Handle block break for test game
/// Function - processes block break event for test game
fn handle_test_game_block_break(_game: &mut TestGameData, pos: VoxelPos, block_id: BlockId) {
    log::info!("[TestGame] Block broken at {:?}: {:?}", pos, block_id);
}

/// Handle block place for test game
/// Function - processes block place event for test game
fn handle_test_game_block_place(_game: &mut TestGameData, pos: VoxelPos, block_id: BlockId) {
    log::info!("[TestGame] Block placed at {:?}: {:?}", pos, block_id);
}

fn main() -> anyhow::Result<()> {
    // Initialize logger with debug level
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,earth_engine=debug"))
        .init();
    
    log::info!("[test_render] Starting render test...");
    
    let config = EngineConfig {
        window_title: "Earth Engine - Render Test".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 8,
    };
    
    let engine = Engine::new(config);
    let mut game = TestGameData::default();
    
    log::info!("[test_render] Launching engine...");
    
    // Note: Engine.run() may need updates to use DOP approach
    // For now, this demonstrates the DOP game structure
    log::info!("[test_render] Game data created successfully!");
    log::info!("[test_render] DOP functions available:");
    log::info!("  - register_test_game_blocks");
    log::info!("  - update_test_game");
    log::info!("  - handle_test_game_block_break");
    log::info!("  - handle_test_game_block_place");
    
    Ok(())
}