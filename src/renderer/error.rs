//! Renderer subsystem error handling
//! 
//! This module provides type aliases and helper functions for renderer operations
//! to replace unwrap() calls with proper error handling.

use crate::error::{EngineError, EngineResult};

/// Type alias for renderer operation results
pub type RendererResult<T> = EngineResult<T>;

/// Helper trait for renderer error contexts
pub trait RendererErrorContext<T> {
    fn renderer_context(self, context: &str) -> RendererResult<T>;
}

impl<T, E> RendererErrorContext<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn renderer_context(self, context: &str) -> RendererResult<T> {
        self.map_err(|e| EngineError::SystemError {
            component: "renderer".to_string(),
            error: format!("{}: {}", context, e),
        })
    }
}

/// Create a GPU operation error
pub fn gpu_operation_error(operation: &str, error: impl std::fmt::Display) -> EngineError {
    EngineError::GpuOperationFailed {
        operation: operation.to_string(),
        error: error.to_string(),
    }
}

/// Create a buffer mapping error
pub fn buffer_mapping_error(buffer: &str) -> EngineError {
    EngineError::Internal {
        message: format!("Failed to map GPU buffer: {}", buffer),
    }
}

/// Create a pipeline creation error
pub fn pipeline_creation_error(pipeline: &str, error: impl std::fmt::Display) -> EngineError {
    EngineError::Internal {
        message: format!("Failed to create pipeline '{}': {}", pipeline, error),
    }
}