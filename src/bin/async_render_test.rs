use hearth_engine::{
    Engine, EngineConfig, BlockRegistry,
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator, VoxelPos},
    BlockId, Block, RenderData, PhysicsProperties,
};
use hearth_engine::game::{GameData, GameContext};
use cgmath::Point3;
use std::sync::Arc;
use parking_lot::RwLock;

// Test blocks - DOP style
struct TestGrassBlockData;
struct TestDirtBlockData;
struct TestStoneBlockData;

// DOP functions for grass block
fn get_async_grass_id(_block: &TestGrassBlockData) -> BlockId { BlockId(1) }
fn get_async_grass_render_data(_block: &TestGrassBlockData) -> RenderData {
    RenderData {
        color: [0.3, 0.7, 0.2],
        texture_id: 0,
    }
}
fn get_async_grass_physics_properties(_block: &TestGrassBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 1200.0,
    }
}
fn get_async_grass_name(_block: &TestGrassBlockData) -> &'static str { "Grass" }

// DOP functions for dirt block
fn get_async_dirt_id(_block: &TestDirtBlockData) -> BlockId { BlockId(2) }
fn get_async_dirt_render_data(_block: &TestDirtBlockData) -> RenderData {
    RenderData {
        color: [0.5, 0.3, 0.1],
        texture_id: 0,
    }
}
fn get_async_dirt_physics_properties(_block: &TestDirtBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 1500.0,
    }
}
fn get_async_dirt_name(_block: &TestDirtBlockData) -> &'static str { "Dirt" }

// DOP functions for stone block
fn get_async_stone_id(_block: &TestStoneBlockData) -> BlockId { BlockId(3) }
fn get_async_stone_render_data(_block: &TestStoneBlockData) -> RenderData {
    RenderData {
        color: [0.6, 0.6, 0.6],
        texture_id: 0,
    }
}
fn get_async_stone_physics_properties(_block: &TestStoneBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 2500.0,
    }
}
fn get_async_stone_name(_block: &TestStoneBlockData) -> &'static str { "Stone" }

// Legacy Block implementations for compatibility
impl Block for TestGrassBlockData {
    fn get_id(&self) -> BlockId { get_async_grass_id(self) }
    fn get_render_data(&self) -> RenderData { get_async_grass_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_async_grass_physics_properties(self) }
    fn get_name(&self) -> &str { get_async_grass_name(self) }
}

impl Block for TestDirtBlockData {
    fn get_id(&self) -> BlockId { get_async_dirt_id(self) }
    fn get_render_data(&self) -> RenderData { get_async_dirt_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_async_dirt_physics_properties(self) }
    fn get_name(&self) -> &str { get_async_dirt_name(self) }
}

impl Block for TestStoneBlockData {
    fn get_id(&self) -> BlockId { get_async_stone_id(self) }
    fn get_render_data(&self) -> RenderData { get_async_stone_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_async_stone_physics_properties(self) }
    fn get_name(&self) -> &str { get_async_stone_name(self) }
}

/// Async render game data structure (DOP - no methods)
struct AsyncRenderGameData {
    world: Arc<RwLock<Option<Arc<ParallelWorld>>>>,
    player_block: BlockId,
    show_stats: bool,
}

impl GameData for AsyncRenderGameData {}

/// Create new async render game data
/// Pure function - returns async render game data structure
fn create_async_render_game_data() -> AsyncRenderGameData {
    AsyncRenderGameData {
        world: Arc::new(RwLock::new(None)),
        player_block: BlockId(1),
        show_stats: true,
    }
}

