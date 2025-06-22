use crate::{BlockId, ChunkPos, VoxelPos};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Packet types for network communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Packet {
    Client(ClientPacket),
    Server(ServerPacket),
}

/// Packets sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientPacket {
    /// Initial connection request
    Connect {
        protocol_version: u32,
        username: String,
        password: Option<String>,
    },
    /// Disconnect notification
    Disconnect { reason: String },
    /// Player movement input
    PlayerInput {
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
        movement_state: MovementState,
        sequence: u32, // For lag compensation
    },
    /// Request to break a block
    BlockBreak { position: VoxelPos, sequence: u32 },
    /// Request to place a block
    BlockPlace {
        position: VoxelPos,
        block_id: BlockId,
        face: BlockFace,
        sequence: u32,
    },
    /// Chat message
    ChatMessage { message: String },
    /// Request chunk data
    ChunkRequest { chunk_pos: ChunkPos },
    /// Heartbeat/keepalive
    Ping { timestamp: u64 },
    /// Inventory action
    InventoryAction {
        action: InventoryActionType,
        sequence: u32,
    },
    /// Request world save
    SaveWorldRequest { force: bool, sequence: u32 },
    /// Request player data save
    SavePlayerRequest { sequence: u32 },
    /// Request world load/restore
    LoadWorldRequest { save_name: String, sequence: u32 },
}

/// Packets sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerPacket {
    /// Connection accepted
    ConnectAccept {
        player_id: u32,
        spawn_position: Vec3,
        world_time: f32,
    },
    /// Connection rejected
    ConnectReject { reason: String },
    /// Player disconnected
    PlayerDisconnect { player_id: u32, reason: String },
    /// New player joined
    PlayerJoin {
        player_id: u32,
        username: String,
        position: Vec3,
        rotation: Quat,
    },
    /// Player position update
    PlayerUpdate {
        player_id: u32,
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
        movement_state: MovementState,
        timestamp: u64,
    },
    /// Multiple player updates (sent via UDP)
    PlayerUpdates {
        updates: Vec<PlayerUpdateData>,
        server_tick: u32,
    },
    /// Block changed
    BlockChange {
        position: VoxelPos,
        block_id: BlockId,
        sequence: u32, // Echo client sequence for confirmation
    },
    /// Multiple block changes
    BlockChanges { changes: Vec<BlockChangeData> },
    /// Chat message broadcast
    ChatBroadcast {
        player_id: Option<u32>, // None for server messages
        username: String,
        message: String,
        timestamp: u64,
    },
    /// Chunk data
    ChunkData {
        chunk_pos: ChunkPos,
        compressed_data: Vec<u8>, // Compressed chunk data
    },
    /// Entity spawned
    EntitySpawn {
        entity_id: u32,
        entity_type: EntityType,
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
        metadata: EntityMetadata,
    },
    /// Entity despawned
    EntityDespawn { entity_id: u32 },
    /// Entity position update
    EntityUpdate {
        entity_id: u32,
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
    },
    /// World time update
    TimeUpdate {
        world_time: f32,
        day_cycle_time: f32,
    },
    /// Pong response
    Pong {
        client_timestamp: u64,
        server_timestamp: u64,
    },
    /// Inventory update
    InventoryUpdate { slots: Vec<InventorySlotData> },
    /// Server info/status
    ServerInfo {
        name: String,
        motd: String,
        player_count: u32,
        max_players: u32,
        tps: f32, // Ticks per second
    },
    /// Save operation progress
    SaveProgress {
        operation_id: u32,
        progress: f32, // 0.0 to 1.0
        status: SaveStatus,
        message: String,
    },
    /// Save operation result
    SaveResult {
        operation_id: u32,
        success: bool,
        error_message: Option<String>,
        save_time: u64,
        sequence: u32, // Echo client sequence
    },
    /// World save completed
    WorldSaved {
        chunks_saved: u32,
        players_saved: u32,
        save_time: u64,
        sequence: u32,
    },
    /// Player data saved
    PlayerSaved {
        success: bool,
        error_message: Option<String>,
        sequence: u32,
    },
    /// World loading progress
    LoadProgress {
        operation_id: u32,
        progress: f32,
        status: LoadStatus,
        message: String,
    },
    /// World loaded
    WorldLoaded {
        save_name: String,
        chunks_loaded: u32,
        players_loaded: u32,
        load_time: u64,
        sequence: u32,
    },
    /// Chunk save/load state
    ChunkSaveState {
        chunk_pos: ChunkPos,
        state: ChunkSaveStatus,
        timestamp: u64,
    },
    /// Batch chunk save states
    ChunkSaveStates { states: Vec<ChunkSaveStateData> },
}

