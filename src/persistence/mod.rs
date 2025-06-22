//! Persistence system for saving and loading game data

pub mod atomic_save;
pub mod backup;
pub mod chunk_serializer;
pub mod compression;
pub mod error;
pub mod metadata;
pub mod migration;
pub mod network_validator;
pub mod player_data_dop;
pub mod state_validator;
pub mod world_save;

pub use atomic_save::{
    AtomicSaveConfig, AtomicSaveManager, AtomicSaveStats, SaveOperation, SaveOperationResult,
    SavePriority,
};
pub use backup::{BackupManager, BackupPolicy};
pub use chunk_serializer::{ChunkFormat, ChunkSerializer};
pub use compression::{CompressionLevel, CompressionType, Compressor};
pub use error::{atomic_write, LockResultExt, PersistenceErrorContext};
pub use metadata::{SaveVersion, WorldMetadata};
pub use migration::{Migration, MigrationManager};
pub use network_validator::{
    ChunkValidationData, NetworkValidator, PlayerValidationData,
    ValidationConfig as NetworkValidationConfig, ValidationError as NetworkValidationError,
    ValidationResult as NetworkValidationResult, ValidationStats as NetworkValidationStats,
    ValidationType, ValidationWarning as NetworkValidationWarning, WorldValidationState,
};
pub use player_data_dop::{
    PlayerBufferMemoryStats, PlayerColdData, PlayerDataBuffer, PlayerHotData, CACHE_LINE_SIZE,
    MAX_PLAYERS,
};
pub use state_validator::{
    StateSnapshot, StateValidator, ValidationConfig, ValidationError, ValidationResult,
    ValidationStats, ValidationWarning,
};
pub use world_save::{WorldSave, WorldSaveError};

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
