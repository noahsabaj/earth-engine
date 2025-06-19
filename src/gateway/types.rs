//! Gateway data types - pure data structures with no methods
//! All operations are performed by functions, not methods

use cgmath::{Vector3, Quaternion, Point3};
use serde::{Serialize, Deserialize};

/// Engine request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineRequest {
    // World operations
    GetBlock { pos: VoxelPos },
    SetBlock { pos: VoxelPos, block_id: BlockId },
    BatchSetBlocks { changes: Vec<(VoxelPos, BlockId)> },
    Raycast { origin: Point3<f32>, direction: Vector3<f32>, max_distance: f32 },
    
    // Entity operations
    SpawnEntity { descriptor: EntityDescriptor },
    DespawnEntity { entity_id: EntityId },
    GetEntityTransform { entity_id: EntityId },
    SetEntityTransform { entity_id: EntityId, transform: Transform },
    QueryEntities { filter: EntityFilter },
    
    // Physics operations
    ApplyImpulse { entity_id: EntityId, impulse: Vector3<f32> },
    SetVelocity { entity_id: EntityId, velocity: Vector3<f32> },
    ApplyForce { entity_id: EntityId, force: Vector3<f32> },
    
    // Rendering operations
    SetCamera { camera: CameraDescriptor },
    SetRenderSettings { settings: RenderSettings },
    QueueParticleEffect { effect: ParticleEffect },
    CaptureScreenshot { path: std::path::PathBuf },
    
    // Game state operations
    SaveGame { slot: u32 },
    LoadGame { slot: u32 },
    GetGameState,
}

/// Engine response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineResponse {
    Success,
    Error { message: String },
    Block { block_id: BlockId },
    Entity { entity_id: EntityId },
    Transform { transform: Transform },
    Entities { entities: Vec<EntityInfo> },
    RaycastHit { hit: Option<RaycastHit> },
    GameState { state: GameState },
}

/// Engine event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineEvent {
    // World events
    BlockChanged { pos: VoxelPos, old_block: BlockId, new_block: BlockId },
    ChunkLoaded { chunk_pos: ChunkPos },
    ChunkUnloaded { chunk_pos: ChunkPos },
    ChunkModified { chunk_pos: ChunkPos },
    
    // Entity events
    EntitySpawned { entity_id: EntityId, descriptor: EntityDescriptor },
    EntityDespawned { entity_id: EntityId },
    EntityMoved { entity_id: EntityId, old_pos: Point3<f32>, new_pos: Point3<f32> },
    
    // Physics events
    Collision { entity_a: EntityId, entity_b: EntityId, contact: ContactInfo },
    TriggerEntered { trigger_id: EntityId, entity_id: EntityId },
    TriggerExited { trigger_id: EntityId, entity_id: EntityId },
    
    // Game events
    SaveCompleted { slot: u32 },
    LoadCompleted { slot: u32 },
    SaveFailed { slot: u32, error: String },
    LoadFailed { slot: u32, error: String },
}

/// Simple position data
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VoxelPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Chunk position data
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Block identifier
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u16);

/// Entity identifier
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// Transform data
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub position: Point3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

/// Entity descriptor data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDescriptor {
    pub transform: Transform,
    pub mesh_id: Option<MeshId>,
    pub collider: Option<ColliderDescriptor>,
    pub physics: Option<PhysicsProperties>,
    pub tags: Vec<String>,
}

/// Entity info data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub entity_id: EntityId,
    pub transform: Transform,
    pub tags: Vec<String>,
}

/// Entity filter data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityFilter {
    pub tags: Vec<String>,
    pub position: Option<Point3<f32>>,
    pub radius: Option<f32>,
}

/// Mesh identifier
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeshId(pub u32);

/// Collider descriptor data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColliderDescriptor {
    Box { half_extents: Vector3<f32> },
    Sphere { radius: f32 },
    Capsule { height: f32, radius: f32 },
    Mesh { mesh_id: MeshId },
}

/// Physics properties data
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct PhysicsProperties {
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub is_kinematic: bool,
}

/// Contact info data
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub point: Point3<f32>,
    pub normal: Vector3<f32>,
    pub penetration: f32,
}

/// Camera descriptor data
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct CameraDescriptor {
    pub position: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

/// Render settings data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSettings {
    pub vsync: bool,
    pub render_distance: f32,
    pub shadows: bool,
    pub ambient_occlusion: bool,
    pub anti_aliasing: AntiAliasingMode,
}

/// Anti-aliasing mode data
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum AntiAliasingMode {
    None,
    FXAA,
    MSAA(u32),
    TAA,
}

/// Particle effect data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticleEffect {
    Explosion {
        position: Point3<f32>,
        intensity: f32,
        color: [f32; 4],
    },
    Smoke {
        position: Point3<f32>,
        velocity: Vector3<f32>,
        lifetime: f32,
    },
    Spark {
        position: Point3<f32>,
        direction: Vector3<f32>,
        count: u32,
    },
}

/// Raycast hit data
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct RaycastHit {
    pub position: Point3<f32>,
    pub normal: Vector3<f32>,
    pub distance: f32,
    pub entity_id: Option<EntityId>,
    pub block_pos: Option<VoxelPos>,
}

/// Game state data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub player_transform: Transform,
    pub time_of_day: f32,
    pub weather: WeatherState,
    pub loaded_chunks: Vec<ChunkPos>,
}

/// Weather state data
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum WeatherState {
    Clear,
    Cloudy,
    Rainy { intensity: f32 },
    Stormy { intensity: f32 },
    Foggy { density: f32 },
}

/// Type conversions (pure functions, not methods)
pub fn voxel_to_engine(pos: VoxelPos) -> crate::world::core::VoxelPos {
    crate::world::core::VoxelPos::new(pos.x, pos.y, pos.z)
}

pub fn engine_to_voxel(pos: crate::world::core::VoxelPos) -> VoxelPos {
    VoxelPos { x: pos.x, y: pos.y, z: pos.z }
}

pub fn chunk_to_engine(pos: ChunkPos) -> crate::world::core::ChunkPos {
    crate::world::core::ChunkPos { x: pos.x, y: pos.y, z: pos.z }
}

pub fn engine_to_chunk(pos: crate::world::core::ChunkPos) -> ChunkPos {
    ChunkPos { x: pos.x, y: pos.y, z: pos.z }
}

pub fn block_to_engine(id: BlockId) -> crate::world::core::BlockId {
    crate::world::core::BlockId(id.0)
}

pub fn engine_to_block(id: crate::world::core::BlockId) -> BlockId {
    BlockId(id.0)
}