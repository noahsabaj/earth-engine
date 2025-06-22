//! Physics data subsystem error handling
//!
//! This module provides type aliases and helper functions for physics data operations
//! to replace unwrap() calls with proper error handling.

use crate::error::{EngineError, EngineResult};

/// Type alias for physics data operation results
pub type PhysicsDataResult<T> = EngineResult<T>;

/// Helper trait for physics data error contexts
pub trait PhysicsDataErrorContext<T> {
    fn physics_data_context(self, context: &str) -> PhysicsDataResult<T>;
}

impl<T, E> PhysicsDataErrorContext<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn physics_data_context(self, context: &str) -> PhysicsDataResult<T> {
        self.map_err(|e| EngineError::SystemError {
            component: "physics_data".to_string(),
            error: format!("{}: {}", context, e),
        })
    }
}

/// Create a physics data not found error
pub fn physics_data_not_found(entity: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Physics data not found for entity: {}", entity),
    }
}

/// Create an invalid physics state error
pub fn invalid_physics_state(reason: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Invalid physics state: {}", reason),
    }
}

/// Create a collision data error
pub fn collision_data_error(context: &str) -> EngineError {
    EngineError::Internal {
        message: format!("Collision data error: {}", context),
    }
}
