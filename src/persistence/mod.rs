//! Persistence system for saving and loading game data

// Data modules (pure data structures)
pub mod atomic_save_data;
pub mod backup_data;
pub mod chunk_serializer_data;
pub mod compression_data;
pub mod metadata_data;
pub mod migration_data;
pub mod network_validator_data;
pub mod player_data_dop;
pub mod state_validator_data;
pub mod world_save_data;

// Operation modules (pure functions)
pub mod atomic_save_operations;
pub mod backup_operations;
pub mod chunk_serializer_operations;
pub mod compression_operations;
pub mod metadata_operations;
pub mod migration_operations;
pub mod network_validator_operations;
pub mod state_validator_operations;
pub mod world_save_operations;

// Utility modules
pub mod error;

// Re-export data structures
pub use atomic_save_data::{
    AtomicSaveConfig, AtomicSaveData, AtomicSaveStats, SaveOperation, SaveOperationResult,
    SavePriority,
};
pub use backup_data::{BackupInfo, BackupManagerData, BackupPolicy, BackupReason, BackupTriggers, RetentionPolicy};
pub use chunk_serializer_data::{ChunkFormat, ChunkSerializerContext};
pub use compression_data::{CompressionAlgorithm, CompressionLevel, CompressionContext};
pub use metadata_data::{
    BannedPlayer, Difficulty, GameRules, SaveVersion, ServerMetadata, WorldBounds, 
    WorldMetadata, WorldStatistics, SAVE_VERSION,
};
pub use migration_data::{
    MigrationData, MigrationManagerData, MigrationStep, MigrationSummary, MigrationType,
    MigrationValidatorData,
};
pub use network_validator_data::{
    ChunkValidationData, NetworkValidatorData, PlayerValidationData, ValidationConfig as NetworkValidationConfig, 
    ValidationError as NetworkValidationError, ValidationResult as NetworkValidationResult, 
    ValidationStats as NetworkValidationStats, ValidationType, ValidationWarning as NetworkValidationWarning, 
    WorldValidationState,
};
pub use player_data_dop::{
    PlayerBufferMemoryStats, PlayerColdData, PlayerDataBuffer, PlayerHotData, CACHE_LINE_SIZE,
    MAX_PLAYERS,
};
pub use state_validator_data::{
    StateSnapshot, StateValidatorData, ValidationConfig, ValidationError, ValidationResult,
    ValidationStats, ValidationWarning,
};
pub use world_save_data::{WorldSaveData, WorldSaveError};

// Re-export commonly used operations
pub use atomic_save_operations::{create_atomic_save_manager, queue_operation, process_next_operation};
pub use backup_operations::{create_backup_manager, create_backup, restore_backup, list_backups};
pub use chunk_serializer_operations::{serialize_chunk, deserialize_chunk, analyze_chunk};
pub use compression_operations::{compress, decompress, analyze_data};
pub use metadata_operations::{create_world_metadata, validate_metadata};
pub use migration_operations::{create_migration_manager, migrate_world};
pub use network_validator_operations::{create_network_validator, validate_chunk_save, validate_chunk_load};
pub use state_validator_operations::{create_state_validator, validate_consistency};
pub use world_save_operations::{create_world_save, load_world_save, save_world, save_chunk, load_chunk};

// Re-export error utilities
pub use error::{atomic_write, LockResultExt, PersistenceErrorContext};

/// Result type for persistence operations
pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Errors that can occur during persistence operations
#[derive(Debug)]
pub enum PersistenceError {
    IoError(std::io::Error),
    SerializationError(String),
    DeserializationError(String),
    CompressionError(String),
    VersionMismatch { expected: u32, found: u32 },
    CorruptedData(String),
    MigrationError(String),
    BackupError(String),
    LockPoisoned(String),
    PlayerNotFound(String),
    CapacityExceeded(String),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistenceError::IoError(e) => write!(f, "IO error: {}", e),
            PersistenceError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            PersistenceError::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
            PersistenceError::CompressionError(e) => write!(f, "Compression error: {}", e),
            PersistenceError::VersionMismatch { expected, found } => {
                write!(
                    f,
                    "Version mismatch: expected {}, found {}",
                    expected, found
                )
            }
            PersistenceError::CorruptedData(e) => write!(f, "Corrupted data: {}", e),
            PersistenceError::MigrationError(e) => write!(f, "Migration error: {}", e),
            PersistenceError::BackupError(e) => write!(f, "Backup error: {}", e),
            PersistenceError::LockPoisoned(e) => write!(f, "Lock poisoned: {}", e),
            PersistenceError::PlayerNotFound(e) => write!(f, "Player not found: {}", e),
            PersistenceError::CapacityExceeded(e) => write!(f, "Capacity exceeded: {}", e),
        }
    }
}

impl std::error::Error for PersistenceError {}

impl From<std::io::Error> for PersistenceError {
    fn from(err: std::io::Error) -> Self {
        PersistenceError::IoError(err)
    }
}

impl From<bincode::Error> for PersistenceError {
    fn from(err: bincode::Error) -> Self {
        PersistenceError::SerializationError(err.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for PersistenceError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        PersistenceError::LockPoisoned("A thread panicked while holding a lock".to_string())
    }
}
