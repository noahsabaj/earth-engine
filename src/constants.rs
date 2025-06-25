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
    pub const DEFAULT_WORLD_SIZE: u32 = 32; // Default world size in chunks for kernel config
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
    pub const COAL_ORE: u16 = 9;
    pub const IRON_ORE: u16 = 10;
    pub const GOLD_ORE: u16 = 11;
    pub const DIAMOND_ORE: u16 = 12;
    pub const BEDROCK: u16 = 13;
    pub const PLANKS: u16 = 14;
    pub const COBBLESTONE: u16 = 15;
    pub const CRAFTING_TABLE: u16 = 16;
    pub const FURNACE: u16 = 17;
    pub const CHEST: u16 = 18;
    pub const TORCH: u16 = 19;
    pub const LADDER: u16 = 20;
    pub const LAVA: u16 = 21;
    pub const LOG: u16 = 22;
    pub const SANDSTONE: u16 = 23;
    pub const RED_SAND: u16 = 24;
    pub const RED_SANDSTONE: u16 = 25;
    pub const TALL_GRASS: u16 = 26;
    pub const FLOWER_RED: u16 = 27;
    pub const FLOWER_YELLOW: u16 = 28;
    pub const CACTUS: u16 = 29;
    pub const SNOW: u16 = 30;
    pub const ICE: u16 = 31;
    pub const MUD: u16 = 32;
    pub const FROZEN_GRASS: u16 = 33;
    pub const WET_STONE: u16 = 34;
    pub const FROST: u16 = 35;
    pub const BRICK: u16 = 36;
    
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
pub mod physics_constants {
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
pub mod camera_constants {
    /// Near clipping plane (voxels)
    /// 0.1m × 10 voxels/m = 1.0 voxel minimum
    pub const ZNEAR: f32 = 1.0;
    
    /// Far clipping plane (voxels) 
    /// 1000m × 10 voxels/m = 10,000 voxels (1km view distance)
    pub const ZFAR: f32 = 10000.0;
    
    /// Default camera position (voxels)
    /// 7m height × 10 voxels/m = 70 voxels above ground (just above sea level)
    pub const DEFAULT_HEIGHT: f32 = 70.0;
    
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
    
    /// Terrain generation height threshold (voxels)
    /// Base terrain generation height: 6.4m × 10 voxels/m = 64 voxels
    pub const TERRAIN_THRESHOLD: i32 = 64;
    
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

/// GPU limits and constraints
pub mod gpu_limits {
    /// Maximum buffer binding size (128MB)
    pub const MAX_BUFFER_BINDING_SIZE: u64 = 134217728; // 128 * 1024 * 1024
    
    /// Maximum workgroup size for compute shaders
    pub const MAX_WORKGROUP_SIZE: u32 = 256;
    
    /// Memory warning threshold in MB (2GB)
    pub const MEMORY_WARNING_THRESHOLD_MB: f32 = 2048.0;
}

/// GPU buffer sizes for world state
pub mod buffer_sizes {
    /// Physics buffers
    pub const COLLISION_PAIRS_BUFFER_SIZE: u64 = 1024 * 1024; // 1MB
    pub const SPATIAL_HASH_BUFFER_SIZE: u64 = 4 * 1024 * 1024; // 4MB
    
    /// Rendering buffers
    pub const VERTEX_BUFFER_SIZE: u64 = 64 * 1024 * 1024; // 64MB
    pub const INDEX_BUFFER_SIZE: u64 = 32 * 1024 * 1024; // 32MB
    pub const INDIRECT_COMMANDS_BUFFER_SIZE: u64 = 1024 * 1024; // 1MB
    
    /// Fluid simulation buffers
    pub const FLUID_CELLS_BUFFER_SIZE: u64 = 8 * 1024 * 1024; // 8MB
    pub const FLUID_PRESSURE_BUFFER_SIZE: u64 = 4 * 1024 * 1024; // 4MB
    pub const FLUID_VELOCITY_BUFFER_SIZE: u64 = 8 * 1024 * 1024; // 8MB
    
    /// Lighting buffers
    pub const LIGHT_SOURCES_BUFFER_SIZE: u64 = 256 * 1024; // 256KB
    
