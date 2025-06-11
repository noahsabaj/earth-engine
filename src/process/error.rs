//! Process subsystem error handling
//! 
//! This module provides type aliases and helper functions for process operations
//! to replace unwrap() calls with proper error handling.

use crate::error::{EngineError, EngineResult};

/// Type alias for process operation results
pub type ProcessResult<T> = EngineResult<T>;

/// Helper trait for process error contexts
pub trait ProcessErrorContext<T> {
    fn process_context(self, context: &str) -> ProcessResult<T>
    where
        Self: Sized;
}


impl<T, E> ProcessErrorContext<T> for Result<T, E>
where
    E: Into<EngineError>,
{
    fn process_context(self, context: &str) -> ProcessResult<T> {
        self.map_err(|e| {
            let base_error = e.into();
            match base_error {
                EngineError::LockPoisoned { .. } => EngineError::LockPoisoned {
                    resource: format!("process::{}", context),
                },
                other => other,
            }
        })
    }
}

/// Create a process not found error
pub fn process_not_found(id: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Process not found: {}", id),
    }
}

/// Create a thread pool creation error
pub fn thread_pool_error(error: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Failed to create thread pool: {}", error),
    }
}

/// Create a process update error
pub fn process_update_error(id: impl std::fmt::Display, error: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Failed to update process {}: {}", id, error),
    }
}