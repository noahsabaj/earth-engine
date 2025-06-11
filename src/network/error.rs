//! Network-specific error handling
//! 
//! This module provides type aliases and helper functions for network operations
//! to replace unwrap() calls with proper error handling.

use crate::error::{EngineError, EngineResult};

/// Type alias for network-specific results
pub type NetworkResult<T> = EngineResult<T>;

/// Helper trait for network-specific error contexts
pub trait NetworkErrorContext {
    fn network_context(self, context: &str) -> NetworkResult<Self>
    where
        Self: Sized;
}

impl<T> NetworkErrorContext for T {
    fn network_context(self, context: &str) -> NetworkResult<Self> {
        Ok(self)
    }
}

impl<T, E> NetworkErrorContext for Result<T, E>
where
    E: Into<EngineError>,
{
    fn network_context(self, context: &str) -> NetworkResult<T> {
        self.map_err(|e| {
            let base_error = e.into();
            match base_error {
                EngineError::LockPoisoned { .. } => EngineError::LockPoisoned {
                    resource: format!("network::{}", context),
                },
                other => other,
            }
        })
    }
}

/// Create a connection error
pub fn connection_error(addr: &str, error: impl std::fmt::Display) -> EngineError {
    EngineError::ConnectionFailed {
        addr: addr.to_string(),
        error: error.to_string(),
    }
}

/// Create a protocol error
pub fn protocol_error(message: impl Into<String>) -> EngineError {
    EngineError::ProtocolError {
        message: message.into(),
    }
}