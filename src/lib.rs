pub mod error;
pub mod panic_handler;
pub mod thread_pool;
pub mod camera;
pub mod crafting;
pub mod ecs;
pub mod game;
pub mod input;
pub mod inventory;
pub mod item;
pub mod lighting;
pub mod network;
pub mod persistence;
pub mod physics;
pub mod profiling;
pub mod renderer;
pub mod ui;
pub mod world;
pub mod weather;
pub mod time;
pub mod particles;
pub mod biome;
pub mod physics_data;
pub mod spatial_index;
pub mod world_gpu;
#[cfg(feature = "native")]
pub mod streaming;
pub mod fluid;
pub mod sdf;
#[cfg(feature = "native")]
pub mod hot_reload;
pub mod morton;
pub mod instance;
pub mod process;
pub mod attributes;
pub mod memory;
pub mod utils;
pub mod world_state;

// Web module removed - no longer supporting browser builds

use anyhow::Result;
use winit::event_loop::{EventLoop, EventLoopBuilder};

pub use error::{EngineError, EngineResult, OptionExt, ErrorContext};
pub use camera::{CameraData, CameraUniform};
pub use game::{Game, GameContext};
pub use input::KeyCode;
pub use physics::{PhysicsWorldData, PhysicsBodyData, AABB};
pub use renderer::Renderer;
pub use world::{Block, BlockId, BlockRegistry, Chunk, ChunkPos, VoxelPos, RenderData, PhysicsProperties, World, Ray, RaycastHit, BlockFace, cast_ray};

/// Main engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub window_title: String,
    pub window_width: u32,
    pub window_height: u32,
    pub chunk_size: u32,
    pub render_distance: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            window_title: "Earth Engine".to_string(),
            window_width: 1280,
            window_height: 720,
            chunk_size: 32,
            render_distance: 8,
        }
    }
}

/// Main engine struct that runs the game loop
pub struct Engine {
    config: EngineConfig,
    event_loop: Option<EventLoop<()>>,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        log::debug!("[Engine::new] Starting engine initialization");
        
        // Force X11 backend for WSL compatibility
        #[cfg(target_os = "linux")]
        let event_loop = {
            log::debug!("[Engine::new] Creating X11 event loop for Linux...");
            use winit::platform::x11::EventLoopBuilderExtX11;
            let result = EventLoopBuilder::new()
                .with_x11()
                .build();
            match result {
                Ok(loop_) => {
                    log::info!("[Engine::new] X11 event loop created successfully");
                    loop_
                }
                Err(e) => {
                    log::error!("[Engine::new] Failed to create X11 event loop: {}", e);
                    panic!("Failed to create event loop: {}", e);
                }
            }
        };
        
        #[cfg(not(target_os = "linux"))]
        let event_loop = {
            log::debug!("[Engine::new] Creating default event loop...");
            match EventLoop::new() {
                Ok(loop_) => {
                    log::info!("[Engine::new] Event loop created successfully");
                    loop_
                }
                Err(e) => {
                    log::error!("[Engine::new] Failed to create event loop: {}", e);
                    panic!("Failed to create event loop: {}", e);
                }
            }
        };
        
        // Initialize thread pool manager with optimized configuration
        let thread_pool_config = thread_pool::ThreadPoolConfig::default();
        if let Err(e) = thread_pool::ThreadPoolManager::initialize(thread_pool_config) {
            log::warn!("[Engine::new] Thread pool manager already initialized or failed: {}", e);
        } else {
            log::info!("[Engine::new] Thread pool manager initialized successfully");
        }
        
        log::info!("[Engine::new] Engine initialization complete");
        
        Self {
            config,
            event_loop: Some(event_loop),
        }
    }

    pub fn run<G: Game + 'static>(mut self, game: G) -> Result<()> {
        log::info!("[Engine::run] Starting engine run method");
        
        let event_loop = match self.event_loop.take() {
            Some(loop_) => {
                log::debug!("[Engine::run] Event loop retrieved successfully");
                loop_
            }
            None => {
                log::error!("[Engine::run] Event loop already taken!");
                panic!("Event loop already taken");
            }
        };
        
        let config = self.config.clone();
        log::info!("[Engine::run] Calling renderer::run with config: {:?}", config);
        
        // This will be implemented when we create the renderer
        let result = renderer::run(event_loop, config, game);
        
        match &result {
            Ok(_) => log::info!("[Engine::run] Renderer returned successfully"),
            Err(e) => log::error!("[Engine::run] Renderer error: {}", e),
        }
        
        result
    }
}

// WASM entry points
#[cfg(target_arch = "wasm32")]
mod wasm_entry {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlCanvasElement;
    use std::sync::{Arc, Mutex};
    use crate::web::{WebGpuContext, WebWorldBuffer, WebRenderer};
    use crate::camera::data_camera::{CameraData, transform};
    use crate::memory::{MemoryManager, MemoryConfig};
    
    // Thread-safe wrapper for WebGL state
    type SharedState = Arc<Mutex<EngineState>>;
    
    struct EngineState {
        context: Arc<WebGpuContext>,
        world_buffer: Arc<WebWorldBuffer>,
        renderer: WebRenderer,
        camera: CameraData,
        memory_manager: Arc<MemoryManager>,
        frame_count: u64,
        last_fps_time: f64,
        fps: f64,
        wireframe: bool,
    }
    
    #[wasm_bindgen]
    pub struct EarthEngineWeb {
        state: SharedState,
    }
    
    #[wasm_bindgen]
    impl EarthEngineWeb {
        // Camera controls
        #[wasm_bindgen]
        pub fn move_forward(&self, amount: f32) {
            if let Ok(mut state) = self.state.lock() {
                state.camera = transform::move_forward(&state.camera, amount);
            }
        }
        
