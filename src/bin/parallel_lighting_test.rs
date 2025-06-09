use earth_engine::{
    Engine, EngineConfig, Game, GameContext, BlockRegistry,
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator, VoxelPos, ChunkPos},
    lighting::{ParallelLightPropagator, ParallelBlockProvider, LightType, MAX_LIGHT_LEVEL},
    BlockId, Block, RenderData, PhysicsProperties,
};
use cgmath::Point3;
use std::sync::Arc;
use parking_lot::RwLock;

// Test blocks
struct TorchBlock;
impl Block for TorchBlock {
    fn get_id(&self) -> BlockId { BlockId(10) }
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [1.0, 0.8, 0.4], // Warm torch color
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: false,
            density: 100.0,
        }
    }
    fn get_name(&self) -> &str { "Torch" }
}

struct GlowstoneBlock;
impl Block for GlowstoneBlock {
    fn get_id(&self) -> BlockId { BlockId(11) }
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.9, 0.9, 0.6], // Glowstone color
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 800.0,
        }
    }
    fn get_name(&self) -> &str { "Glowstone" }
}

// Game with parallel lighting
struct ParallelLightingGame {
    world: Arc<RwLock<Option<Arc<ParallelWorld>>>>,
    light_propagator: Arc<RwLock<Option<Arc<ParallelLightPropagator>>>>,
    torch_id: BlockId,
    glowstone_id: BlockId,
    show_stats: bool,
}

impl ParallelLightingGame {
    fn new() -> Self {
        Self {
            world: Arc::new(RwLock::new(None)),
            light_propagator: Arc::new(RwLock::new(None)),
            torch_id: BlockId(10),
            glowstone_id: BlockId(11),
            show_stats: true,
        }
    }
}

impl Game for ParallelLightingGame {
    fn register_blocks(&mut self, registry: &mut BlockRegistry) {
        println!("Registering blocks...");
        
        // Register light-emitting blocks
        self.torch_id = registry.register("test:torch", TorchBlock);
        self.glowstone_id = registry.register("test:glowstone", GlowstoneBlock);
        
        // Create parallel world
        let config = ParallelWorldConfig {
            generation_threads: 4,
            mesh_threads: 4,
            chunks_per_frame: 8,
            view_distance: 6,
            chunk_size: 32,
        };
        
        println!("Creating parallel world with lighting system...");
        
        let generator = Box::new(DefaultWorldGenerator::new(
            12345,
            BlockId(1), // grass
            BlockId(2), // dirt
            BlockId(3), // stone
            BlockId(4), // water
            BlockId(5), // sand
        ));
        
        let world = Arc::new(ParallelWorld::new(generator, config.clone()));
        
        // Create parallel light propagator
        let block_provider = Arc::new(ParallelBlockProvider::new(
            world.chunk_manager_arc(),
        ));
        
        let light_propagator = Arc::new(ParallelLightPropagator::new(
            block_provider,
            config.chunk_size,
            None, // Auto-detect thread count
        ));
        
        // Pregenerate spawn area
        println!("Pregenerating spawn area...");
        let spawn_pos = Point3::new(0.0, 100.0, 0.0);
        world.pregenerate_spawn_area(spawn_pos, 3);
        
        // Calculate initial skylight
        println!("Calculating initial skylight...");
        for x in -3..=3 {
            for y in 0..8 {
                for z in -3..=3 {
                    light_propagator.calculate_chunk_skylight(ChunkPos::new(x, y, z));
                }
            }
        }
        
        *self.world.write() = Some(world);
        *self.light_propagator.write() = Some(light_propagator);
    }
    
