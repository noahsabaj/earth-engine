//! Attributes subsystem error handling
//! 
//! This module provides type aliases and helper functions for attribute operations
//! to replace unwrap() calls with proper error handling.

use crate::error::{EngineError, EngineResult};

/// Type alias for attribute operation results
pub type AttributeResult<T> = EngineResult<T>;

/// Helper trait for attribute error contexts
pub trait AttributeErrorContext<T> {
    fn attribute_context(self, context: &str) -> AttributeResult<T>
    where
        Self: Sized;
}


impl<T, E> AttributeErrorContext<T> for Result<T, E>
where
    E: Into<EngineError>,
{
    fn attribute_context(self, context: &str) -> AttributeResult<T> {
        self.map_err(|e| {
            let base_error = e.into();
            match base_error {
                EngineError::LockPoisoned { .. } => EngineError::LockPoisoned {
                    resource: format!("attribute::{}", context),
                },
                other => other,
            }
        })
    }
}

/// Create an attribute not found error
pub fn attribute_not_found(key: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Attribute not found: {}", key),
    }
}

/// Create an attribute type error
pub fn attribute_type_error(key: &str, expected: &str, found: &str) -> EngineError {
    EngineError::Internal {
        message: format!("Attribute '{}' type mismatch: expected {}, found {}", key, expected, found),
    }
}

/// Create an attribute validation error
pub fn attribute_validation_error(key: &str, reason: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Attribute '{}' validation failed: {}", key, reason),
    }
}

/// Create an attribute storage error
pub fn storage_error(operation: &str, reason: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Attribute storage {} failed: {}", operation, reason),
    }
}