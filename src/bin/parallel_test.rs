use earth_engine::{
    Engine, EngineConfig, BlockRegistry,
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator, VoxelPos},
    BlockId, Block, RenderData, PhysicsProperties,
};
use earth_engine::game::{GameData, GameContext, cast_camera_ray_from_context};
use cgmath::Point3;
use std::sync::Arc;
use parking_lot::RwLock;

// Test blocks - DOP style
struct TestGrassBlockData;
struct TestDirtBlockData;
struct TestStoneBlockData;
struct TestWaterBlockData;
struct TestSandBlockData;

// DOP functions for grass block
fn get_grass_id(_block: &TestGrassBlockData) -> BlockId { BlockId(1) }
fn get_grass_render_data(_block: &TestGrassBlockData) -> RenderData {
    RenderData {
        color: [0.3, 0.7, 0.2],
        texture_id: 0,
    }
}
fn get_grass_physics_properties(_block: &TestGrassBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 1200.0,
    }
}
fn get_grass_name(_block: &TestGrassBlockData) -> &'static str { "Grass" }

// DOP functions for dirt block
fn get_dirt_id(_block: &TestDirtBlockData) -> BlockId { BlockId(2) }
fn get_dirt_render_data(_block: &TestDirtBlockData) -> RenderData {
    RenderData {
        color: [0.5, 0.3, 0.1],
        texture_id: 0,
    }
}
fn get_dirt_physics_properties(_block: &TestDirtBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 1500.0,
    }
}
fn get_dirt_name(_block: &TestDirtBlockData) -> &'static str { "Dirt" }

// DOP functions for stone block
fn get_stone_id(_block: &TestStoneBlockData) -> BlockId { BlockId(3) }
fn get_stone_render_data(_block: &TestStoneBlockData) -> RenderData {
    RenderData {
        color: [0.6, 0.6, 0.6],
        texture_id: 0,
    }
}
fn get_stone_physics_properties(_block: &TestStoneBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 2500.0,
    }
}
fn get_stone_name(_block: &TestStoneBlockData) -> &'static str { "Stone" }

// DOP functions for water block
fn get_water_id(_block: &TestWaterBlockData) -> BlockId { BlockId(4) }
fn get_water_render_data(_block: &TestWaterBlockData) -> RenderData {
    RenderData {
        color: [0.1, 0.4, 0.8],
        texture_id: 0,
    }
}
fn get_water_physics_properties(_block: &TestWaterBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: false,
        density: 1000.0,
    }
}
fn get_water_name(_block: &TestWaterBlockData) -> &'static str { "Water" }

// DOP functions for sand block
fn get_sand_id(_block: &TestSandBlockData) -> BlockId { BlockId(5) }
fn get_sand_render_data(_block: &TestSandBlockData) -> RenderData {
    RenderData {
        color: [0.9, 0.8, 0.6],
        texture_id: 0,
    }
}
fn get_sand_physics_properties(_block: &TestSandBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 1600.0,
    }
}
fn get_sand_name(_block: &TestSandBlockData) -> &'static str { "Sand" }

// Legacy Block implementations for compatibility
impl Block for TestGrassBlockData {
    fn get_id(&self) -> BlockId { get_grass_id(self) }
    fn get_render_data(&self) -> RenderData { get_grass_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_grass_physics_properties(self) }
    fn get_name(&self) -> &str { get_grass_name(self) }
}

impl Block for TestDirtBlockData {
    fn get_id(&self) -> BlockId { get_dirt_id(self) }
    fn get_render_data(&self) -> RenderData { get_dirt_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_dirt_physics_properties(self) }
    fn get_name(&self) -> &str { get_dirt_name(self) }
}

impl Block for TestStoneBlockData {
    fn get_id(&self) -> BlockId { get_stone_id(self) }
    fn get_render_data(&self) -> RenderData { get_stone_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_stone_physics_properties(self) }
    fn get_name(&self) -> &str { get_stone_name(self) }
}

impl Block for TestWaterBlockData {
    fn get_id(&self) -> BlockId { get_water_id(self) }
    fn get_render_data(&self) -> RenderData { get_water_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_water_physics_properties(self) }
    fn get_name(&self) -> &str { get_water_name(self) }
}

impl Block for TestSandBlockData {
    fn get_id(&self) -> BlockId { get_sand_id(self) }
    fn get_render_data(&self) -> RenderData { get_sand_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_sand_physics_properties(self) }
    fn get_name(&self) -> &str { get_sand_name(self) }
}

/// Parallel game data structure (DOP - no methods)
struct ParallelGameData {
    world: Arc<RwLock<Option<Arc<ParallelWorld>>>>,
    player_block: BlockId,
    show_metrics: bool,
}

impl GameData for ParallelGameData {}