/// Player movement state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MovementState {
    Normal,
    Sprinting,
    Crouching,
    Swimming,
    Flying,
}

/// Block face for placement
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BlockFace {
    Top,
    Bottom,
    North,
    South,
    East,
    West,
}

/// Entity types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Item { item_id: u32, count: u32 },
    Mob { mob_type: String },
}

/// Entity metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    pub health: Option<f32>,
    pub name: Option<String>,
    pub custom_data: Vec<u8>,
}

/// Player update data for batch updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerUpdateData {
    pub player_id: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub movement_state: MovementState,
}

/// Block change data for batch updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockChangeData {
    pub position: VoxelPos,
    pub block_id: BlockId,
}

/// Inventory action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryActionType {
    MoveItem {
        from_slot: u32,
        to_slot: u32,
        count: u32,
    },
    DropItem {
        slot: u32,
        count: u32,
    },
    UseItem {
        slot: u32,
    },
    CraftItem {
        recipe_id: String,
    },
}

/// Inventory slot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySlotData {
    pub slot_index: u32,
    pub item_id: Option<u32>,
    pub count: u32,
}

/// Save operation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SaveStatus {
    Starting,
    InProgress,
    CompressingData,
    WritingToDisk,
    CreatingBackup,
    Completed,
    Failed,
}

/// Load operation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoadStatus {
    Starting,
    ReadingFromDisk,
    DecompressingData,
    ValidatingData,
    LoadingChunks,
    LoadingPlayers,
    Completed,
    Failed,
}

/// Chunk save status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ChunkSaveStatus {
    Clean,      // No changes to save
    Dirty,      // Has unsaved changes
    Saving,     // Currently being saved
    Saved,      // Successfully saved
    SaveFailed, // Failed to save
    Loading,    // Currently being loaded
    Loaded,     // Successfully loaded
    LoadFailed, // Failed to load
}

/// Chunk save state data for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkSaveStateData {
    pub chunk_pos: ChunkPos,
    pub state: ChunkSaveStatus,
    pub timestamp: u64,
    pub error_message: Option<String>,
}

/// Packet type for routing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PacketType {
    Reliable,   // TCP - Important packets that must arrive
    Unreliable, // UDP - Fast updates that can be dropped
}

impl Packet {
    /// Get the packet type for routing
    pub fn packet_type(&self) -> PacketType {
        match self {
            // Client packets
            Packet::Client(client) => match client {
                ClientPacket::PlayerInput { .. } => PacketType::Unreliable,
                ClientPacket::Ping { .. } => PacketType::Unreliable,
                // Save/load operations must be reliable
                ClientPacket::SaveWorldRequest { .. } => PacketType::Reliable,
                ClientPacket::SavePlayerRequest { .. } => PacketType::Reliable,
                ClientPacket::LoadWorldRequest { .. } => PacketType::Reliable,
                _ => PacketType::Reliable,
            },
            // Server packets
            Packet::Server(server) => match server {
                ServerPacket::PlayerUpdate { .. } => PacketType::Unreliable,
                ServerPacket::PlayerUpdates { .. } => PacketType::Unreliable,
                ServerPacket::EntityUpdate { .. } => PacketType::Unreliable,
                ServerPacket::Pong { .. } => PacketType::Unreliable,
                // Save/load progress can be unreliable for frequent updates
                ServerPacket::SaveProgress { .. } => PacketType::Unreliable,
                ServerPacket::LoadProgress { .. } => PacketType::Unreliable,
                ServerPacket::ChunkSaveState { .. } => PacketType::Unreliable,
                // Save/load results must be reliable
                ServerPacket::SaveResult { .. } => PacketType::Reliable,
                ServerPacket::WorldSaved { .. } => PacketType::Reliable,
                ServerPacket::PlayerSaved { .. } => PacketType::Reliable,
                ServerPacket::WorldLoaded { .. } => PacketType::Reliable,
                ServerPacket::ChunkSaveStates { .. } => PacketType::Reliable,
                _ => PacketType::Reliable,
            },
        }
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize packet from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}
