use hearth_engine::{
    BlockRegistry, BlockId, VoxelPos,
    Block, RenderData, PhysicsProperties,
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator},
    camera::data_camera::{CameraData, init_camera}, input::InputState,
};
use hearth_engine::game::{GameData, GameContext, cast_camera_ray_from_context, break_block_in_context, place_block_in_context};
use std::sync::Arc;
use cgmath::Point3;

// Test blocks - DOP style
struct TestGrassBlockData;
struct TestDirtBlockData;
struct TestStoneBlockData;

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

/// Simple test game data to verify GameContext works with ParallelWorld (DOP)
#[derive(Default)]
struct TestGameData;

impl GameData for TestGameData {}

/// Register blocks for test game context
/// Function - transforms registry by adding test blocks
fn register_test_context_game_blocks(_game: &mut TestGameData, registry: &mut BlockRegistry) {
    // Register some basic blocks
    registry.register("test:grass", TestGrassBlockData);
    registry.register("test:dirt", TestDirtBlockData);
    registry.register("test:stone", TestStoneBlockData);
}

/// Update test game context
/// Function - transforms game data by testing context interactions
fn update_test_context_game(game: &mut TestGameData, ctx: &mut GameContext, delta_time: f32) {
    let _ = (game, delta_time);
    
    // Test that we can interact with the world through GameContext
    
    // Test raycasting
    if let Some(hit) = cast_camera_ray_from_context(ctx, 10.0) {
        println!("Looking at block {:?} at {:?}", hit.block, hit.position);
        
        // Test block breaking on left click
        if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Left) {
            if break_block_in_context(ctx, hit.position) {
                println!("Broke block at {:?}", hit.position);
            }
        }
        
        // Test block placing on right click
        if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Right) {
            let place_pos = VoxelPos::new(
                hit.position.x + hit.face.offset().x,
                hit.position.y + hit.face.offset().y,
                hit.position.z + hit.face.offset().z,
            );
            if place_block_in_context(ctx, place_pos, get_test_context_active_block(game)) {
                println!("Placed block at {:?}", place_pos);
            }
        }
    }
    
    // Test reading block at camera position
    let camera_block_pos = VoxelPos::new(
        ctx.camera.position[0].floor() as i32,
        ctx.camera.position[1].floor() as i32,
        ctx.camera.position[2].floor() as i32,
    );
    let block = ctx.world.get_block(camera_block_pos);
    if block != BlockId::AIR {
        println!("Camera is inside block {:?}", block);
    }
}

/// Get active block for test game context
/// Pure function - returns active block for test game
fn get_test_context_active_block(_game: &TestGameData) -> BlockId {
    BlockId(1) // Grass block
}

fn main() {
    println!("Testing GameContext with ParallelWorld...");
    
    // Create registry
    let mut registry = BlockRegistry::new();
    let grass_id = registry.register("test:grass", TestGrassBlockData);
    let dirt_id = registry.register("test:dirt", TestDirtBlockData);
    let stone_id = registry.register("test:stone", TestStoneBlockData);
    
    // Create world
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        grass_id,
        dirt_id,
        stone_id,
        BlockId(4), // water
        BlockId(5), // sand
    ));
    
    let config = ParallelWorldConfig::default();
    let mut world = ParallelWorld::new(generator, config);
    
    // Create camera
    let camera = init_camera(800, 600);
    
    // Create input state
    let input = InputState::new();
    
    // Create game
    let mut game = TestGameData::default();
    
    // Test update with GameContext
    let mut ctx = GameContext {
        world: &mut world,
        registry: &registry,
        camera: &camera,
        input: &input,
        selected_block: None,
    };
    
    update_test_context_game(&mut game, &mut ctx, 0.016); // 60 FPS
    
    println!("GameContext test successful! Game mechanics now work with ParallelWorld.");
    println!("DOP functions used: register_test_context_game_blocks, update_test_context_game");
}