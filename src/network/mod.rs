pub mod error;
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
pub mod player_sync;
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
pub use server::{
    Server, ServerConfig, ServerPlayer,
    server_start, server_stop, server_run_game_loop, server_tick, server_handle_packet,
    server_handle_connect, server_handle_player_input, server_handle_block_break,
    server_handle_block_place, server_handle_chat_message, server_handle_chunk_request,
    server_handle_ping, server_send_player_updates, server_send_time_updates,
    server_disconnect_player, server_start_tcp_accept_thread, server_start_udp_receive_thread,
    server_send_to_player, server_broadcast, server_broadcast_except,
};
pub use client::{
    Client, ClientState, RemotePlayer,
    client_connect, client_disconnect, client_update_player, client_break_block,
    client_place_block, client_send_chat, client_request_chunk, client_update,
    client_start_receive_thread, client_send_packet, client_handle_packet,
};
pub use connection::{Connection, ConnectionState, ConnectionManager};
pub use replication::{
    ReplicationManager, NetworkEntity, NetworkEntityId, ChunkReplicationData,
    ChunkSyncPriority, ReplicationConfig, ChunkSyncStats, IntegratedReplicationSystem,
    ReplicationStats, ReplicationReceiver,
};
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
pub use compression::{
    DeltaEncoder, DeltaDecoder, EntityStateDelta, EntityState, EntityFieldChanges,
    ChunkDelta, CompressedBlockChange, ChunkCompressor, PacketOptimizer,
};
pub use anticheat::{
    AntiCheat, ValidationResult, InteractionType, CombatAction,
};
pub use sync::{
    NetworkSync, SyncStats,
    network_sync_add_player, network_sync_remove_player, network_sync_tick,
};
pub use player_sync::{
    PlayerSyncBridge, PlayerSyncManager, PlayerSyncEvent, PlayerSyncState, 
    PlayerSyncConfig, PlayerSyncStats, PlayerSyncTickResult,
};
pub use disconnect_handler::{
    DisconnectHandler, DisconnectConfig, DisconnectingPlayer, ConnectionState as DisconnectConnectionState, DisconnectStats,
};
pub use error::{NetworkResult, NetworkErrorContext, connection_error, protocol_error};