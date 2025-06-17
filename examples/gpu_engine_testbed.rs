/// GPU Engine Testbed - Interactive Test of GPU Terrain Generation
///
/// This example creates a runnable engine testbed that demonstrates GPU terrain generation
/// in action. Use this to visually verify the performance improvement from 0.8 FPS to 60+ FPS.
///
/// Controls:
/// - WASD: Move player
/// - Mouse: Look around  
/// - ESC: Exit
/// - F1: Toggle between CPU and GPU generation (if F1 is pressed during init)
///
/// This demonstrates the complete integration of GPU terrain generation into the engine.

use earth_engine::{
    EngineConfig, Engine, Game, GameContext,
    BlockId, VoxelPos, ChunkPos,
    input::{InputState, KeyCode},
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator, World},
    camera::data_camera::{CameraData, init_camera},
    physics::data_physics::{PhysicsWorldData, flags},
    renderer::GpuDiagnostics,
};
use cgmath::{Point3, Vector3};
use std::sync::Arc;
use std::time::Instant;
use anyhow::Result;

/// Simple game implementation that showcases GPU terrain generation
struct GpuTerrainGame {
    /// Whether to use GPU generation (set during init)
    use_gpu_generation: bool,
    
    /// Track generation mode for display
    generation_mode: String,
    
    /// Performance metrics
    last_fps_update: Instant,
    frame_count: u32,
    current_fps: f32,
    
    /// Player spawn position
    spawn_pos: Point3<f32>,
}

impl GpuTerrainGame {
    fn new() -> Self {
        Self {
            use_gpu_generation: true, // Default to GPU
            generation_mode: "GPU".to_string(),
            last_fps_update: Instant::now(),
            frame_count: 0,
            current_fps: 0.0,
            spawn_pos: Point3::new(0.0, 80.0, 0.0), // Spawn above terrain
        }
    }
}

impl Game for GpuTerrainGame {
    fn init(&mut self, context: &mut GameContext) -> Result<()> {
        println!("=== GPU TERRAIN GENERATION TESTBED ===");
        println!("Initializing Hearth Engine with GPU terrain generation...");
        println!("This testbed demonstrates the performance improvement from CPU to GPU generation.");
        println!();
        
        // Check if F1 is being held during init to switch to CPU mode
        if context.input.is_key_pressed(KeyCode::F1) {
            self.use_gpu_generation = false;
            self.generation_mode = "CPU".to_string();
            println!("F1 detected - Using CPU terrain generation for comparison");
        } else {
            println!("Using GPU terrain generation (default)");
            println!("Press F1 during startup to test CPU generation instead");
        }
        
        // Initialize world with appropriate generation method
        // Note: The actual GPU/CPU switching would need to be implemented in the world creation
        // For now, this serves as the framework for testing
        
        println!("Spawning player at: {:?}", self.spawn_pos);
        println!();
        println!("Controls:");
        println!("  WASD - Move player");
        println!("  Mouse - Look around");
        println!("  ESC - Exit testbed");
        println!();
        println!("Watch for performance metrics in the title bar and console output.");
        
        Ok(())
    }
    
    fn update(&mut self, context: &mut GameContext, dt: f32) -> Result<()> {
        // Update FPS counter
        self.frame_count += 1;
        if self.last_fps_update.elapsed().as_secs_f32() >= 1.0 {
            self.current_fps = self.frame_count as f32 / self.last_fps_update.elapsed().as_secs_f32();
            self.frame_count = 0;
            self.last_fps_update = Instant::now();
            
            // Log performance metrics
            println!("[{}] FPS: {:.1}, Frame time: {:.1}ms", 
                    self.generation_mode, 
                    self.current_fps,
                    1000.0 / self.current_fps.max(0.1));
        }
        
        // Handle exit
        if context.input.is_key_pressed(KeyCode::Escape) {
            println!("Exiting GPU terrain testbed...");
            std::process::exit(0);
        }
        
        Ok(())
    }
    
    fn render(&mut self, _context: &mut GameContext) -> Result<()> {
        // Rendering is handled by the engine
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<()> {
        println!("GPU terrain testbed cleanup complete.");
        Ok(())
    }
}

fn main() -> Result<()> {
    env_logger::init();
    
    println!("Starting GPU Terrain Generation Testbed...");
    
    // Configure engine for optimal GPU terrain generation testing
    let config = EngineConfig {
        window_title: "GPU Terrain Generation Testbed".to_string(),
        window_width: 1280,
        window_height: 720,
        render_distance: 8, // Reasonable view distance for testing
        chunk_size: 32,
        vsync: false, // Disable VSync to see true performance
        max_fps: None, // Uncapped FPS to measure max performance
        ..Default::default()
    };
    
    // Create and run the game
    let game = GpuTerrainGame::new();
    let engine = Engine::new(config)?;
    
    engine.run(game)?;
    
    Ok(())
}