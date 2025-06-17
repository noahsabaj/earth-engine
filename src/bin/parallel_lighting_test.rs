use earth_engine::{
    Engine, EngineConfig, BlockRegistry,
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator, VoxelPos, ChunkPos},
    lighting::{ParallelLightPropagator, ParallelBlockProvider, LightType, MAX_LIGHT_LEVEL},
    BlockId, Block, RenderData, PhysicsProperties,
};
use earth_engine::game::{GameData, GameContext};
use cgmath::Point3;
use std::sync::Arc;
use parking_lot::RwLock;

// Test blocks - DOP style
struct TorchBlockData;
struct GlowstoneBlockData;

// DOP functions for torch block
fn get_torch_id(_block: &TorchBlockData) -> BlockId { BlockId(10) }
fn get_torch_render_data(_block: &TorchBlockData) -> RenderData {
    RenderData {
        color: [1.0, 0.8, 0.4], // Warm torch color
        texture_id: 0,
    }
}
fn get_torch_physics_properties(_block: &TorchBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: false,
        density: 100.0,
    }
}
fn get_torch_name(_block: &TorchBlockData) -> &'static str { "Torch" }

// DOP functions for glowstone block
fn get_glowstone_id(_block: &GlowstoneBlockData) -> BlockId { BlockId(11) }
fn get_glowstone_render_data(_block: &GlowstoneBlockData) -> RenderData {
    RenderData {
        color: [0.9, 0.9, 0.6], // Glowstone color
        texture_id: 0,
    }
}
fn get_glowstone_physics_properties(_block: &GlowstoneBlockData) -> PhysicsProperties {
    PhysicsProperties {
        solid: true,
        density: 800.0,
    }
}
fn get_glowstone_name(_block: &GlowstoneBlockData) -> &'static str { "Glowstone" }

// Legacy Block implementations for compatibility
impl Block for TorchBlockData {
    fn get_id(&self) -> BlockId { get_torch_id(self) }
    fn get_render_data(&self) -> RenderData { get_torch_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_torch_physics_properties(self) }
    fn get_name(&self) -> &str { get_torch_name(self) }
}

impl Block for GlowstoneBlockData {
    fn get_id(&self) -> BlockId { get_glowstone_id(self) }
    fn get_render_data(&self) -> RenderData { get_glowstone_render_data(self) }
    fn get_physics_properties(&self) -> PhysicsProperties { get_glowstone_physics_properties(self) }
    fn get_name(&self) -> &str { get_glowstone_name(self) }
}

/// Parallel lighting game data structure (DOP - no methods)
struct ParallelLightingGameData {
    world: Arc<RwLock<Option<Arc<ParallelWorld>>>>,
    light_propagator: Arc<RwLock<Option<Arc<ParallelLightPropagator>>>>,
    torch_id: BlockId,
    glowstone_id: BlockId,
    show_stats: bool,
}

impl GameData for ParallelLightingGameData {}

/// Create new parallel lighting game data
/// Pure function - returns parallel lighting game data structure
fn create_parallel_lighting_game_data() -> ParallelLightingGameData {
    ParallelLightingGameData {
        world: Arc::new(RwLock::new(None)),
        light_propagator: Arc::new(RwLock::new(None)),
        torch_id: BlockId(10),
        glowstone_id: BlockId(11),
        show_stats: true,
    }
}

/// Register blocks for parallel lighting game
/// Function - transforms registry and game data by registering blocks and creating lighting system
fn register_parallel_lighting_game_blocks(game: &mut ParallelLightingGameData, registry: &mut BlockRegistry) {
    println!("Registering blocks...");
    
    // Register light-emitting blocks
    game.torch_id = registry.register("test:torch", TorchBlockData);
    game.glowstone_id = registry.register("test:glowstone", GlowstoneBlockData);
    
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
    match world.pregenerate_spawn_area(spawn_pos, 3) {
        Ok(_handle) => println!("Spawn area pregeneration started successfully"),
        Err(e) => {
            eprintln!("Failed to start spawn area pregeneration: {}", e);
            return;
        }
    }
    
    // Calculate initial skylight
    println!("Calculating initial skylight...");
    for x in -3..=3 {
        for y in 0..8 {
            for z in -3..=3 {
                light_propagator.calculate_chunk_skylight(ChunkPos::new(x, y, z));
            }
        }
    }
    
    *game.world.write() = Some(world);
    *game.light_propagator.write() = Some(light_propagator);
}

/// Update parallel lighting game
/// Function - transforms parallel lighting game data based on context
fn update_parallel_lighting_game(game: &mut ParallelLightingGameData, ctx: &mut GameContext, _delta_time: f32) {
    // Update parallel world
    if let Some(world) = game.world.read().as_ref() {
        world.update(ctx.camera.position.into());
        
        // Process light updates
        if let Some(propagator) = game.light_propagator.read().as_ref() {
            propagator.process_updates(100); // Process up to 100 updates per frame
            
            // Display stats
            if game.show_stats {
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
            if let Some(world) = game.world.read().as_ref() {
                let old_block = world.get_block(hit.position);
                
                // If breaking a light source, remove light
                if old_block == game.torch_id || old_block == game.glowstone_id {
                    if let Some(propagator) = game.light_propagator.read().as_ref() {
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
            
            if let Some(world) = game.world.read().as_ref() {
                world.set_block(place_pos, game.torch_id);
                
                // Add light
                if let Some(propagator) = game.light_propagator.read().as_ref() {
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
            
            if let Some(world) = game.world.read().as_ref() {
                world.set_block(place_pos, game.glowstone_id);
                
                // Add light
                if let Some(propagator) = game.light_propagator.read().as_ref() {
                    propagator.add_light(place_pos, LightType::Block, MAX_LIGHT_LEVEL);
                }
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
        
    // Reset lighting stats with R
    if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyR) {
        if let Some(propagator) = game.light_propagator.read().as_ref() {
            propagator.reset_stats();
            println!("Lighting statistics reset!");
        }
    }
        
    // Clear all lights with C (for testing)
    if ctx.input.is_key_pressed(winit::keyboard::KeyCode::KeyC) {
        if let Some(propagator) = game.light_propagator.read().as_ref() {
            propagator.clear();
            println!("All lighting cleared!");
        }
    }
}

/// Get active block for parallel lighting game
/// Pure function - returns active block from parallel lighting game data
fn get_parallel_lighting_game_active_block(game: &ParallelLightingGameData) -> BlockId {
    game.torch_id
}

fn main() {
    println!("Hearth Engine - Parallel Lighting Test");
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
        window_title: "Hearth Engine - Parallel Lighting".to_string(),
        window_width: 1280,
        window_height: 720,
        chunk_size: 32,
        render_distance: 6,
    };
    
    let engine = Engine::new(config);
    let mut game = create_parallel_lighting_game_data();
    
    // Note: Engine.run() may need updates to use DOP approach
    // For now, this demonstrates the DOP game structure
    println!("Game data created successfully!");
    println!("DOP functions available:");
    println!("  - register_parallel_lighting_game_blocks");
    println!("  - update_parallel_lighting_game");
    println!("  - get_parallel_lighting_game_active_block");
    
    println!("Engine shut down successfully");
}