use earth_engine::{
    Game, GameContext, BlockRegistry, BlockId, VoxelPos,
    Block, RenderData, PhysicsProperties,
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator},
    camera::data_camera::{CameraData, init_camera}, input::InputState,
};
use std::sync::Arc;
use cgmath::Point3;

// Test blocks
struct TestGrassBlock;
impl Block for TestGrassBlock {
    fn get_id(&self) -> BlockId { BlockId(1) }
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
    fn get_id(&self) -> BlockId { BlockId(2) }
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
    fn get_id(&self) -> BlockId { BlockId(3) }
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

/// Simple test game to verify GameContext works with ParallelWorld
struct TestGame;

impl Game for TestGame {
    fn register_blocks(&mut self, registry: &mut BlockRegistry) {
        // Register some basic blocks
        registry.register("test:grass", TestGrassBlock);
        registry.register("test:dirt", TestDirtBlock);
        registry.register("test:stone", TestStoneBlock);
    }
    
    fn update(&mut self, ctx: &mut GameContext, delta_time: f32) {
        // Test that we can interact with the world through GameContext
        
        // Test raycasting
        if let Some(hit) = ctx.cast_camera_ray(10.0) {
            println!("Looking at block {:?} at {:?}", hit.block, hit.position);
            
            // Test block breaking on left click
            if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Left) {
                if ctx.break_block(hit.position) {
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
                if ctx.place_block(place_pos, self.get_active_block()) {
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
    
    fn get_active_block(&self) -> BlockId {
        BlockId(1) // Grass block
    }
}

fn main() {
    println!("Testing GameContext with ParallelWorld...");
    
    // Create registry
    let mut registry = BlockRegistry::new();
    let grass_id = registry.register("test:grass", TestGrassBlock);
    let dirt_id = registry.register("test:dirt", TestDirtBlock);
    let stone_id = registry.register("test:stone", TestStoneBlock);
    
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
    let mut game = TestGame;
    
    // Test update with GameContext
    let mut ctx = GameContext {
        world: &mut world,
        registry: &registry,
        camera: &camera,
        input: &input,
        selected_block: None,
    };
    
    game.update(&mut ctx, 0.016); // 60 FPS
    
    println!("GameContext test successful! Game mechanics now work with ParallelWorld.");
}