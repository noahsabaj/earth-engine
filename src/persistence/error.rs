//! Persistence-specific error handling
//! 
//! This module provides helper functions and traits for persistence operations
//! to replace unwrap() calls with proper error handling.

use crate::persistence::{PersistenceError, PersistenceResult};

/// Helper trait for persistence-specific error contexts
pub trait PersistenceErrorContext<T> {
    fn persistence_context(self, context: &str) -> PersistenceResult<T>;
}

impl<T, E> PersistenceErrorContext<T> for Result<T, E>
where
    E: std::error::Error + 'static,
{
    fn persistence_context(self, context: &str) -> PersistenceResult<T> {
        self.map_err(|e| {
            PersistenceError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, format!("{}: {}", context, e))
            )
        })
    }
}

/// Helper for lock operations
pub trait LockResultExt<T> {
    fn persistence_lock(self, resource: &str) -> PersistenceResult<T>;
}

impl<T> LockResultExt<T> for Result<T, std::sync::PoisonError<T>> {
    fn persistence_lock(self, resource: &str) -> PersistenceResult<T> {
        self.map_err(|_| PersistenceError::LockPoisoned(resource.to_string()))
    }
}

/// Create a save error
pub fn save_error(path: impl AsRef<std::path::Path>, error: impl std::fmt::Display) -> PersistenceError {
    PersistenceError::IoError(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Save failed for {}: {}", path.as_ref().display(), error)
        )
    )
}

/// Create a load error
pub fn load_error(path: impl AsRef<std::path::Path>, error: impl std::fmt::Display) -> PersistenceError {
    PersistenceError::IoError(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Load failed for {}: {}", path.as_ref().display(), error)
        )
    )
}

/// Create a corrupted data error
pub fn corrupted_data(reason: impl Into<String>) -> PersistenceError {
    PersistenceError::CorruptedData(reason.into())
}

/// Create a version mismatch error
pub fn version_mismatch(expected: u32, found: u32) -> PersistenceError {
    PersistenceError::VersionMismatch { expected, found }
}