#![allow(unused_variables, dead_code, unused_imports)]

// Core engine modules
pub mod error;
pub mod panic_handler;

// Essential systems
pub mod camera;
pub mod game;
pub mod input;
pub mod lighting;
pub mod memory;
pub mod morton;
pub mod network;
pub mod particles;
pub mod persistence;
pub mod physics;
pub mod renderer;
pub mod world;
pub mod world_gpu;

// Advanced features removed - deemed premature for core engine

// GPU and data systems
pub mod gpu;
pub mod spatial_index;

// Utilities
pub mod thread_pool;
pub mod utils;
pub mod world_state;
pub mod system_monitor;
pub mod event_system;
pub mod instance;
pub mod process;
pub mod profiling;

// Web module removed - no longer supporting browser builds

use anyhow::Result;
use std::sync::Arc;
use winit::event_loop::{EventLoop, EventLoopBuilder};

pub use error::{EngineError, EngineResult, OptionExt, ErrorContext};
pub use camera::{CameraData, CameraUniform};
pub use game::{GameContext, GameData};
pub use input::KeyCode;
pub use physics::{AABB};
pub use renderer::Renderer;
pub use world::{Block, BlockId, BlockRegistry, Chunk, ChunkPos, VoxelPos, RenderData, PhysicsProperties, World, Ray, RaycastHit, BlockFace, cast_ray, WorldGenerator};

// Re-export wgpu for games that need GPU access (e.g., custom world generators)
pub use wgpu;

/// World generator type for EngineConfig
#[derive(Debug, Clone, PartialEq)]
pub enum WorldGeneratorType {
    Default,
    DangerMoney,
    Custom(String),
}

/// Factory function type for creating world generators when GPU resources are available
pub type WorldGeneratorFactory = Box<dyn Fn(Arc<wgpu::Device>, Arc<wgpu::Queue>) -> Box<dyn WorldGenerator + Send + Sync> + Send + Sync>;

/// Main engine configuration
pub struct EngineConfig {
    pub window_title: String,
    pub window_width: u32,
    pub window_height: u32,
    pub chunk_size: u32,
    pub render_distance: u32,
    pub world_generator: Option<Box<dyn WorldGenerator + Send + Sync>>,
    pub world_generator_type: WorldGeneratorType,
    pub world_generator_factory: Option<WorldGeneratorFactory>,
}

impl std::fmt::Debug for EngineConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineConfig")
            .field("window_title", &self.window_title)
            .field("window_width", &self.window_width)
            .field("window_height", &self.window_height)
            .field("chunk_size", &self.chunk_size)
            .field("render_distance", &self.render_distance)
            .field("world_generator", &self.world_generator.as_ref().map(|_| "<Custom WorldGenerator>"))
            .field("world_generator_type", &self.world_generator_type)
            .field("world_generator_factory", &self.world_generator_factory.as_ref().map(|_| "<WorldGenerator Factory>"))
            .finish()
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            window_title: "Hearth Engine".to_string(),
            window_width: 1280,
            window_height: 720,
            chunk_size: 50, // Optimized for 1dcm³ (10cm) voxels: 5m x 5m x 5m chunks
            render_distance: 8,
            world_generator: None, // Use engine's default generator when None
            world_generator_type: WorldGeneratorType::Default,
            world_generator_factory: None, // Use engine's default generator when None
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

    pub fn run<G: GameData + 'static>(mut self, game: G) -> Result<()> {
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
        
        let config = self.config;
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
