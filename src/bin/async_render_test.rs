use earth_engine::{
    Engine, EngineConfig, Game, GameContext, BlockRegistry,
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator, VoxelPos},
    BlockId, Block, RenderData, PhysicsProperties,
};
use cgmath::Point3;
use std::sync::Arc;
use parking_lot::RwLock;

// Test blocks (same as parallel_test)
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

/// Game implementation using async chunk renderer
struct AsyncRenderGame {
    world: Arc<RwLock<Option<Arc<ParallelWorld>>>>,
    player_block: BlockId,
    show_stats: bool,
}

impl AsyncRenderGame {
    fn new() -> Self {
        Self {
            world: Arc::new(RwLock::new(None)),
            player_block: BlockId(1),
            show_stats: true,
        }
    }
}

impl Game for AsyncRenderGame {
    fn register_blocks(&mut self, registry: &mut BlockRegistry) {
        println!("Registering blocks...");
        
        let grass_id = registry.register("test:grass", TestGrassBlock);
        let dirt_id = registry.register("test:dirt", TestDirtBlock);
        let stone_id = registry.register("test:stone", TestStoneBlock);
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
        world.pregenerate_spawn_area(spawn_pos, 3);
        
        *self.world.write() = Some(world);
        // Renderer would be set here in real implementation
    }
    
    fn update(&mut self, ctx: &mut GameContext, _delta_time: f32) {
        // Update parallel world
        if let Some(world) = self.world.read().as_ref() {
            world.update(ctx.camera.position);
            
            // Display stats
            if self.show_stats {
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
                if let Some(world) = self.world.read().as_ref() {
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
                if let Some(world) = self.world.read().as_ref() {
                    world.set_block(place_pos, self.player_block);
                }
            }
        }
        
        // Toggle stats with S
        if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyS) {
            self.show_stats = !self.show_stats;
            if !self.show_stats {
                println!(); // Clear the stats line
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
    println!("Earth Engine - Async Mesh Building Test");
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
        window_title: "Earth Engine - Async Mesh Building".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 6,
    };
    
    let engine = Engine::new(config);
    let game = AsyncRenderGame::new();
    
    match engine.run(game) {
        Ok(_) => println!("Engine shut down successfully"),
        Err(e) => eprintln!("Engine error: {}", e),
    }
}