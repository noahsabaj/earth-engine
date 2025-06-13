#![deny(warnings, clippy::all)]

/// Main Earth Engine game executable
/// This is the primary entry point for the full game experience
/// Features: block placement/breaking, multiple block types, full input handling

use earth_engine::{Engine, EngineConfig, Game, GameContext};
use earth_engine::world::{BlockId, BlockRegistry};
use earth_engine::input::KeyCode;

/// Full game implementation with complete gameplay features
struct EarthGame {
    player_block: BlockId,
}

impl EarthGame {
    fn new() -> Self {
        Self {
            player_block: BlockId(1), // Stone
        }
    }
}

impl Game for EarthGame {
    fn register_blocks(&mut self, _registry: &mut BlockRegistry) {
        // Blocks are already registered in the world module
        log::info!("Blocks registered");
    }
    
    fn update(&mut self, ctx: &mut GameContext, _delta_time: f32) {
        // Block breaking is now handled by the engine's GpuState with progressive breaking
        // Block placing is also handled by the engine
        // The game just needs to handle block type selection
        
        // Switch blocks with number keys
        if ctx.input.is_key_pressed(KeyCode::Digit1) {
            self.player_block = BlockId(1); // Stone
        } else if ctx.input.is_key_pressed(KeyCode::Digit2) {
            self.player_block = BlockId(2); // Dirt
        } else if ctx.input.is_key_pressed(KeyCode::Digit3) {
            self.player_block = BlockId(3); // Grass
        } else if ctx.input.is_key_pressed(KeyCode::Digit4) {
            self.player_block = BlockId(4); // Wood
        }
    }
    
    fn get_active_block(&self) -> BlockId {
        self.player_block
    }
}

fn main() {
    println!("[MAIN] Starting Earth Engine...");
    
    // Initialize logging first so we can see what's happening
    env_logger::Builder::from_env(env_logger::Env::default().   default_filter_or("debug"))
        .format_timestamp_millis()
        .init();
    
    log::info!("[MAIN] Logger initialized");
    
    // Install panic handler for telemetry
    log::info!("[MAIN] Installing panic handler...");
    earth_engine::panic_handler::install_panic_handler();
    log::info!("[MAIN] Panic handler installed");
    
    // Create engine with default config
    log::info!("[MAIN] Creating engine config...");
    let config = EngineConfig {
        window_title: "Earth Engine".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 8,
    };
    log::info!("[MAIN] Engine config created: {:?}", config);
    
    log::info!("[MAIN] Creating Engine instance...");
    let engine = Engine::new(config);
    log::info!("[MAIN] Engine instance created");
    
    log::info!("[MAIN] Creating game instance...");
    let game = EarthGame::new();
    log::info!("[MAIN] Game instance created");
    
    // Run the game
    log::info!("[MAIN] Starting game loop...");
    match engine.run(game) {
        Ok(_) => {
            log::info!("[MAIN] Game loop exited normally");
        }
        Err(e) => {
            log::error!("[MAIN] Engine error: {}", e);
            eprintln!("[MAIN] Fatal error: {}", e);
            std::process::exit(1);
        }
    }
    
    log::info!("[MAIN] Application exiting normally");
}