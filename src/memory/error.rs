//! Memory subsystem error handling
//!
//! This module provides type aliases and helper functions for memory operations
//! to replace unwrap() calls with proper error handling.

use crate::error::{EngineError, EngineResult};

/// Type alias for memory operation results
pub type MemoryResult<T> = EngineResult<T>;

/// Helper trait for memory error contexts
pub trait MemoryErrorContext<T> {
    fn memory_context(self, context: &str) -> MemoryResult<T>
    where
        Self: Sized;
}

impl<T, E> MemoryErrorContext<T> for Result<T, E>
where
    E: Into<EngineError>,
{
    fn memory_context(self, context: &str) -> MemoryResult<T> {
        self.map_err(|e| {
            let base_error = e.into();
            match base_error {
                EngineError::LockPoisoned { .. } => EngineError::LockPoisoned {
                    resource: format!("memory::{}", context),
                },
                other => other,
            }
        })
    }
}

/// Create an allocation error
pub fn allocation_error(size: usize, reason: impl std::fmt::Display) -> EngineError {
    EngineError::AllocationFailed {
        size,
        reason: reason.to_string(),
    }
}

/// Create an out of memory error
pub fn out_of_memory_error(requested: usize, available: usize) -> EngineError {
    EngineError::OutOfMemory {
        requested,
        available,
    }
}
