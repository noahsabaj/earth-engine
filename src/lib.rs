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