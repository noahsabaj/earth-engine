use earth_engine::{
    Engine, EngineConfig, Game, GameContext, BlockId, VoxelPos,
    BlockRegistry, Block, RenderData, PhysicsProperties,
};

// Define some basic blocks for the test
struct TestGrassBlock;
impl Block for TestGrassBlock {
    fn get_id(&self) -> BlockId { BlockId::GRASS }
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.3, 0.7, 0.2],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1200.0,
        }
    }
    fn get_name(&self) -> &str { "Grass" }
}

struct TestDirtBlock;
impl Block for TestDirtBlock {
    fn get_id(&self) -> BlockId { BlockId::DIRT }
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.5, 0.3, 0.1],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1500.0,
        }
    }
    fn get_name(&self) -> &str { "Dirt" }
}

struct TestStoneBlock;
impl Block for TestStoneBlock {
    fn get_id(&self) -> BlockId { BlockId::STONE }
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.6, 0.6, 0.6],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 2500.0,
        }
    }
    fn get_name(&self) -> &str { "Stone" }
}

struct TestGame;

impl Game for TestGame {
    fn register_blocks(&mut self, registry: &mut BlockRegistry) {
        // Register basic blocks for testing
        registry.register("test:grass", TestGrassBlock);
        registry.register("test:dirt", TestDirtBlock);
        registry.register("test:stone", TestStoneBlock);
        
        log::info!("[TestGame] Registered {} blocks", 3);
    }
    
    fn get_active_block(&self) -> BlockId {
        BlockId(1) // Grass block
    }
    
    fn update(&mut self, _ctx: &mut GameContext, _delta_time: f32) {
        // Log camera position periodically
        static mut COUNTER: u32 = 0;
        // SAFETY: Accessing static mut COUNTER is safe because:
        // - This is a test binary running single-threaded
        // - Game::update is called sequentially from the main thread
        // - No concurrent access to the static variable occurs
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
    
    let config = EngineConfig {
        window_title: "Earth Engine - Render Test".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 8,
    };
    
    let engine = Engine::new(config);
    let game = TestGame;
    
    log::info!("[test_render] Launching engine...");
    engine.run(game)
}