    /// Network buffers
    pub const PACKET_BUFFER_SIZE: u64 = 1024 * 1024; // 1MB
    pub const TEMP_BUFFER_SIZE: usize = 4096; // 4KB
}

/// Gameplay constants
pub mod gameplay {
    /// Target frames per second for physics simulation
    pub const TARGET_FPS: u32 = 60;
    
    /// Fixed timestep for physics (1/60 second)
    pub const FIXED_TIMESTEP: f32 = 1.0 / 60.0;
}

/// Network system constants
pub mod network_constants {
    /// Maximum number of snapshots to keep for interpolation
    pub const MAX_SNAPSHOTS: usize = 20;
    
    /// Interpolation delay in milliseconds
    pub const INTERPOLATION_DELAY_MS: u64 = 100;
    
    /// Maximum lag compensation history in milliseconds
    pub const MAX_HISTORY_MS: u64 = 1000;
    
    /// Maximum history snapshots for lag compensation
    pub const MAX_HISTORY_SNAPSHOTS: usize = 50;
    
    /// Maximum entity view distance in voxels
    pub const MAX_ENTITY_VIEW_DISTANCE: f32 = 128.0;
    
    /// Maximum chunk view distance in chunks
    pub const MAX_CHUNK_VIEW_DISTANCE: i32 = 8;
    
    /// Interest management update rate (Hz)
    pub const INTEREST_UPDATE_RATE: f32 = 2.0;
    
    /// Maximum input buffer size (6 seconds at 20Hz)
    pub const MAX_INPUT_BUFFER: usize = 120;
    
    /// Broadcast channel size
    pub const BROADCAST_CHANNEL_SIZE: usize = 1024;
    
    /// P2P buffer size (16KB)
    pub const P2P_BUFFER_SIZE: usize = 1024 * 16;
}


/// Lighting system constants
pub mod lighting {
    /// Maximum light level (full brightness)
    pub const MAX_LIGHT_LEVEL: u8 = 15;
    
    /// Minimum light level (complete darkness)
    pub const MIN_LIGHT_LEVEL: u8 = 0;
    
    /// Light falloff per block
    pub const LIGHT_FALLOFF: u8 = 1;
}

/// Weather system constants
pub mod weather {
    /// Weather type values (0-7 stored in lower 8 bits)
    pub const WEATHER_CLEAR: u32 = 0;
    pub const WEATHER_RAIN: u32 = 1;
    pub const WEATHER_SNOW: u32 = 2;
    pub const WEATHER_FOG: u32 = 3;
    pub const WEATHER_STORM: u32 = 4;
    pub const WEATHER_HAIL: u32 = 5;
    pub const WEATHER_SANDSTORM: u32 = 6;
    pub const WEATHER_BLIZZARD: u32 = 7;
    
    /// Weather intensity values (0-255 stored in upper 8 bits)
    pub const INTENSITY_NONE: u32 = 0;
    pub const INTENSITY_LIGHT: u32 = 64;
    pub const INTENSITY_MEDIUM: u32 = 128;
    pub const INTENSITY_HEAVY: u32 = 192;
    pub const INTENSITY_EXTREME: u32 = 255;
    
    /// Temperature thresholds (in Celsius * 10)
    pub const FREEZING_POINT: i32 = 0;       // 0°C
    pub const SNOW_THRESHOLD: i32 = 20;      // 2°C
    pub const HOT_THRESHOLD: i32 = 300;      // 30°C
    pub const EXTREME_COLD: i32 = -200;      // -20°C
    pub const EXTREME_HOT: i32 = 400;        // 40°C
    
    /// Typical snow accumulation heights (in voxels) - not guaranteed, emerges from temperature
    pub const SNOW_HEIGHT_TYPICAL_LOW: i32 = 1200;    // 120m - where snow might start appearing
    pub const SNOW_HEIGHT_TYPICAL_HIGH: i32 = 1800;   // 180m - commonly snowy due to temperature
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

/// Persistence constants
pub mod persistence_constants {
    /// Version of the chunk format
    pub const CHUNK_FORMAT_VERSION: u32 = 1;
    
    /// Magic bytes to identify chunk files
    pub const CHUNK_MAGIC: &[u8] = b"ECNK";
    
    /// Default chunk cache size
    pub const DEFAULT_CHUNK_CACHE_SIZE: usize = 1024;
    
    /// Save lock timeout in seconds (30s)
    pub const SAVE_LOCK_TIMEOUT_SECS: u64 = 30;
    
