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

/// Atomically write data to a file to prevent corruption
/// 
/// This function writes to a temporary file first, then atomically renames it
/// to the target path. This ensures that either the write succeeds completely
/// or fails completely, preventing partial writes that could corrupt data.
pub fn atomic_write(path: impl AsRef<std::path::Path>, data: &[u8]) -> PersistenceResult<()> {
    use std::fs;
    
    
    let path = path.as_ref();
    
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Create a temporary file in the same directory
    let temp_path = if let Some(parent) = path.parent() {
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| format!(".{}.tmp", s))
            .unwrap_or_else(|| "tmpfile.tmp".to_string());
        parent.join(filename)
    } else {
        path.with_extension("tmp")
    };
    
    // Write to temporary file first
    fs::write(&temp_path, data)
        .map_err(|e| PersistenceError::IoError(
            std::io::Error::new(e.kind(), 
                format!("Failed to write temporary file {}: {}", temp_path.display(), e))))?;
    
    // Atomically rename temporary file to target path
    fs::rename(&temp_path, path)
        .map_err(|e| PersistenceError::IoError(
            std::io::Error::new(e.kind(), 
                format!("Failed to rename {} to {}: {}", temp_path.display(), path.display(), e))))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_atomic_write_success() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let test_data = b"Hello, atomic world!";

        // Atomic write should succeed
        atomic_write(&test_file, test_data).unwrap();

        // File should exist and contain the correct data
        assert!(test_file.exists());
        let read_data = fs::read(&test_file).unwrap();
        assert_eq!(read_data, test_data);
    }

    #[test]
    fn test_atomic_write_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_file = temp_dir.path().join("nested").join("dir").join("test.txt");
        let test_data = b"Nested file content";

        // Should create parent directories automatically
        atomic_write(&nested_file, test_data).unwrap();

        assert!(nested_file.exists());
        let read_data = fs::read(&nested_file).unwrap();
        assert_eq!(read_data, test_data);
    }

    #[test]
    fn test_atomic_write_no_partial_files() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        
        // First write
        atomic_write(&test_file, b"first content").unwrap();
        
        // Second write - if this was non-atomic, we might see partial content
        atomic_write(&test_file, b"second content that is much longer").unwrap();
        
        // Should only see the complete second write
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "second content that is much longer");
        
        // No temporary files should be left behind
        let temp_files: Vec<_> = fs::read_dir(temp_dir.path()).unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_name().to_string_lossy().contains(".tmp")
            })
            .collect();
        assert_eq!(temp_files.len(), 0);
    }
}