    fn update(&mut self, ctx: &mut GameContext, _delta_time: f32) {
        // Update parallel world
        if let Some(world) = self.world.read().as_ref() {
            world.update(ctx.camera.position);
            
            // Process light updates
            if let Some(propagator) = self.light_propagator.read().as_ref() {
                propagator.process_updates(100); // Process up to 100 updates per frame
                
                // Display stats
                if self.show_stats {
                    let world_metrics = world.get_performance_metrics();
                    let light_stats = propagator.get_stats();
                    
                    println!("\rChunks: {} | Gen: {:.1}/s | Light updates: {} | Light chunks: {} | FPS: {:.0}",
                             world_metrics.loaded_chunks,
                             world_metrics.chunks_per_second,
                             light_stats.updates_processed,
                             light_stats.chunks_affected,
                             world_metrics.fps);
                }
            }
        }
        
        // Handle block breaking (removes light)
        if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Left) {
            if let Some(hit) = ctx.cast_camera_ray(10.0) {
                if let Some(world) = self.world.read().as_ref() {
                    let old_block = world.get_block(hit.position);
                    
                    // If breaking a light source, remove light
                    if old_block == self.torch_id || old_block == self.glowstone_id {
                        if let Some(propagator) = self.light_propagator.read().as_ref() {
                            propagator.remove_light(hit.position, LightType::Block);
                        }
                    }
                    
                    world.set_block(hit.position, BlockId::AIR);
                }
            }
        }
        
        // Handle torch placing (adds light)
        if ctx.input.is_mouse_button_pressed(winit::event::MouseButton::Right) {
            if let Some(hit) = ctx.cast_camera_ray(10.0) {
                let offset = hit.face.offset();
                let place_pos = VoxelPos::new(
                    hit.position.x + offset.x,
                    hit.position.y + offset.y,
                    hit.position.z + offset.z,
                );
                
                if let Some(world) = self.world.read().as_ref() {
                    world.set_block(place_pos, self.torch_id);
                    
                    // Add light
                    if let Some(propagator) = self.light_propagator.read().as_ref() {
                        propagator.add_light(place_pos, LightType::Block, MAX_LIGHT_LEVEL);
                    }
                }
            }
        }
        
        // Place glowstone with G key
        if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyG) {
            if let Some(hit) = ctx.cast_camera_ray(10.0) {
                let offset = hit.face.offset();
                let place_pos = VoxelPos::new(
                    hit.position.x + offset.x,
                    hit.position.y + offset.y,
                    hit.position.z + offset.z,
                );
                
                if let Some(world) = self.world.read().as_ref() {
                    world.set_block(place_pos, self.glowstone_id);
                    
                    // Add light
                    if let Some(propagator) = self.light_propagator.read().as_ref() {
                        propagator.add_light(place_pos, LightType::Block, MAX_LIGHT_LEVEL);
                    }
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
        
        // Reset lighting stats with R
        if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyR) {
            if let Some(propagator) = self.light_propagator.read().as_ref() {
                propagator.reset_stats();
                println!("Lighting statistics reset!");
            }
        }
        
        // Clear all lights with C (for testing)
        if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyC) {
            if let Some(propagator) = self.light_propagator.read().as_ref() {
                propagator.clear();
                println!("All lighting cleared!");
            }
        }
    }
    
    fn get_active_block(&self) -> BlockId {
        self.torch_id
    }
}

fn main() {
    println!("Earth Engine - Parallel Lighting Test");
    println!("====================================");
    println!("Controls:");
    println!("  WASD - Move");
    println!("  Mouse - Look around");
    println!("  Left Click - Break block (removes light)");
    println!("  Right Click - Place torch (adds light)");
    println!("  G - Place glowstone (adds light)");
    println!("  S - Toggle stats display");
    println!("  R - Reset lighting statistics");
    println!("  C - Clear all lighting");
    println!("  ESC - Toggle cursor lock");
    println!();
    println!("This test demonstrates parallel lighting:");
    println!("- Light propagation across multiple threads");
    println!("- Cross-chunk light updates");
    println!("- Dynamic light addition/removal");
    println!();
    
    let config = EngineConfig {
        window_title: "Earth Engine - Parallel Lighting".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 6,
    };
    
    let engine = Engine::new(config);
    let game = ParallelLightingGame::new();
    
    match engine.run(game) {
        Ok(_) => println!("Engine shut down successfully"),
        Err(e) => eprintln!("Engine error: {}", e),
    }
}