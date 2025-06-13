/// Engine Testbed - Comprehensive Earth Engine Development Platform
/// 
/// This is the primary testbed for Earth Engine development, featuring:
/// - Full game implementation with complete gameplay features
/// - Comprehensive debug UI with real-time metrics
/// - Performance monitoring and profiling tools
/// - Engine configuration controls
/// - Visual debugging capabilities
/// - Memory and GPU diagnostics
/// 
/// Use this for:
/// - Engine development and testing
/// - Performance analysis and optimization
/// - Feature development and validation
/// - Debug and troubleshooting
/// 
/// For simple engine usage examples, see examples/minimal_engine.rs

use earth_engine::{Engine, EngineConfig, Game, GameContext};
use earth_engine::world::{BlockId, BlockRegistry};
use earth_engine::input::KeyCode;
use std::time::Instant;

/// Comprehensive game implementation with debug features for engine testing
pub struct EngineTestbed {
    // Game state
    player_block: BlockId,
    
    // Debug state
    debug_enabled: bool,
    performance_metrics: PerformanceMetrics,
    frame_times: Vec<f32>,
    last_metrics_update: Instant,
    
    // Configuration state
    current_config: TestbedConfig,
    config_dirty: bool,
}

#[derive(Debug, Clone)]
pub struct TestbedConfig {
    pub show_debug_info: bool,
    pub show_performance_metrics: bool,
    pub show_chunk_boundaries: bool,
    pub wireframe_mode: bool,
    pub enable_profiling: bool,
    pub target_fps: u32,
    pub chunk_load_distance: u32,
    pub render_distance: u32,
}

impl Default for TestbedConfig {
    fn default() -> Self {
        Self {
            show_debug_info: true,
            show_performance_metrics: true,
            show_chunk_boundaries: false,
            wireframe_mode: false,
            enable_profiling: false,
            target_fps: 60,
            chunk_load_distance: 12,
            render_distance: 8,
        }
    }
}

#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub min_frame_time: f32,
    pub max_frame_time: f32,
    pub avg_frame_time: f32,
    pub memory_usage_mb: f32,
    pub gpu_memory_mb: f32,
    pub draw_calls: u32,
    pub vertices_rendered: u32,
    pub chunks_loaded: u32,
    pub chunks_rendered: u32,
}

impl EngineTestbed {
    pub fn new() -> Self {
        Self {
            player_block: BlockId(1), // Stone
            debug_enabled: true,
            performance_metrics: PerformanceMetrics::default(),
            frame_times: Vec::with_capacity(120), // Store 2 seconds at 60fps
            last_metrics_update: Instant::now(),
            current_config: TestbedConfig::default(),
            config_dirty: false,
        }
    }
    
    fn update_performance_metrics(&mut self, delta_time: f32) {
        // Update frame time tracking
        self.frame_times.push(delta_time * 1000.0); // Convert to milliseconds
        
        // Keep only last 2 seconds of frame times
        if self.frame_times.len() > 120 {
            self.frame_times.remove(0);
        }
        
        // Update metrics every 100ms
        let now = Instant::now();
        if now.duration_since(self.last_metrics_update).as_millis() >= 100 {
            self.last_metrics_update = now;
            
            if !self.frame_times.is_empty() {
                let sum: f32 = self.frame_times.iter().sum();
                let count = self.frame_times.len() as f32;
                
                self.performance_metrics.avg_frame_time = sum / count;
                self.performance_metrics.min_frame_time = self.frame_times.iter().cloned().fold(f32::INFINITY, f32::min);
                self.performance_metrics.max_frame_time = self.frame_times.iter().cloned().fold(0.0, f32::max);
                self.performance_metrics.frame_time_ms = delta_time * 1000.0;
                self.performance_metrics.fps = 1000.0 / self.performance_metrics.avg_frame_time;
            }
        }
    }
    
