use earth_engine::{
    EngineConfig, Game, GameContext, Camera, BlockId, VoxelPos,
    world::{RaycastHit, BlockFace}, run,
};
use winit::event_loop::EventLoop;

struct TestGame;

impl Game for TestGame {
    fn get_active_block(&self) -> BlockId {
        BlockId(1) // Grass block
    }
    
    fn update(&mut self, _ctx: &mut GameContext, _delta_time: f32) {
        // Log camera position periodically
        static mut COUNTER: u32 = 0;
        unsafe {
            COUNTER += 1;
            if COUNTER % 60 == 0 {
                log::info!("[TestGame] Frame {}", COUNTER);
            }
        }
    }
    
    fn on_block_break(&mut self, pos: VoxelPos, block_id: BlockId) {
        log::info!("[TestGame] Block broken at {:?}: {:?}", pos, block_id);
    }
    
    fn on_block_place(&mut self, pos: VoxelPos, block_id: BlockId) {
        log::info!("[TestGame] Block placed at {:?}: {:?}", pos, block_id);
    }
}

fn main() -> anyhow::Result<()> {
    // Initialize logger with debug level
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,earth_engine=debug"))
        .init();
    
    log::info!("[test_render] Starting render test...");
    
    let event_loop = EventLoop::new()?;
    let config = EngineConfig {
        window_title: "Earth Engine - Render Test".to_string(),
        window_width: 1280,
        window_height: 720,
    };
    
    let game = TestGame;
    
    log::info!("[test_render] Launching engine...");
    run(event_loop, config, game)
}