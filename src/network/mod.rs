pub mod anticheat;
pub mod connection;
pub mod disconnect_handler;
pub mod error;
pub mod interest;
pub mod interpolation;
pub mod lag_compensation;
pub mod packet;
pub mod prediction;
pub mod protocol;

pub use connection::{Connection, ConnectionManager, ConnectionState};
pub use interest::{
    interest_add_player, interest_remove_entity, interest_remove_player,
    interest_set_view_distance, interest_update_all_interests, interest_update_entity_position,
    interest_update_player_interests, interest_update_player_position, interest_update_position,
    InterestManager, InterestStats, PlayerInterest, RegionCoord,
};
pub use interpolation::{
    entity_interpolator_add_snapshot, entity_interpolator_clear,
    entity_interpolator_get_interpolated, entity_interpolator_set_extrapolation,
    entity_interpolator_set_interpolation_delay, interpolation_manager_add_snapshot,
    interpolation_manager_auto_adjust_delay, interpolation_manager_get_interpolated,
    interpolation_manager_remove_entity, interpolation_manager_set_global_delay,
    interpolation_manager_set_global_extrapolation, EntityInterpolator, InterpolationManager,
    PositionSnapshot,
};
pub use lag_compensation::{
    lag_compensation_add_player_snapshot, lag_compensation_add_world_snapshot,
    lag_compensation_cleanup_old_history, lag_compensation_update_time, BlockChange, HitValidation,
    LagCompensation, PlayerStateSnapshot, WorldStateSnapshot,
};
pub use packet::{
    BlockChangeData, BlockFace, ChunkSaveStateData, ChunkSaveStatus, ClientPacket, EntityMetadata,
    EntityType, InventoryActionType, InventorySlotData, LoadStatus, MovementState, Packet,
    PacketType, PlayerUpdateData, SaveStatus, ServerPacket,
};
pub use prediction::{
    ClientPrediction, MoveValidationError, MoveValidator, PlayerInput, PredictedState,
};
pub use protocol::{
    Protocol, CONNECTION_TIMEOUT, DEFAULT_TCP_PORT, DEFAULT_UDP_PORT, KEEPALIVE_INTERVAL,
    PROTOCOL_VERSION, TICK_DURATION, TICK_RATE,
};
// Compression module removed - used game-specific inventory types
pub use anticheat::{AntiCheat, CombatAction, InteractionType, ValidationResult};
// Sync module removed - had game-specific dependencies
// Player sync module removed - used game-specific inventory types
pub use disconnect_handler::{
    ConnectionState as DisconnectConnectionState, DisconnectConfig, DisconnectHandler,
    DisconnectStats, DisconnectingPlayer,
};
pub use error::{connection_error, protocol_error, NetworkErrorContext, NetworkResult};