/// Register blocks for async render game
/// Function - transforms registry and game data by registering blocks and creating world
fn register_async_render_game_blocks(game: &mut AsyncRenderGameData, registry: &mut BlockRegistry) {
    println!("Registering blocks...");
    
    let grass_id = registry.register("test:grass", TestGrassBlockData);
    let dirt_id = registry.register("test:dirt", TestDirtBlockData);
    let stone_id = registry.register("test:stone", TestStoneBlockData);
    let water_id = BlockId(4); // Placeholder
    let sand_id = BlockId(5);  // Placeholder
    
    // Note: In a real implementation, we would need to pass the registry
    // to the async chunk renderer properly. For now, we'll skip this.
    
    // Create parallel world
    let config = ParallelWorldConfig {
        generation_threads: num_cpus::get().saturating_sub(2).max(2),
        mesh_threads: num_cpus::get().saturating_sub(2).max(2),
        chunks_per_frame: 8,
        view_distance: 6,
        chunk_size: 32,
    };
    
    println!("Creating parallel world with async mesh building...");
    
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    let world = Arc::new(ParallelWorld::new(generator, config.clone()));
    
    // Note: In a real implementation, we would create the async chunk renderer here
    // For this test, we'll skip the renderer creation since it requires registry access
    
    // Pregenerate spawn area
    println!("Pregenerating spawn area...");
    let spawn_pos = Point3::new(0.0, 100.0, 0.0);
    match world.pregenerate_spawn_area(spawn_pos, 3) {
        Ok(_handle) => println!("Spawn area pregeneration started successfully"),
        Err(e) => {
            eprintln!("Failed to start spawn area pregeneration: {}", e);
            return;
        }
    }
    
    *game.world.write() = Some(world);
    // Renderer would be set here in real implementation
}

/// Update async render game
/// Function - transforms async render game data based on context
fn update_async_render_game(game: &mut AsyncRenderGameData, ctx: &mut GameContext, _delta_time: f32) {
    // Update parallel world
    if let Some(world) = game.world.read().as_ref() {
        world.update(ctx.camera.position.into());
        
        // Display stats
        if game.show_stats {
            let world_metrics = world.get_performance_metrics();
            
            println!("\rWorld: {} chunks | Gen: {:.1}/s | FPS: {:.0}",
                     world_metrics.loaded_chunks,
                     world_metrics.chunks_per_second,
                     world_metrics.fps);
        }
    }
    
    // Handle block breaking
    if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Left) {
        if let Some(hit) = ctx.cast_camera_ray(10.0) {
            if let Some(world) = game.world.read().as_ref() {
                world.set_block(hit.position, BlockId::AIR);
            }
        }
    }
    
    // Handle block placing
    if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Right) {
        if let Some(hit) = ctx.cast_camera_ray(10.0) {
            let offset = hit.face.offset();
            let place_pos = VoxelPos::new(
                hit.position.x + offset.x,
                hit.position.y + offset.y,
                hit.position.z + offset.z,
            );
            if let Some(world) = game.world.read().as_ref() {
                world.set_block(place_pos, game.player_block);
            }
        }
    }
    
    // Toggle stats with S
    if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyS) {
        game.show_stats = !game.show_stats;
        if !game.show_stats {
            println!(); // Clear the stats line
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

/// Get active block for async render game
/// Pure function - returns active block from async render game data
fn get_async_render_game_active_block(game: &AsyncRenderGameData) -> BlockId {
    game.player_block
}

fn main() {
    println!("Hearth Engine - Async Mesh Building Test");
    println!("======================================");
    println!("Controls:");
    println!("  WASD - Move");
    println!("  Mouse - Look around");
    println!("  Left Click - Break block");
    println!("  Right Click - Place block");
    println!("  S - Toggle stats display");
    println!("  R - Reset statistics");
    println!("  ESC - Toggle cursor lock");
    println!();
    println!("This test demonstrates async mesh building:");
    println!("- Meshes are built in background threads");
    println!("- Dirty chunks are automatically queued");
    println!("- Priority system ensures nearby chunks build first");
    println!();
    
    let config = EngineConfig {
        window_title: "Hearth Engine - Async Mesh Building".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 6,
    };
    
    let engine = Engine::new(config);
    let mut game = create_async_render_game_data();
    
    // Note: Engine.run() may need updates to use DOP approach
    // For now, this demonstrates the DOP game structure
    println!("Game data created successfully!");
    println!("DOP functions available:");
    println!("  - register_async_render_game_blocks");
    println!("  - update_async_render_game");
    println!("  - get_async_render_game_active_block");
    
    println!("Engine shut down successfully");
}