        #[wasm_bindgen]
        pub fn move_backward(&self, amount: f32) {
            if let Ok(mut state) = self.state.lock() {
                state.camera = transform::move_forward(&state.camera, -amount);
            }
        }
        
        #[wasm_bindgen]
        pub fn move_left(&self, amount: f32) {
            if let Ok(mut state) = self.state.lock() {
                state.camera = transform::move_right(&state.camera, -amount);
            }
        }
        
        #[wasm_bindgen]
        pub fn move_right(&self, amount: f32) {
            if let Ok(mut state) = self.state.lock() {
                state.camera = transform::move_right(&state.camera, amount);
            }
        }
        
        #[wasm_bindgen]
        pub fn rotate_camera(&self, yaw_delta: f32, pitch_delta: f32) {
            if let Ok(mut state) = self.state.lock() {
                state.camera = transform::rotate(&state.camera, yaw_delta, pitch_delta);
            }
        }
        
        #[wasm_bindgen]
        pub fn get_stats(&self) -> WebStats {
            if let Ok(state) = self.state.lock() {
                let memory_stats = state.memory_manager.get_stats();
                WebStats {
                    fps: state.fps,
                    gpu_memory: memory_stats.total_allocated as u64,
                    draw_calls: state.renderer.get_draw_calls(),
                    vertices: state.renderer.get_vertex_count(),
                    loaded_chunks: state.world_buffer.get_loaded_chunk_count(),
                }
            } else {
                WebStats::default()
            }
        }
        
        #[wasm_bindgen]
        pub fn set_wireframe(&mut self, enabled: bool) {
            if let Ok(mut state) = self.state.lock() {
                state.wireframe = enabled;
                state.renderer.set_wireframe(enabled);
            }
        }
        
        #[wasm_bindgen]
        pub fn reload_chunks(&mut self) {
            if let Ok(state) = self.state.lock() {
                // In a real implementation, this would reload chunks
                log::info!("Reloading chunks...");
                state.world_buffer.clear_chunks();
            }
        }
        
        #[wasm_bindgen]
        pub fn resize(&mut self, width: u32, height: u32) {
            if let Ok(state) = self.state.lock() {
                state.context.resize(width, height);
            }
        }
        
        #[wasm_bindgen]
        pub fn render(&self, timestamp: f64) {
            if let Ok(mut state) = self.state.lock() {
                // Update FPS
                state.frame_count += 1;
                if state.frame_count % 30 == 0 {
                    let delta = timestamp - state.last_fps_time;
                    state.fps = 30000.0 / delta;
                    state.last_fps_time = timestamp;
                }
                
                // Render frame
                if let Err(e) = state.renderer.render(&state.context, &state.world_buffer, &state.camera) {
                    log::error!("Render error: {:?}", e);
                }
            }
        }
    }
    
    #[wasm_bindgen]
    #[derive(Default)]
    pub struct WebStats {
        pub fps: f64,
        pub gpu_memory: u64,
        pub draw_calls: u32,
        pub vertices: u32,
        pub loaded_chunks: u32,
    }
    
    #[wasm_bindgen]
    pub async fn init_earth_engine(canvas_id: &str) -> Result<EarthEngineWeb, JsValue> {
        // Initialize panic hook for better error messages
        console_error_panic_hook::set_once();
        
        // Initialize logging
        console_log::init_with_level(log::Level::Info)
            .map_err(|_| JsValue::from_str("Failed to initialize logging"))?;
        
        log::info!("Initializing Earth Engine WASM v0.35.0");
        
        // Get canvas element
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("No window object"))?;
        let document = window.document()
            .ok_or_else(|| JsValue::from_str("No document object"))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("Canvas not found"))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("Not a canvas element"))?;
        
        // Initialize WebGPU context
        let context = Arc::new(
            WebGpuContext::new(&canvas).await
                .map_err(|e| JsValue::from_str(&format!("WebGPU init failed: {:?}", e)))?
        );
        
        // Initialize memory manager (Sprint 33)
        let memory_config = MemoryConfig {
            general_pool_size: 64 * 1024 * 1024, // 64MB for web
            persistent_pool_size: 32 * 1024 * 1024, // 32MB
            enable_profiling: true,
        };
        let device = Arc::new(context.device.clone());
        let memory_manager = Arc::new(
            MemoryManager::new(device, memory_config)
        );
        
        // Create world buffer (Sprint 21/22)
        let world_buffer = Arc::new(
            WebWorldBuffer::new(&context)
                .map_err(|e| JsValue::from_str(&format!("World buffer init failed: {:?}", e)))?
        );
        
        // Create renderer (Sprint 22)
        let renderer = WebRenderer::new(&context, &world_buffer)
            .map_err(|e| JsValue::from_str(&format!("Renderer init failed: {:?}", e)))?;
        
        // Initialize camera (Sprint 35 - pure functional)
        let camera = CameraData {
            position: [0.0, 10.0, 10.0],
            yaw_radians: 0.0,
            pitch_radians: -0.3,
            aspect_ratio: canvas.width() as f32 / canvas.height() as f32,
            fovy_radians: 60.0_f32.to_radians(),
            znear: 0.1,
            zfar: 1000.0,
            _padding: [0.0; 3],
        };
        
        // Create engine state
        let state = Arc::new(Mutex::new(EngineState {
            context,
            world_buffer,
            renderer,
            camera,
            memory_manager,
            frame_count: 0,
            last_fps_time: 0.0,
            fps: 0.0,
            wireframe: false,
        }));
        
        log::info!("Earth Engine WASM initialized successfully");
        
        Ok(EarthEngineWeb { state })
    }
}

// Re-export WASM types when building for web
#[cfg(target_arch = "wasm32")]
pub use wasm_entry::*;