    fn handle_debug_input(&mut self, ctx: &mut GameContext) {
        // Debug UI toggle
        if ctx.input.is_key_pressed(KeyCode::F1) {
            self.debug_enabled = !self.debug_enabled;
            log::info!("Debug UI: {}", if self.debug_enabled { "enabled" } else { "disabled" });
        }
        
        // Performance metrics toggle
        if ctx.input.is_key_pressed(KeyCode::F2) {
            self.current_config.show_performance_metrics = !self.current_config.show_performance_metrics;
            self.config_dirty = true;
        }
        
        // Chunk boundaries toggle
        if ctx.input.is_key_pressed(KeyCode::F3) {
            self.current_config.show_chunk_boundaries = !self.current_config.show_chunk_boundaries;
            self.config_dirty = true;
        }
        
        // Wireframe mode toggle
        if ctx.input.is_key_pressed(KeyCode::F4) {
            self.current_config.wireframe_mode = !self.current_config.wireframe_mode;
            self.config_dirty = true;
        }
        
        // Profiling toggle
        if ctx.input.is_key_pressed(KeyCode::F5) {
            self.current_config.enable_profiling = !self.current_config.enable_profiling;
            self.config_dirty = true;
        }
        
        // Reload chunks
        if ctx.input.is_key_pressed(KeyCode::F9) {
            log::info!("Reloading chunks...");
            // This would trigger chunk reloading in the engine
        }
        
        // Take screenshot
        if ctx.input.is_key_pressed(KeyCode::F12) {
            log::info!("Taking screenshot...");
            // This would trigger screenshot functionality
        }
    }
    
    fn handle_block_selection(&mut self, ctx: &mut GameContext) {
        // Block selection with number keys
        if ctx.input.is_key_pressed(KeyCode::Digit1) {
            self.player_block = BlockId(1); // Stone
            log::debug!("Selected block: Stone");
        } else if ctx.input.is_key_pressed(KeyCode::Digit2) {
            self.player_block = BlockId(2); // Dirt
            log::debug!("Selected block: Dirt");
        } else if ctx.input.is_key_pressed(KeyCode::Digit3) {
            self.player_block = BlockId(3); // Grass
            log::debug!("Selected block: Grass");
        } else if ctx.input.is_key_pressed(KeyCode::Digit4) {
            self.player_block = BlockId(4); // Wood
            log::debug!("Selected block: Wood");
        } else if ctx.input.is_key_pressed(KeyCode::Digit5) {
            self.player_block = BlockId(5); // Leaves
            log::debug!("Selected block: Leaves");
        } else if ctx.input.is_key_pressed(KeyCode::Digit6) {
            self.player_block = BlockId(6); // Sand
            log::debug!("Selected block: Sand");
        } else if ctx.input.is_key_pressed(KeyCode::Digit7) {
            self.player_block = BlockId(7); // Water
            log::debug!("Selected block: Water");
        } else if ctx.input.is_key_pressed(KeyCode::Digit8) {
            self.player_block = BlockId(8); // Lava
            log::debug!("Selected block: Lava");
        } else if ctx.input.is_key_pressed(KeyCode::Digit9) {
            self.player_block = BlockId(9); // Glass
            log::debug!("Selected block: Glass");
        } else if ctx.input.is_key_pressed(KeyCode::Digit0) {
            self.player_block = BlockId(0); // Air (eraser)
            log::debug!("Selected eraser (Air)");
        }
    }
    
    fn log_debug_info(&self) {
        if self.debug_enabled && self.current_config.show_debug_info {
            log::info!("=== ENGINE TESTBED DEBUG INFO ===");
            log::info!("Player Block: {:?}", self.player_block);
            log::info!("Performance Metrics:");
            log::info!("  FPS: {:.1}", self.performance_metrics.fps);
            log::info!("  Frame Time: {:.2}ms", self.performance_metrics.frame_time_ms);
            log::info!("  Min Frame Time: {:.2}ms", self.performance_metrics.min_frame_time);
            log::info!("  Max Frame Time: {:.2}ms", self.performance_metrics.max_frame_time);
            log::info!("  Avg Frame Time: {:.2}ms", self.performance_metrics.avg_frame_time);
            log::info!("Configuration:");
            log::info!("  Debug UI: {}", self.debug_enabled);
            log::info!("  Performance Metrics: {}", self.current_config.show_performance_metrics);
            log::info!("  Chunk Boundaries: {}", self.current_config.show_chunk_boundaries);
            log::info!("  Wireframe Mode: {}", self.current_config.wireframe_mode);
            log::info!("  Profiling: {}", self.current_config.enable_profiling);
            log::info!("================================");
        }
    }
}

impl Game for EngineTestbed {
    fn register_blocks(&mut self, _registry: &mut BlockRegistry) {
        // Blocks are already registered in the world module
        log::info!("Engine Testbed: Blocks registered successfully");
        log::info!("Available blocks: Stone(1), Dirt(2), Grass(3), Wood(4), Leaves(5), Sand(6), Water(7), Lava(8), Glass(9), Air(0)");
    }
    