    /// State validation interval in seconds (60s)
    pub const VALIDATION_INTERVAL_SECS: u64 = 60;
    
    /// Maximum snapshot age in seconds (5 minutes)
    pub const MAX_SNAPSHOT_AGE_SECS: u64 = 300;
    
    /// Backup minimum interval in seconds (1 hour)
    pub const BACKUP_MIN_INTERVAL_SECS: u64 = 3600;
    
    /// Backup periodic interval in seconds (24 hours)
    pub const BACKUP_PERIODIC_INTERVAL_SECS: u64 = 86400;
    
    // Compression constants
    
    /// Compression buffer sizes
    pub const COMPRESSION_BUFFER_SIZE: usize = 65536; // 64KB default buffer
    pub const COMPRESSION_CHUNK_SIZE: usize = 32768;  // 32KB chunks for streaming
    
    /// Compression level mappings for flate2
    pub const FLATE2_FAST_LEVEL: u32 = 1;
    pub const FLATE2_DEFAULT_LEVEL: u32 = 6;
    pub const FLATE2_BEST_LEVEL: u32 = 9;
    
    /// Compression level mappings for zstd
    pub const ZSTD_FAST_LEVEL: i32 = 1;
    pub const ZSTD_DEFAULT_LEVEL: i32 = 3;
    pub const ZSTD_BEST_LEVEL: i32 = 9;
    
    /// Compression ratio estimates (for planning)
    pub const GZIP_ESTIMATED_RATIO: f32 = 0.3;   // ~70% reduction
    pub const ZLIB_ESTIMATED_RATIO: f32 = 0.35;  // ~65% reduction
    pub const ZSTD_ESTIMATED_RATIO: f32 = 0.25;  // ~75% reduction
    pub const LZ4_ESTIMATED_RATIO: f32 = 0.5;    // ~50% reduction
    
    /// Entropy thresholds for compression selection
    pub const HIGH_ENTROPY_THRESHOLD: f32 = 7.0;  // Random data
    pub const MEDIUM_ENTROPY_THRESHOLD: f32 = 5.0; // Mixed data
    
    /// Size thresholds for compression selection
    pub const SMALL_DATA_THRESHOLD: usize = 1024; // 1KB - use fast compression
    
    /// Compression magic bytes/headers
    pub const COMPRESSED_DATA_MAGIC: &[u8] = b"HCMP"; // Hearth Compressed
    pub const COMPRESSED_DATA_VERSION: u8 = 1;
}

/// Event system constants
pub mod event_system {
    /// Default maximum number of events in queue
    pub const DEFAULT_MAX_QUEUE_SIZE: usize = 10000;
    
    /// Default batch size for event processing
    pub const DEFAULT_BATCH_SIZE: usize = 100;
    
    /// Default processing interval in milliseconds (~60 FPS)
    pub const DEFAULT_PROCESSING_INTERVAL_MS: u64 = 16;
    
    /// Default maximum processing time per frame in milliseconds
    pub const DEFAULT_MAX_PROCESSING_TIME_MS: f64 = 5.0;
    
    /// Default maximum history size for event replay
    pub const DEFAULT_MAX_HISTORY_SIZE: usize = 1000;
    
    /// Default maximum retry attempts for failed events
    pub const DEFAULT_MAX_RETRY_ATTEMPTS: u32 = 3;
    
    /// Base retry delay in milliseconds
    pub const RETRY_BASE_DELAY_MS: u64 = 100;
    
    /// Initial subscription ID
    pub const INITIAL_SUBSCRIPTION_ID: u64 = 1;
    
    /// Initial event ID
    pub const INITIAL_EVENT_ID: u64 = 1;
}

/// Buffer layout constants
pub mod buffer_layouts {
    use super::core::VOXELS_PER_CHUNK;
    
    // ===== Buffer Element Sizes =====
    
    /// Size of a single voxel data element (u32)
    pub const VOXEL_DATA_SIZE: u64 = 4;
    
    /// Size of chunk metadata structure (8 u32 fields = 32 bytes)
    pub const CHUNK_METADATA_SIZE: u64 = 32;
    
    /// Size of instance data structure
    pub const INSTANCE_DATA_SIZE: u64 = 96; // 4x4 matrix + 4 floats color + 4 floats custom
    
