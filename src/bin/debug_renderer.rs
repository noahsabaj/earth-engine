use earth_engine::{
    Engine, EngineConfig, Game, GameContext, BlockId, VoxelPos,
};
use anyhow::Result;

struct DebugGame {
    active_block: BlockId,
}

impl DebugGame {
    fn new() -> Self {
        Self {
            active_block: BlockId(1), // Grass
        }
    }
}

impl Game for DebugGame {
    fn register_blocks(&mut self, _registry: &mut earth_engine::BlockRegistry) {
        // Blocks already registered in gpu_state
    }

    fn update(&mut self, _ctx: &mut GameContext, _delta_time: f32) {
        // No update needed for debug
    }

    fn get_active_block(&self) -> BlockId {
        self.active_block
    }

    fn on_block_break(&mut self, pos: VoxelPos, block: BlockId) {
        log::info!("Block broken at {:?}: {:?}", pos, block);
    }

    fn on_block_place(&mut self, pos: VoxelPos, block: BlockId) {
        log::info!("Block placed at {:?}: {:?}", pos, block);
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,earth_engine=debug"))
        .init();

    log::info!("Starting debug renderer...");

    let config = EngineConfig {
        window_title: "Earth Engine Debug Renderer".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 4,
    };

    let engine = Engine::new(config);
    let game = DebugGame::new();
    engine.run(game)?;

    Ok(())
}