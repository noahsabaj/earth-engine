/// Minimal Engine Test
/// Purpose: Test if the Engine framework initializes and runs without errors
/// Use this for debugging engine initialization issues
/// For full game functionality, use the main executable instead

use earth_engine::{Engine, EngineConfig, Game, GameContext};
use earth_engine::world::BlockRegistry;

/// Minimal game implementation for testing engine startup
struct MinimalGame;

impl Game for MinimalGame {
    fn register_blocks(&mut self, _registry: &mut BlockRegistry) {
        println!("Registering blocks...");
    }
    
    fn update(&mut self, _ctx: &mut GameContext, _delta_time: f32) {
        // Do nothing for now
    }
}

fn main() {
    println!("Starting minimal Earth Engine test...");
    
    // Create engine with minimal config
    let config = EngineConfig::default();
    let engine = Engine::new(config);
    let game = MinimalGame;
    
    println!("Running engine...");
    
    // Run the game
    match engine.run(game) {
        Ok(_) => println!("Engine ran successfully!"),
        Err(e) => eprintln!("Engine error: {}", e),
    }
}