    /// Size of culling instance data
    pub const CULLING_INSTANCE_SIZE: u64 = 32; // 3 floats pos + 1 float radius + 2 u32 + 2 u32 padding
    
    /// Size of indirect draw command
    pub const INDIRECT_COMMAND_SIZE: u64 = 16; // 4 u32 values
    
    /// Size of indirect indexed draw command  
    pub const INDIRECT_INDEXED_COMMAND_SIZE: u64 = 20; // 5 u32 values
    
    /// Size of draw metadata structure
    pub const DRAW_METADATA_SIZE: u64 = 32; // 8 floats + 4 u32 values
    
    /// Size of camera uniform buffer (aligned to 256)
    pub const CAMERA_UNIFORM_SIZE: u64 = 256;
    
    /// Size of culling camera data
    pub const CULLING_CAMERA_SIZE: u64 = 256; // Includes frustum planes
    
    // ===== Buffer Slot Sizes =====
    
    /// Size of a single chunk slot in world buffer
    pub const CHUNK_BUFFER_SLOT_SIZE: u64 = VOXELS_PER_CHUNK as u64 * VOXEL_DATA_SIZE;
    
    /// Maximum chunks based on view distance
    pub const MAX_CHUNKS_VIEW_DISTANCE_3: u32 = 343; // (2*3+1)³ = 7³
    pub const MAX_CHUNKS_VIEW_DISTANCE_4: u32 = 729; // (2*4+1)³ = 9³
    pub const MAX_CHUNKS_VIEW_DISTANCE_5: u32 = 1331; // (2*5+1)³ = 11³
    
    // ===== Alignment Requirements =====
    
    /// WGSL storage buffer alignment
    pub const STORAGE_BUFFER_ALIGNMENT: u64 = 16;
    
    /// WGSL uniform buffer alignment
    pub const UNIFORM_BUFFER_ALIGNMENT: u64 = 256;
    
    /// Vertex buffer optimal alignment
    pub const VERTEX_BUFFER_ALIGNMENT: u64 = 4;
    
    // ===== Buffer Limits =====
    
    /// Maximum instance count per buffer
    pub const MAX_INSTANCES_PER_BUFFER: u32 = 100_000;
    
    /// Maximum indirect draws per pass
    pub const MAX_INDIRECT_DRAWS: u32 = 10_000;
    
    /// Maximum objects/draw commands for GPU culling
    pub const MAX_GPU_OBJECTS: u32 = 100_000;
    
    /// Maximum vertices per mesh
    pub const MAX_VERTICES_PER_MESH: u32 = 65_536;
    
    /// Maximum indices per mesh
    pub const MAX_INDICES_PER_MESH: u32 = 98_304; // 65536 * 1.5
    
    // ===== Mesh Geometry Constants =====
    
    /// Standard cube mesh geometry
    pub const CUBE_VERTEX_COUNT: u32 = 24; // 6 faces * 4 vertices per face
    pub const CUBE_INDEX_COUNT: u32 = 36;  // 6 faces * 2 triangles * 3 indices
    
    /// Default index count for GPU-generated terrain meshes (estimated)
    /// This is a conservative estimate - actual counts vary per chunk
    /// Most chunks will have less than this, preventing buffer overruns
    pub const GPU_TERRAIN_DEFAULT_INDEX_COUNT: u32 = 1000;
    
    /// Chunk bounding sphere radius multiplier (sqrt(3) for cube diagonal)
    pub const CHUNK_BOUNDING_RADIUS_MULTIPLIER: f32 = 1.732;
    
    // ===== Memory Budget Constants =====
    
    /// Target GPU memory usage for world data (MB)
    pub const WORLD_BUFFER_MEMORY_BUDGET_MB: u32 = 512;
    
    /// Target GPU memory usage for instance data (MB)
    pub const INSTANCE_BUFFER_MEMORY_BUDGET_MB: u32 = 128;
    
    /// Target GPU memory usage for mesh data (MB)
    pub const MESH_BUFFER_MEMORY_BUDGET_MB: u32 = 256;
    
    // ===== Helper Functions =====
    
    /// Calculate the number of chunks that fit in a given memory budget
    pub fn chunks_per_memory_budget(budget_mb: u32) -> u32 {
        let budget_bytes = (budget_mb as u64) * 1024 * 1024;
        let chunks = budget_bytes / CHUNK_BUFFER_SLOT_SIZE;
        chunks.min(u32::MAX as u64) as u32
    }
    
