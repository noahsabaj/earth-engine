pub mod packet;
pub mod protocol;
pub mod server;
pub mod client;
pub mod connection;
pub mod replication;
pub mod interpolation;
pub mod lag_compensation;
pub mod prediction;
pub mod interest;
pub mod compression;
pub mod anticheat;
pub mod sync;

pub use packet::{
    Packet, PacketType, ClientPacket, ServerPacket,
    MovementState, BlockFace, EntityType, EntityMetadata,
    PlayerUpdateData, BlockChangeData, InventoryActionType,
    InventorySlotData,
};
pub use protocol::{
    Protocol, PROTOCOL_VERSION,
    DEFAULT_TCP_PORT, DEFAULT_UDP_PORT,
    TICK_DURATION, TICK_RATE,
    KEEPALIVE_INTERVAL, CONNECTION_TIMEOUT,
};
pub use server::{Server, ServerConfig, ServerPlayer};
pub use client::{Client, ClientState, RemotePlayer};
pub use connection::{Connection, ConnectionState, ConnectionManager};
pub use replication::{ReplicationManager, NetworkEntity, NetworkEntityId};
pub use interpolation::{
    EntityInterpolator, InterpolationManager, PositionSnapshot,
};
pub use lag_compensation::{
    LagCompensation, PlayerStateSnapshot, WorldStateSnapshot, BlockChange, HitValidation,
};
pub use prediction::{
    ClientPrediction, PlayerInput, PredictedState, MoveValidator, MoveValidationError,
};
pub use interest::{
    InterestManager, RegionCoord, PlayerInterest, InterestStats,
};
pub use compression::{
    DeltaEncoder, DeltaDecoder, EntityStateDelta, EntityState, EntityFieldChanges,
    ChunkDelta, CompressedBlockChange, ChunkCompressor, PacketOptimizer,
};
pub use anticheat::{
    AntiCheat, ValidationResult, InteractionType, CombatAction,
};
pub use sync::{
    NetworkSync, SyncStats,
};