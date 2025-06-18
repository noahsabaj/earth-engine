pub mod error;
pub mod packet;
pub mod protocol;
pub mod connection;
pub mod interpolation;
pub mod lag_compensation;
pub mod prediction;
pub mod interest;
pub mod anticheat;
pub mod disconnect_handler;

pub use packet::{
    Packet, PacketType, ClientPacket, ServerPacket,
    MovementState, BlockFace, EntityType, EntityMetadata,
    PlayerUpdateData, BlockChangeData, InventoryActionType,
    InventorySlotData, SaveStatus, LoadStatus, ChunkSaveStatus,
    ChunkSaveStateData,
};
pub use protocol::{
    Protocol, PROTOCOL_VERSION,
    DEFAULT_TCP_PORT, DEFAULT_UDP_PORT,
    TICK_DURATION, TICK_RATE,
    KEEPALIVE_INTERVAL, CONNECTION_TIMEOUT,
};
pub use connection::{Connection, ConnectionState, ConnectionManager};
pub use interpolation::{
    EntityInterpolator, InterpolationManager, PositionSnapshot,
    entity_interpolator_add_snapshot, entity_interpolator_get_interpolated,
    entity_interpolator_set_interpolation_delay, entity_interpolator_set_extrapolation,
    entity_interpolator_clear,
    interpolation_manager_add_snapshot, interpolation_manager_get_interpolated,
    interpolation_manager_remove_entity, interpolation_manager_set_global_delay,
    interpolation_manager_set_global_extrapolation, interpolation_manager_auto_adjust_delay,
};
pub use lag_compensation::{
    LagCompensation, PlayerStateSnapshot, WorldStateSnapshot, BlockChange, HitValidation,
    lag_compensation_add_player_snapshot, lag_compensation_add_world_snapshot,
    lag_compensation_update_time, lag_compensation_cleanup_old_history,
};
pub use prediction::{
    ClientPrediction, PlayerInput, PredictedState, MoveValidator, MoveValidationError,
};
pub use interest::{
    InterestManager, RegionCoord, PlayerInterest, InterestStats,
    interest_update_position, interest_set_view_distance,
    interest_add_player, interest_remove_player, interest_update_player_position,
    interest_update_entity_position, interest_remove_entity, interest_update_all_interests,
    interest_update_player_interests,
};
// Compression module removed - used game-specific inventory types
pub use anticheat::{
    AntiCheat, ValidationResult, InteractionType, CombatAction,
};
// Sync module removed - had game-specific dependencies
// Player sync module removed - used game-specific inventory types
pub use disconnect_handler::{
    DisconnectHandler, DisconnectConfig, DisconnectingPlayer, ConnectionState as DisconnectConnectionState, DisconnectStats,
};
pub use error::{NetworkResult, NetworkErrorContext, connection_error, protocol_error};