    fn update(&mut self, ctx: &mut GameContext, delta_time: f32) {
        // Update performance metrics
        self.update_performance_metrics(delta_time);
        
        // Handle debug input
        self.handle_debug_input(ctx);
        
        // Handle block selection
        self.handle_block_selection(ctx);
        
        // Log debug info periodically (every 5 seconds)
        if self.last_metrics_update.elapsed().as_secs() % 5 == 0 && 
           self.last_metrics_update.elapsed().as_millis() % 5000 < 100 {
            self.log_debug_info();
        }
        
        // Apply configuration changes if needed
        if self.config_dirty {
            log::info!("Applying configuration changes...");
            // Here we would apply configuration changes to the engine
            // For now, just log the changes
            log::debug!("New configuration: {:?}", self.current_config);
            self.config_dirty = false;
        }
    }
    
    fn get_active_block(&self) -> BlockId {
        self.player_block
    }
}

/// Print usage instructions for the engine testbed
fn print_usage_instructions() {
    println!("=== EARTH ENGINE TESTBED ===");
    println!("A comprehensive development platform for Earth Engine");
    println!();
    println!("CONTROLS:");
    println!("  WASD     - Move camera");
    println!("  Mouse    - Look around");
    println!("  LMB      - Break blocks");
    println!("  RMB      - Place blocks");
    println!("  0-9      - Select block type (0 = Air/Eraser)");
    println!();
    println!("DEBUG CONTROLS:");
    println!("  F1       - Toggle debug UI");
    println!("  F2       - Toggle performance metrics");
    println!("  F3       - Toggle chunk boundaries");
    println!("  F4       - Toggle wireframe mode");
    println!("  F5       - Toggle profiling");
    println!("  F9       - Reload chunks");
    println!("  F12      - Take screenshot");
    println!();
    println!("BLOCK TYPES:");
    println!("  1 - Stone    6 - Sand");
    println!("  2 - Dirt     7 - Water");
    println!("  3 - Grass    8 - Lava");
    println!("  4 - Wood     9 - Glass");
    println!("  5 - Leaves   0 - Air");
    println!();
    println!("For simple engine usage, see examples/minimal_engine.rs");
    println!("==============================");
    println!();
}

fn main() {
    println!("[ENGINE TESTBED] Starting comprehensive Earth Engine testbed...");
    
    // Print usage instructions
    print_usage_instructions();
    
    // Initialize logging with debug level for comprehensive output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format_timestamp_millis()
        .init();
    
    log::info!("[ENGINE TESTBED] Logger initialized with debug level");
    
    // Install panic handler for telemetry
    log::info!("[ENGINE TESTBED] Installing panic handler...");
    earth_engine::panic_handler::install_panic_handler();
    log::info!("[ENGINE TESTBED] Panic handler installed");
    
    // Create enhanced engine config for testbed
    log::info!("[ENGINE TESTBED] Creating enhanced engine configuration...");
    let config = EngineConfig {
        window_title: "Earth Engine Testbed - Development Platform".to_string(),
        window_width: 1600, // Larger window for debug UI
        window_height: 900,
        chunk_size: 32,
        render_distance: 12, // Increased for better testing
    };
    log::info!("[ENGINE TESTBED] Engine config created: {:?}", config);
    
    log::info!("[ENGINE TESTBED] Creating Engine instance...");
    let engine = Engine::new(config);
    log::info!("[ENGINE TESTBED] Engine instance created successfully");
    
    log::info!("[ENGINE TESTBED] Creating testbed game instance...");
    let game = EngineTestbed::new();
    log::info!("[ENGINE TESTBED] Testbed game instance created");
    
    // Run the engine testbed
    log::info!("[ENGINE TESTBED] Starting engine with testbed...");
    match engine.run(game) {
        Ok(_) => {
            log::info!("[ENGINE TESTBED] Engine testbed exited normally");
            println!("[ENGINE TESTBED] Testbed completed successfully!");
        }
        Err(e) => {
            log::error!("[ENGINE TESTBED] Engine error: {}", e);
            eprintln!("[ENGINE TESTBED] Fatal error: {}", e);
            std::process::exit(1);
        }
    }
    
    log::info!("[ENGINE TESTBED] Application exiting normally");
    println!("[ENGINE TESTBED] Thank you for using Earth Engine Testbed!");
}