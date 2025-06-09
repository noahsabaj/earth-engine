/// Main Earth Engine game executable
/// This is the primary entry point for the full game experience
/// Features: block placement/breaking, multiple block types, full input handling

use earth_engine::{Engine, EngineConfig, Game, GameContext};
use earth_engine::world::{BlockId, BlockRegistry, VoxelPos};
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
    fn register_blocks(&mut self, registry: &mut BlockRegistry) {
        // Blocks are already registered in the world module
        log::info!("Blocks registered");
    }
    
    fn update(&mut self, ctx: &mut GameContext, _delta_time: f32) {
        // Handle block breaking with left click
        if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Left) {
            if let Some(hit) = ctx.cast_camera_ray(10.0) {
                ctx.break_block(hit.position);
                log::info!("Broke block at {:?}", hit.position);
            }
        }
        
        // Handle block placing with right click
        if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Right) {
            if let Some(hit) = ctx.cast_camera_ray(10.0) {
                // Place block on the face we hit
                let place_pos = hit.position + hit.face.normal();
                ctx.place_block(place_pos, self.player_block);
                log::info!("Placed block at {:?}", place_pos);
            }
        }
        
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
    println!("Starting Earth Engine...");
    
    // Create engine with default config
    let config = EngineConfig {
        window_title: "Earth Engine".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 8,
    };
    
    let engine = Engine::new(config);
    let game = EarthGame::new();
    
    // Run the game
    if let Err(e) = engine.run(game) {
        log::error!("Engine error: {}", e);
    }
}