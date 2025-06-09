use earth_engine::{
    Engine, EngineConfig, Game, GameContext, BlockRegistry,
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator, VoxelPos},
    BlockId, Block, RenderData, PhysicsProperties,
};
use cgmath::Point3;
use std::sync::Arc;
use parking_lot::RwLock;

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

struct TestWaterBlock;
impl Block for TestWaterBlock {
    fn get_id(&self) -> BlockId { BlockId(4) }
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.1, 0.4, 0.8],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: false,
            density: 1000.0,
        }
    }
    fn get_name(&self) -> &str { "Water" }
}

struct TestSandBlock;
impl Block for TestSandBlock {
    fn get_id(&self) -> BlockId { BlockId(5) }
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.9, 0.8, 0.6],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1600.0,
        }
    }
    fn get_name(&self) -> &str { "Sand" }
}

/// Game implementation using parallel world
struct ParallelGame {
    world: Arc<RwLock<Option<Arc<ParallelWorld>>>>,
    player_block: BlockId,
    show_metrics: bool,
}

impl ParallelGame {
    fn new() -> Self {
        Self {
            world: Arc::new(RwLock::new(None)),
            player_block: BlockId(1), // Default to grass
            show_metrics: true,
        }
    }
}

impl Game for ParallelGame {
    fn register_blocks(&mut self, registry: &mut BlockRegistry) {
        println!("Registering blocks...");
        
        let grass_id = registry.register("test:grass", TestGrassBlock);
        let dirt_id = registry.register("test:dirt", TestDirtBlock);
        let stone_id = registry.register("test:stone", TestStoneBlock);
        let water_id = registry.register("test:water", TestWaterBlock);
        let sand_id = registry.register("test:sand", TestSandBlock);
        
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
        world.pregenerate_spawn_area(spawn_pos, 2);
        
        *self.world.write() = Some(world);
    }
    
    fn update(&mut self, ctx: &mut GameContext, _delta_time: f32) {
        // Update parallel world
        if let Some(world) = self.world.read().as_ref() {
            world.update(ctx.camera.position);
            
            // Display performance metrics
            if self.show_metrics {
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
            if let Some(hit) = ctx.cast_camera_ray(10.0) {
                if let Some(world) = self.world.read().as_ref() {
                    world.set_block(hit.position, BlockId::AIR);
                    // Note: In real implementation, would need to sync with main world
                }
            }
        }
        
        // Handle block placing with right click
        if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Right) {
            if let Some(hit) = ctx.cast_camera_ray(10.0) {
                let offset = hit.face.offset();
                let place_pos = VoxelPos::new(
                    hit.position.x + offset.x,
                    hit.position.y + offset.y,
                    hit.position.z + offset.z,
                );
                if let Some(world) = self.world.read().as_ref() {
                    world.set_block(place_pos, self.player_block);
                    // Note: In real implementation, would need to sync with main world
                }
            }
        }
        
        // Toggle metrics display with M
        if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyM) {
            self.show_metrics = !self.show_metrics;
            if !self.show_metrics {
                println!(); // Clear the metrics line
            }
        }
        
        // Reset stats with R
        if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyR) {
            if let Some(world) = self.world.read().as_ref() {
                world.reset_stats();
                println!("Statistics reset!");
            }
        }
    }
    
    fn get_active_block(&self) -> BlockId {
        self.player_block
    }
}

fn main() {
    println!("Earth Engine - Parallel World Test");
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
        window_title: "Earth Engine - Parallel World Test".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 4,
    };
    
    let engine = Engine::new(config);
    let game = ParallelGame::new();
    
    match engine.run(game) {
        Ok(_) => println!("Engine shut down successfully"),
        Err(e) => eprintln!("Engine error: {}", e),
    }
}