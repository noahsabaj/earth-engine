use serde::{Serialize, Deserialize};
use crate::world::{VoxelPos, BlockId, ChunkPos};
use glam::{Vec3, Quat};

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
    Disconnect {
        reason: String,
    },
    /// Player movement input
    PlayerInput {
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
        movement_state: MovementState,
        sequence: u32, // For lag compensation
    },
    /// Request to break a block
    BlockBreak {
        position: VoxelPos,
        sequence: u32,
    },
    /// Request to place a block
    BlockPlace {
        position: VoxelPos,
        block_id: BlockId,
        face: BlockFace,
        sequence: u32,
    },
    /// Chat message
    ChatMessage {
        message: String,
    },
    /// Request chunk data
    ChunkRequest {
        chunk_pos: ChunkPos,
    },
    /// Heartbeat/keepalive
    Ping {
        timestamp: u64,
    },
    /// Inventory action
    InventoryAction {
        action: InventoryActionType,
        sequence: u32,
    },
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
    ConnectReject {
        reason: String,
    },
    /// Player disconnected
    PlayerDisconnect {
        player_id: u32,
        reason: String,
    },
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
    BlockChanges {
        changes: Vec<BlockChangeData>,
    },
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
    EntityDespawn {
        entity_id: u32,
    },
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
    InventoryUpdate {
        slots: Vec<InventorySlotData>,
    },
    /// Server info/status
    ServerInfo {
        name: String,
        motd: String,
        player_count: u32,
        max_players: u32,
        tps: f32, // Ticks per second
    },
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
    Item {
        item_id: u32,
        count: u32,
    },
    Mob {
        mob_type: String,
    },
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
                _ => PacketType::Reliable,
            },
            // Server packets
            Packet::Server(server) => match server {
                ServerPacket::PlayerUpdate { .. } => PacketType::Unreliable,
                ServerPacket::PlayerUpdates { .. } => PacketType::Unreliable,
                ServerPacket::EntityUpdate { .. } => PacketType::Unreliable,
                ServerPacket::Pong { .. } => PacketType::Unreliable,
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