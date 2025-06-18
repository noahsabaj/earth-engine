// Hearth Engine Constants - SINGLE SOURCE OF TRUTH
// 
// This file contains ALL constants used throughout the engine.
// Both CPU and GPU code include this file to ensure perfect consistency.
// 
// CRITICAL: Do NOT define constants anywhere else in the codebase!

/// Core GPU/World constants
pub mod core {
    /// Chunk dimensions - 1dcm³ (10cm) voxels with 50×50×50 chunks (5m³ per chunk)
    pub const CHUNK_SIZE: u32 = 50;
    pub const CHUNK_SIZE_F32: f32 = 50.0;
    pub const VOXELS_PER_CHUNK: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    
    /// World limits
    pub const MAX_WORLD_SIZE: u32 = 512; // 512³ chunks
    pub const MAX_BLOCK_DISTRIBUTIONS: usize = 16;
}

/// Block ID constants - Single source of truth (raw u16 values)
pub mod blocks {
    // Core engine blocks (0-99)
    pub const AIR: u16 = 0;
    pub const STONE: u16 = 1;
    pub const DIRT: u16 = 2;
    pub const GRASS: u16 = 3;
    pub const WOOD: u16 = 4;
    pub const SAND: u16 = 5;
    pub const WATER: u16 = 6;
    pub const LEAVES: u16 = 7;
    pub const GLASS: u16 = 8;
    pub const CHEST: u16 = 9;
    pub const LAVA: u16 = 10;
    pub const BRICK: u16 = 11;
    
    // Reserved for game-specific blocks (100+)
    // Games can define their own blocks starting from ID 100
    pub const GAME_BLOCK_START: u16 = 100;
}

/// World measurement system - SINGLE SOURCE OF TRUTH
/// All measurements in the engine are based on 1dcm³ voxels (10cm cubes)
pub mod measurements {
    /// Core voxel measurement definition
    pub const VOXEL_SIZE_METERS: f32 = 0.1; // 1 voxel = 10cm = 0.1m
    pub const METERS_TO_VOXELS: f32 = 10.0;  // 1m = 10 voxels
    pub const VOXELS_TO_METERS: f32 = 0.1;   // 1 voxel = 0.1m
    
    /// Chunk physical dimensions
    pub const CHUNK_SIZE_VOXELS: f32 = 50.0;
    pub const CHUNK_SIZE_METERS: f32 = CHUNK_SIZE_VOXELS * VOXEL_SIZE_METERS; // 5.0m
    
    /// Conversion utilities for external APIs
    #[inline]
    pub const fn meters_to_voxels(meters: f32) -> f32 {
        meters * METERS_TO_VOXELS
    }
    
    #[inline] 
    pub const fn voxels_to_meters(voxels: f32) -> f32 {
        voxels * VOXELS_TO_METERS
    }
}

/// Physics constants - ALL IN VOXEL UNITS
/// These values are scaled for 1dcm³ voxels, not meters
pub mod physics {
    /// Gravitational acceleration (voxels/s²)
    /// Earth gravity: -9.81 m/s² × 10 voxels/m = -98.1 voxels/s²
    pub const GRAVITY: f32 = -98.1;
    
    /// Terminal velocity for falling objects (voxels/s) 
    /// Realistic terminal velocity: -50 m/s × 10 voxels/m = -500 voxels/s
    pub const TERMINAL_VELOCITY: f32 = -500.0;
    
    /// Fixed physics timestep (seconds)
    /// 60 FPS physics simulation 
    pub const FIXED_TIMESTEP: f32 = 1.0 / 60.0;
    
    /// Spatial hash cell size (voxels)
    /// 4 meters × 10 voxels/m = 40 voxels per cell
    pub const SPATIAL_HASH_CELL_SIZE: f32 = 40.0;
    
    /// Player collision box half-extents (voxels)
    /// Typical player: 0.4m wide, 0.9m tall → 4 voxels wide, 9 voxels tall
    pub const PLAYER_HALF_EXTENTS: [f32; 3] = [4.0, 9.0, 4.0];
    
    /// Block collision box half-extents (voxels)
    /// 1 voxel = 10cm, so half-extents = 5cm = 0.5 voxels
    pub const BLOCK_HALF_EXTENTS: [f32; 3] = [0.5, 0.5, 0.5];
}

/// Camera and rendering constants - ALL IN VOXEL UNITS
pub mod camera {
    /// Near clipping plane (voxels)
    /// 0.1m × 10 voxels/m = 1.0 voxel minimum
    pub const ZNEAR: f32 = 1.0;
    
    /// Far clipping plane (voxels) 
    /// 1000m × 10 voxels/m = 10,000 voxels (1km view distance)
    pub const ZFAR: f32 = 10000.0;
    
    /// Default camera position (voxels)
    /// 10m height × 10 voxels/m = 100 voxels above ground
    pub const DEFAULT_HEIGHT: f32 = 100.0;
    
    /// Camera movement speeds (voxels/s)
    pub const WALK_SPEED: f32 = 43.0;      // ~4.3 m/s walking
    pub const RUN_SPEED: f32 = 80.0;       // ~8.0 m/s running  
    pub const FLY_SPEED: f32 = 100.0;      // ~10.0 m/s flying
}

/// Terrain generation constants - ALL IN VOXEL UNITS
pub mod terrain {
    /// Terrain height variations (voxels)
    /// Mountains: 32m × 10 voxels/m = 320 voxels
    pub const MOUNTAIN_AMPLITUDE: f32 = 320.0;
    
    /// Hill height variations (voxels)
    /// Hills: 8m × 10 voxels/m = 80 voxels  
    pub const HILL_AMPLITUDE: f32 = 80.0;
    
    /// Detail noise amplitude (voxels)
    /// Fine details: 2m × 10 voxels/m = 20 voxels
    pub const DETAIL_AMPLITUDE: f32 = 20.0;
    
    /// Base terrain height (voxels)
    /// Sea level: 64m × 10 voxels/m = 640 voxels
    pub const SEA_LEVEL: i32 = 640;
    
    /// Terrain height limits (voxels)
    /// Range: 10m-200m × 10 voxels/m = 100-2000 voxels
    pub const MIN_HEIGHT: i32 = 100;
    pub const MAX_HEIGHT: i32 = 2000;
}

/// GPU buffer alignment requirements
pub mod alignment {
    /// WGSL requires 16-byte alignment for storage buffers
    pub const STORAGE_BUFFER_ALIGN: u64 = 16;
    
    /// Uniform buffers require 256-byte alignment
    pub const UNIFORM_BUFFER_ALIGN: u64 = 256;
}

/// Shader path constants - Single source of truth for shader locations
pub mod shader_paths {
    /// Base shader directory (relative to src/)
    pub const SHADER_BASE: &str = "gpu/shaders";
    
    /// SOA shader paths
    pub const SOA_TERRAIN_GENERATION: &str = "gpu/shaders/soa/terrain_generation_soa.wgsl";
    
    /// Generated shader paths (SOA only for optimal performance)
    pub const GENERATED_TYPES_SOA: &str = "gpu/shaders/generated/types_soa.wgsl";
    pub const GENERATED_CONSTANTS: &str = "gpu/shaders/generated/constants.wgsl";
    
    /// Common shader includes
    pub const PERLIN_NOISE: &str = "renderer/shaders/perlin_noise.wgsl";
}