    /// Calculate memory requirement for a given view distance
    pub fn memory_for_view_distance(view_distance: u32) -> u64 {
        let diameter = 2 * view_distance + 1;
        let max_chunks = diameter * diameter * diameter;
        max_chunks as u64 * CHUNK_BUFFER_SLOT_SIZE
    }
    
    /// Get recommended view distance for available GPU memory
    pub fn recommended_view_distance(available_memory_mb: u32) -> u32 {
        match available_memory_mb {
            0..=128 => 2,      // Very limited memory
            129..=256 => 3,    // ~45MB for world data
            257..=512 => 4,    // ~95MB for world data  
            513..=1024 => 5,   // ~173MB for world data
            1025..=2048 => 6,  // ~283MB for world data
            _ => 7,            // ~427MB for world data
        }
    }
}

/// GPU-driven rendering constants
pub mod gpu_driven {
    /// Level of Detail (LOD) distances in voxels
    pub const LOD_DISTANCE_NEAR: f32 = 50.0;   // 5m - full detail
    pub const LOD_DISTANCE_FAR: f32 = 200.0;   // 20m - reduced detail
    
    /// Visibility flags for GPU culling
    pub const VISIBILITY_FLAG_VISIBLE: u32 = 1;        // Object is visible
    pub const VISIBILITY_FLAG_ALWAYS_VISIBLE: u32 = 4; // Skip frustum culling
    pub const VISIBILITY_FLAG_DEFAULT: u32 = 5;        // Visible + Always Visible
}

/// System monitoring constants
pub mod monitoring {
    /// Metric sampling configuration
    pub const DEFAULT_METRIC_SAMPLE_SIZE: usize = 300;    // 5 minutes at 60fps
    pub const MAX_METRIC_SAMPLE_SIZE: usize = 3600;       // 1 hour at 60fps
    pub const METRIC_HISTORY_WINDOW_SECS: u64 = 300;      // 5 minutes
    
    /// Performance profiling
    pub const DEFAULT_PROFILING_HISTORY: usize = 1000;     // Max profiling sessions
    pub const PROFILING_SESSION_TIMEOUT_MS: u64 = 5000;   // 5 seconds max session
    
    /// Alert configuration
    pub const MAX_ACTIVE_ALERTS: usize = 100;              // Maximum concurrent alerts
    pub const ALERT_HISTORY_WINDOW_SECS: u64 = 300;       // 5 minutes
    pub const ALERT_DEDUPLICATION_WINDOW_MS: u64 = 1000;  // 1 second
    
    /// Error tracking
    pub const ERROR_HISTORY_WINDOW_SECS: u64 = 600;       // 10 minutes
    pub const ERROR_RATE_SAMPLE_WINDOW_SECS: u64 = 60;    // 1 minute for rate calculation
    
    /// Health check intervals
    pub const HEALTH_CHECK_INTERVAL_MS: u64 = 1000;       // 1 second
    pub const RESOURCE_UPDATE_INTERVAL_MS: u64 = 500;     // 500ms
    
    /// Performance thresholds (defaults)
    pub const DEFAULT_FRAME_TIME_WARNING_MS: f64 = 20.0;  // 50 FPS
    pub const DEFAULT_FRAME_TIME_CRITICAL_MS: f64 = 33.3; // 30 FPS
    pub const DEFAULT_MEMORY_WARNING_PERCENT: f64 = 75.0;
    pub const DEFAULT_MEMORY_CRITICAL_PERCENT: f64 = 90.0;
    pub const DEFAULT_CPU_WARNING_PERCENT: f64 = 80.0;
    pub const DEFAULT_CPU_CRITICAL_PERCENT: f64 = 95.0;
    pub const DEFAULT_ERROR_RATE_WARNING: f64 = 1.0;      // 1 error per minute
    pub const DEFAULT_ERROR_RATE_CRITICAL: f64 = 5.0;     // 5 errors per minute
    
