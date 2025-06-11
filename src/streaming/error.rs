/// Streaming System Error Handling
/// 
/// Provides error types and utilities for the streaming subsystem.

use crate::error::{EngineError, EngineResult};

/// Streaming-specific result type
pub type StreamingResult<T> = EngineResult<T>;

/// Error context for streaming operations
pub trait StreamingErrorContext<T> {
    fn streaming_context(self, context: &str) -> StreamingResult<T>;
}

impl<T> StreamingErrorContext<T> for Option<T> {
    fn streaming_context(self, context: &str) -> StreamingResult<T> {
        self.ok_or_else(|| EngineError::ResourceNotFound {
            resource_type: "streaming".to_string(),
            id: context.to_string(),
        })
    }
}

impl<T, E> StreamingErrorContext<T> for Result<T, E> 
where 
    E: std::fmt::Display 
{
    fn streaming_context(self, context: &str) -> StreamingResult<T> {
        self.map_err(|e| EngineError::SystemError {
            component: "streaming".to_string(),
            error: format!("{}: {}", context, e),
        })
    }
}