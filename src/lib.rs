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
pub mod streaming;
pub mod fluid;
pub mod sdf;
pub mod hot_reload;
pub mod morton;
pub mod instance;
pub mod process;

// Web-specific module
#[cfg(target_arch = "wasm32")]
pub mod web;

use anyhow::Result;
use winit::event_loop::{EventLoop, EventLoopBuilder};

pub use camera::Camera;
pub use game::{Game, GameContext};
pub use input::KeyCode;
pub use physics::{PhysicsWorld, PhysicsBody, RigidBody, AABB};
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
        // Force X11 backend for WSL compatibility
        #[cfg(target_os = "linux")]
        let event_loop = {
            use winit::platform::x11::EventLoopBuilderExtX11;
            EventLoopBuilder::new()
                .with_x11()
                .build()
                .expect("Failed to create event loop")
        };
        
        #[cfg(not(target_os = "linux"))]
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        
        Self {
            config,
            event_loop: Some(event_loop),
        }
    }

    pub fn run<G: Game + 'static>(mut self, game: G) -> Result<()> {
        let event_loop = self.event_loop.take().expect("Event loop already taken");
        let config = self.config.clone();
        
        // This will be implemented when we create the renderer
        renderer::run(event_loop, config, game)
    }
}

// WASM entry points
#[cfg(target_arch = "wasm32")]
mod wasm_entry {
    use wasm_bindgen::prelude::*;
    use crate::web;
    
    #[wasm_bindgen]
    pub struct EarthEngineWeb {
        // Internal state would go here
    }
    
    #[wasm_bindgen]
    impl EarthEngineWeb {
        #[wasm_bindgen(constructor)]
        pub fn new() -> Self {
            Self {}
        }
        
        #[wasm_bindgen]
        pub fn get_stats(&self) -> WebStats {
            WebStats {
                fps: 60.0,
                gpu_memory: 100 * 1024 * 1024,
                draw_calls: 50,
                vertices: 100000,
                loaded_chunks: 64,
            }
        }
        
        #[wasm_bindgen]
        pub fn set_wireframe(&mut self, enabled: bool) {
            log::info!("Wireframe mode: {}", enabled);
        }
        
        #[wasm_bindgen]
        pub fn reload_chunks(&mut self) {
            log::info!("Reloading chunks");
        }
        
        #[wasm_bindgen]
        pub fn resize(&mut self, width: u32, height: u32) {
            log::info!("Resizing to {}x{}", width, height);
        }
    }
    
    #[wasm_bindgen]
    pub struct WebStats {
        pub fps: f64,
        pub gpu_memory: u64,
        pub draw_calls: u32,
        pub vertices: u32,
        pub loaded_chunks: u32,
    }
    
    #[wasm_bindgen]
    pub async fn start_earth_engine() -> Result<EarthEngineWeb, JsValue> {
        // Initialize panic hook for better error messages
        console_error_panic_hook::set_once();
        
        // Run the web version
        web::run_web().await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(EarthEngineWeb::new())
    }
}

// Re-export WASM types when building for web
#[cfg(target_arch = "wasm32")]
pub use wasm_entry::*;