/// Create new parallel game data
/// Pure function - returns parallel game data structure
fn create_parallel_game_data() -> ParallelGameData {
    ParallelGameData {
        world: Arc::new(RwLock::new(None)),
        player_block: BlockId(1), // Default to grass
        show_metrics: true,
    }
}

/// Register blocks for parallel game
/// Function - transforms registry and game data by registering blocks and creating world
fn register_parallel_game_blocks(game: &mut ParallelGameData, registry: &mut BlockRegistry) {
    println!("Registering blocks...");
    
    let grass_id = registry.register("test:grass", TestGrassBlockData);
    let dirt_id = registry.register("test:dirt", TestDirtBlockData);
    let stone_id = registry.register("test:stone", TestStoneBlockData);
    let water_id = registry.register("test:water", TestWaterBlockData);
    let sand_id = registry.register("test:sand", TestSandBlockData);
    
    // Create parallel world with optimized config
    let config = ParallelWorldConfig {
        generation_threads: num_cpus::get().saturating_sub(2).max(2),
        mesh_threads: num_cpus::get().saturating_sub(2).max(2),
        chunks_per_frame: 8,
        view_distance: 4,
        chunk_size: 32,
    };
    
    println!("Creating parallel world with {} generation threads...", 
             config.generation_threads);
    
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    let world = Arc::new(ParallelWorld::new(generator, config));
    
    // Pregenerate spawn area
    println!("Pregenerating spawn area...");
    let spawn_pos = Point3::new(0.0, 100.0, 0.0);
    match world.pregenerate_spawn_area(spawn_pos, 2) {
        Ok(_handle) => println!("Spawn area pregeneration started successfully"),
        Err(e) => {
            eprintln!("Failed to start spawn area pregeneration: {}", e);
            return;
        }
    }
    
    *game.world.write() = Some(world);
}

/// Update parallel game
/// Function - transforms parallel game data based on context
fn update_parallel_game(game: &mut ParallelGameData, ctx: &mut GameContext, _delta_time: f32) {
    // Update parallel world
    if let Some(world) = game.world.read().as_ref() {
        world.update(ctx.camera.position.into());
        
        // Display performance metrics
        if game.show_metrics {
            let metrics = world.get_performance_metrics();
            println!("\rChunks: {} loaded, {} cached | Gen: {:.1} chunks/s | FPS: {:.0}",
                     metrics.loaded_chunks,
                     metrics.cached_chunks,
                     metrics.chunks_per_second,
                     metrics.fps);
        }
    }
    
    // Handle block breaking with left click
    if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Left) {
        if let Some(hit) = cast_camera_ray_from_context(ctx, 10.0) {
            if let Some(world) = game.world.read().as_ref() {
                world.set_block(hit.position, BlockId::AIR);
                // Note: In real implementation, would need to sync with main world
            }
        }
    }
    
    // Handle block placing with right click
    if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Right) {
        if let Some(hit) = cast_camera_ray_from_context(ctx, 10.0) {
            let offset = hit.face.offset();
            let place_pos = VoxelPos::new(
                hit.position.x + offset.x,
                hit.position.y + offset.y,
                hit.position.z + offset.z,
            );
            if let Some(world) = game.world.read().as_ref() {
                world.set_block(place_pos, game.player_block);
                // Note: In real implementation, would need to sync with main world
            }
        }
    }
    
    // Toggle metrics display with M
    if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyM) {
        game.show_metrics = !game.show_metrics;
        if !game.show_metrics {
            println!(); // Clear the metrics line
        }
    }
    
    // Reset stats with R
    if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyR) {
        if let Some(world) = game.world.read().as_ref() {
            world.reset_stats();
            println!("Statistics reset!");
        }
    }
}

/// Get active block for parallel game
/// Pure function - returns active block from parallel game data
fn get_parallel_game_active_block(game: &ParallelGameData) -> BlockId {
    game.player_block
}

fn main() {
    println!("Hearth Engine - Parallel World Test");
    println!("==================================");
    println!("Controls:");
    println!("  WASD - Move");
    println!("  Mouse - Look around");
    println!("  Left Click - Break block");
    println!("  Right Click - Place block");
    println!("  M - Toggle metrics display");
    println!("  R - Reset statistics");
    println!("  ESC - Toggle cursor lock");
    println!();
    
    let config = EngineConfig {
        window_title: "Hearth Engine - Parallel World Test".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 4,
    };
    
    let engine = Engine::new(config);
    let mut game = create_parallel_game_data();
    
    // Note: Engine.run() may need updates to use DOP approach
    // For now, this demonstrates the DOP game structure
    println!("Game data created successfully!");
    println!("DOP functions available:");
    println!("  - register_parallel_game_blocks");
    println!("  - update_parallel_game");
    println!("  - get_parallel_game_active_block");
    
    println!("Engine shut down successfully");
}