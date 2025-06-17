use hearth_engine::{EngineConfig, renderer, Game, Block, BlockId, RenderData, BlockRegistry};
use std::sync::Arc;
use cgmath::{Point3, Vector3};

/// Test game to debug screenshot issues
struct ScreenshotDebugGame;

impl Game for ScreenshotDebugGame {
    fn register_blocks(&mut self, registry: &mut BlockRegistry) {
        // Blocks are registered in GpuState::new()
    }
    
    fn update(&mut self, _ctx: &mut hearth_engine::GameContext, _delta_time: f32) {
        // No updates needed for this test
    }
    
    fn get_active_block(&self) -> BlockId {
        BlockId(1) // Default to grass
    }
    
    fn on_block_break(&mut self, _pos: hearth_engine::VoxelPos, _block: BlockId) {}
    fn on_block_place(&mut self, _pos: hearth_engine::VoxelPos, _block: BlockId) {}
}

fn main() {
    env_logger::init();
    
    println!("=== Screenshot Debug Test ===");
    println!("This test will help identify why screenshots show empty scenes");
    println!();
    println!("Expected behavior:");
    println!("1. Game renders terrain normally on screen");
    println!("2. Press F6 to capture screenshot");
    println!("3. Screenshot should show the same terrain visible on screen");
    println!();
    println!("Debug information will be logged to help identify issues");
    println!();
    println!("Controls:");
    println!("- WASD: Move");
    println!("- Mouse: Look around");
    println!("- F5: Toggle automatic screenshot capture (every 2 seconds)");
    println!("- F6: Take single screenshot");
    println!("- ESC: Toggle cursor lock");
    println!();
    
    let config = EngineConfig {
        window_title: "Screenshot Debug Test".to_string(),
        window_width: 1280,
        window_height: 720,
        ..Default::default()
    };
    
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let game = ScreenshotDebugGame;
    
    if let Err(e) = renderer::run(event_loop, config, game) {
        eprintln!("Error running game: {}", e);
    }
}