    /// Memory layout optimization
    pub const CACHE_LINE_SIZE: usize = 64;                // CPU cache line size
    pub const METRIC_BUFFER_ALIGNMENT: usize = 64;        // Align to cache lines
}

/// Block ID constants - Wrapped in BlockId type for type safety
/// This requires importing BlockId from the crate when used
pub mod typed_blocks {
    pub const AIR: u16 = super::blocks::AIR;
    pub const STONE: u16 = super::blocks::STONE;
    pub const DIRT: u16 = super::blocks::DIRT;
    pub const GRASS: u16 = super::blocks::GRASS;
    pub const WOOD: u16 = super::blocks::WOOD;
    pub const SAND: u16 = super::blocks::SAND;
    pub const WATER: u16 = super::blocks::WATER;
    pub const LEAVES: u16 = super::blocks::LEAVES;
    pub const GLASS: u16 = super::blocks::GLASS;
    pub const COAL_ORE: u16 = super::blocks::COAL_ORE;
    pub const IRON_ORE: u16 = super::blocks::IRON_ORE;
    pub const GOLD_ORE: u16 = super::blocks::GOLD_ORE;
    pub const DIAMOND_ORE: u16 = super::blocks::DIAMOND_ORE;
    pub const BEDROCK: u16 = super::blocks::BEDROCK;
    pub const PLANKS: u16 = super::blocks::PLANKS;
    pub const COBBLESTONE: u16 = super::blocks::COBBLESTONE;
    pub const CRAFTING_TABLE: u16 = super::blocks::CRAFTING_TABLE;
    pub const FURNACE: u16 = super::blocks::FURNACE;
    pub const CHEST: u16 = super::blocks::CHEST;
    pub const TORCH: u16 = super::blocks::TORCH;
    pub const LADDER: u16 = super::blocks::LADDER;
    pub const LAVA: u16 = super::blocks::LAVA;
    pub const LOG: u16 = super::blocks::LOG;
    pub const SANDSTONE: u16 = super::blocks::SANDSTONE;
    pub const RED_SAND: u16 = super::blocks::RED_SAND;
    pub const RED_SANDSTONE: u16 = super::blocks::RED_SANDSTONE;
    pub const TALL_GRASS: u16 = super::blocks::TALL_GRASS;
    pub const FLOWER_RED: u16 = super::blocks::FLOWER_RED;
    pub const FLOWER_YELLOW: u16 = super::blocks::FLOWER_YELLOW;
    pub const CACTUS: u16 = super::blocks::CACTUS;
    pub const SNOW: u16 = super::blocks::SNOW;
    pub const ICE: u16 = super::blocks::ICE;
    pub const MUD: u16 = super::blocks::MUD;
    pub const FROZEN_GRASS: u16 = super::blocks::FROZEN_GRASS;
    pub const WET_STONE: u16 = super::blocks::WET_STONE;
    pub const FROST: u16 = super::blocks::FROST;
    pub const BRICK: u16 = super::blocks::BRICK;
}

/// Generate WGSL constants file content
pub fn generate_wgsl_constants() -> String {
    format!(r#"// AUTO-GENERATED GPU CONSTANTS - DO NOT EDIT
// Generated from constants.rs

// Core constants
const CHUNK_SIZE: u32 = {}u;
const CHUNK_SIZE_F: f32 = {};
const VOXELS_PER_CHUNK: u32 = {}u;
const MAX_WORLD_SIZE: u32 = {}u;
const MAX_BLOCK_DISTRIBUTIONS: u32 = {}u;

// Block IDs
const BLOCK_AIR: u32 = {}u;
const BLOCK_STONE: u32 = {}u;
const BLOCK_DIRT: u32 = {}u;
const BLOCK_GRASS: u32 = {}u;
const BLOCK_WOOD: u32 = {}u;
const BLOCK_SAND: u32 = {}u;
const BLOCK_WATER: u32 = {}u;
const BLOCK_LEAVES: u32 = {}u;
const BLOCK_GLASS: u32 = {}u;
const BLOCK_CHEST: u32 = {}u;
const BLOCK_LAVA: u32 = {}u;
const BLOCK_BRICK: u32 = {}u;
const BLOCK_SNOW: u32 = {}u;
const BLOCK_ICE: u32 = {}u;
const BLOCK_MUD: u32 = {}u;
const BLOCK_FROZEN_GRASS: u32 = {}u;
const BLOCK_WET_STONE: u32 = {}u;
const BLOCK_FROST: u32 = {}u;
const BLOCK_BEDROCK: u32 = {}u;

// Game blocks start at ID 100
const GAME_BLOCK_START: u32 = {}u;

// Weather constants
const WEATHER_CLEAR: u32 = {}u;
const WEATHER_RAIN: u32 = {}u;
const WEATHER_SNOW: u32 = {}u;
const WEATHER_FOG: u32 = {}u;
const WEATHER_STORM: u32 = {}u;
const WEATHER_HAIL: u32 = {}u;
const WEATHER_SANDSTORM: u32 = {}u;
const WEATHER_BLIZZARD: u32 = {}u;

// Weather intensity
const INTENSITY_NONE: u32 = {}u;
const INTENSITY_LIGHT: u32 = {}u;
const INTENSITY_MEDIUM: u32 = {}u;
const INTENSITY_HEAVY: u32 = {}u;
const INTENSITY_EXTREME: u32 = {}u;

// Temperature thresholds
const FREEZING_POINT: i32 = {}i;
const SNOW_THRESHOLD: i32 = {}i;
const HOT_THRESHOLD: i32 = {}i;
const EXTREME_COLD: i32 = {}i;
const EXTREME_HOT: i32 = {}i;

// Snow heights
const SNOW_HEIGHT_TYPICAL_LOW: i32 = {}i;
const SNOW_HEIGHT_TYPICAL_HIGH: i32 = {}i;

// Terrain constants
const TERRAIN_THRESHOLD: i32 = {}i;
const SEA_LEVEL: i32 = {}i;

// World dimensions
const WORLD_SIZE: u32 = {}u;
const WORLD_HEIGHT: u32 = {}u;

// Weather simulation values
const DEFAULT_HUMIDITY: u32 = 500u;
const MAX_HUMIDITY: u32 = 5000u;
const DEFAULT_WIND_SPEED: u32 = 50u;

// Culling constants
const MAX_RENDER_DISTANCE: f32 = 500.0;

// GPU buffer constants
const MAX_VERTICES_PER_CHUNK: u32 = {}u;
const MAX_INDICES_PER_CHUNK: u32 = {}u;
"#, 
        core::CHUNK_SIZE,
        core::CHUNK_SIZE_F32,
        core::VOXELS_PER_CHUNK,
        core::MAX_WORLD_SIZE,
        core::MAX_BLOCK_DISTRIBUTIONS as u32,
        blocks::AIR as u32,
        blocks::STONE as u32,
        blocks::DIRT as u32,
        blocks::GRASS as u32,
        blocks::WOOD as u32,
        blocks::SAND as u32,
        blocks::WATER as u32,
        blocks::LEAVES as u32,
        blocks::GLASS as u32,
        blocks::CHEST as u32,
        blocks::LAVA as u32,
        blocks::BRICK as u32,
        blocks::SNOW as u32,
        blocks::ICE as u32,
        blocks::MUD as u32,
        blocks::FROZEN_GRASS as u32,
        blocks::WET_STONE as u32,
        blocks::FROST as u32,
        blocks::BEDROCK as u32,
        blocks::GAME_BLOCK_START as u32,
        weather::WEATHER_CLEAR,
        weather::WEATHER_RAIN,
        weather::WEATHER_SNOW,
        weather::WEATHER_FOG,
        weather::WEATHER_STORM,
        weather::WEATHER_HAIL,
        weather::WEATHER_SANDSTORM,
        weather::WEATHER_BLIZZARD,
        weather::INTENSITY_NONE,
        weather::INTENSITY_LIGHT,
        weather::INTENSITY_MEDIUM,
        weather::INTENSITY_HEAVY,
        weather::INTENSITY_EXTREME,
        weather::FREEZING_POINT,
        weather::SNOW_THRESHOLD,
        weather::HOT_THRESHOLD,
        weather::EXTREME_COLD,
        weather::EXTREME_HOT,
        weather::SNOW_HEIGHT_TYPICAL_LOW,
        weather::SNOW_HEIGHT_TYPICAL_HIGH,
        terrain::TERRAIN_THRESHOLD,
        terrain::SEA_LEVEL,
        core::MAX_WORLD_SIZE,
        256u32, // WORLD_HEIGHT (hardcoded for now)
        buffer_layouts::MAX_VERTICES_PER_MESH,
        buffer_layouts::MAX_INDICES_PER_MESH,
    )
}