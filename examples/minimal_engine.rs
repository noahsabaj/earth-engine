/// Minimal Engine Example
/// 
/// This example demonstrates the simplest possible usage of Earth Engine as a library.
/// Perfect for:
/// - Learning the basic API
/// - Quick prototyping
/// - Understanding engine initialization
/// - Testing basic functionality
/// 
/// For comprehensive testing and debugging, see examples/engine_testbed.rs

use earth_engine::{Engine, EngineConfig, Game, GameContext};
use earth_engine::world::{BlockId, BlockRegistry};

/// Minimal game implementation demonstrating basic engine usage
struct MinimalGame {
    selected_block: BlockId,
}

impl MinimalGame {
    fn new() -> Self {
        Self {
            selected_block: BlockId(1), // Start with stone
        }
    }
}

impl Game for MinimalGame {
    fn register_blocks(&mut self, _registry: &mut BlockRegistry) {
        // Blocks are automatically registered by the engine
        println!("Blocks registered successfully");
    }
    
    fn update(&mut self, ctx: &mut GameContext, _delta_time: f32) {
        // Simple block switching with number keys
        if ctx.input.is_key_pressed(earth_engine::input::KeyCode::Digit1) {
            self.selected_block = BlockId(1); // Stone
            println!("Selected: Stone");
        } else if ctx.input.is_key_pressed(earth_engine::input::KeyCode::Digit2) {
            self.selected_block = BlockId(2); // Dirt
            println!("Selected: Dirt");
        } else if ctx.input.is_key_pressed(earth_engine::input::KeyCode::Digit3) {
            self.selected_block = BlockId(3); // Grass
            println!("Selected: Grass");
        }
        
        // Simple logging every few seconds
        static mut LAST_LOG: std::time::Instant = unsafe { std::mem::zeroed() };
        static mut INITIALIZED: bool = false;
        
        unsafe {
            if !INITIALIZED {
                LAST_LOG = std::time::Instant::now();
                INITIALIZED = true;
            }
            
            if LAST_LOG.elapsed().as_secs() >= 10 {
                println!("Engine running smoothly! Current block: {:?}", self.selected_block);
                LAST_LOG = std::time::Instant::now();
            }
        }
    }
    
    fn get_active_block(&self) -> BlockId {
        self.selected_block
    }
}

fn main() {
    println!("=== MINIMAL EARTH ENGINE EXAMPLE ===");
    println!("Demonstrating basic engine usage");
    println!();
    println!("Controls:");
    println!("  WASD - Move");
    println!("  Mouse - Look");
    println!("  1,2,3 - Switch blocks");
    println!("  LMB - Break blocks");
    println!("  RMB - Place blocks");
    println!("=====================================");
    
    // Initialize logging
    env_logger::init();
    
    // Create simple engine configuration
    let config = EngineConfig {
        window_title: "Minimal Earth Engine Example".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 6, // Smaller for better performance
    };
    
    // Create and run engine
    let engine = Engine::new(config);
    let game = MinimalGame::new();
    
    println!("Starting engine...");
    
    match engine.run(game) {
        Ok(_) => println!("Engine finished successfully!"),
        Err(e) => {
            eprintln!("Engine error: {}", e);
            std::process::exit(1);
        }
    }
}