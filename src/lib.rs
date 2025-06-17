#![allow(unused_variables, dead_code, unused_imports)]

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
pub mod system_monitor;
pub mod event_system;
pub mod analysis;
pub mod gpu;

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
            window_title: "Hearth Engine".to_string(),
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
