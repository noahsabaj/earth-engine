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
// TODO: world_unified module is incomplete - future work to unify world and world_gpu
// pub mod world_unified;

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
/// Accepts the full EngineConfig to ensure proper configuration propagation
pub type WorldGeneratorFactory = Box<dyn Fn(Arc<wgpu::Device>, Arc<wgpu::Queue>, &EngineConfig) -> Box<dyn WorldGenerator + Send + Sync> + Send + Sync>;

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

impl EngineConfig {
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<()> {
        // Validate chunk size
        if self.chunk_size == 0 {
            return Err(anyhow::anyhow!("EngineConfig: chunk_size cannot be 0"));
        }
        
        if self.chunk_size > 256 {
            return Err(anyhow::anyhow!("EngineConfig: chunk_size {} exceeds maximum of 256", self.chunk_size));
        }
        
        // Validate render distance
        if self.render_distance == 0 {
            return Err(anyhow::anyhow!("EngineConfig: render_distance cannot be 0"));
        }
        
        // Calculate memory requirements for world buffer
        const GPU_BINDING_LIMIT: u64 = 134217728; // 128MB
        let voxel_data_size = 4u64; // 4 bytes per voxel
        let voxels_per_chunk = (self.chunk_size as u64).pow(3);
        let chunk_memory_bytes = voxels_per_chunk * voxel_data_size;
        
        // Maximum view distance based on chunk size and GPU limits
        let max_safe_chunks = GPU_BINDING_LIMIT / chunk_memory_bytes;
        let max_safe_diameter = (max_safe_chunks as f64).powf(1.0/3.0).floor() as u32;
        let max_safe_view_distance = (max_safe_diameter.saturating_sub(1)) / 2;
        
        log::info!(
            "[EngineConfig] Validation: chunk_size={}, voxels_per_chunk={}, chunk_memory={}KB, max_safe_view_distance={}",
            self.chunk_size, voxels_per_chunk, chunk_memory_bytes / 1024, max_safe_view_distance
        );
        
        // Validate window dimensions
        if self.window_width < 320 || self.window_height < 240 {
            return Err(anyhow::anyhow!("EngineConfig: Window dimensions too small (min 320x240)"));
        }
        
        if self.window_width > 16384 || self.window_height > 16384 {
            return Err(anyhow::anyhow!("EngineConfig: Window dimensions too large (max 16384x16384)"));
        }
        
        log::info!("[EngineConfig] Configuration validated successfully");
        Ok(())
    }
    
    /// Calculate safe view distance for a given chunk size
    pub fn calculate_safe_view_distance(chunk_size: u32) -> u32 {
        const GPU_BINDING_LIMIT: u64 = 134217728; // 128MB
        let voxel_data_size = 4u64; // 4 bytes per voxel
        let voxels_per_chunk = (chunk_size as u64).pow(3);
        let chunk_memory_bytes = voxels_per_chunk * voxel_data_size;
        
        let max_safe_chunks = GPU_BINDING_LIMIT / chunk_memory_bytes;
        let max_safe_diameter = (max_safe_chunks as f64).powf(1.0/3.0).floor() as u32;
        (max_safe_diameter.saturating_sub(1)) / 2
    }
    
    /// Suggest safe configuration parameters
    pub fn suggest_safe_config(&self) -> String {
        let mut suggestions = Vec::new();
        
        if self.chunk_size > 0 {
            let safe_view_distance = Self::calculate_safe_view_distance(self.chunk_size);
            suggestions.push(format!(
                "For chunk_size={}, maximum safe view_distance is {}",
                self.chunk_size, safe_view_distance
            ));
        }
        
        // Common safe configurations
        suggestions.push("Common safe configurations:".to_string());
        suggestions.push("  - chunk_size=32, view_distance=3 (27MB)".to_string());
        suggestions.push("  - chunk_size=50, view_distance=2 (62.5MB)".to_string());
        suggestions.push("  - chunk_size=64, view_distance=1 (64MB)".to_string());
        
        suggestions.join("\n")
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
        
        // Validate configuration before proceeding
        if let Err(e) = config.validate() {
            log::error!("[Engine::new] Configuration validation failed: {}", e);
            log::error!("[Engine::new] Suggestions:\n{}", config.suggest_safe_config());
            panic!("Invalid engine configuration: {}. See log for suggestions.", e);
        }
        
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
