//! Instance subsystem error handling
//!
//! This module provides type aliases and helper functions for instance operations
//! to replace unwrap() calls with proper error handling.

use crate::error::{EngineError, EngineResult};

/// Type alias for instance operation results
pub type InstanceResult<T> = EngineResult<T>;

/// Helper trait for instance error contexts
pub trait InstanceErrorContext<T> {
    fn instance_context(self, context: &str) -> InstanceResult<T>
    where
        Self: Sized;
}

impl<T, E> InstanceErrorContext<T> for Result<T, E>
where
    E: Into<EngineError>,
{
    fn instance_context(self, context: &str) -> InstanceResult<T> {
        self.map_err(|e| {
            let base_error = e.into();
            match base_error {
                EngineError::LockPoisoned { .. } => EngineError::LockPoisoned {
                    resource: format!("instance::{}", context),
                },
                other => other,
            }
        })
    }
}

/// Create an instance not found error
pub fn instance_not_found(id: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Instance not found: {}", id),
    }
}

/// Create a metadata error
pub fn metadata_error(key: &str, reason: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Metadata error for key '{}': {}", key, reason),
    }
}

/// Create a type mismatch error
pub fn type_mismatch_error(expected: &str, found: &str) -> EngineError {
    EngineError::Internal {
        message: format!("Type mismatch: expected {}, found {}", expected, found),
    }
}

/// Create a timestamp error
pub fn timestamp_error(context: &str) -> EngineError {
    EngineError::Internal {
        message: format!("Failed to get system timestamp for {}", context),
    }
}
