/// SDF System Error Handling
/// 
/// Provides error types and utilities for the SDF subsystem.

use crate::error::{EngineError, EngineResult};

/// SDF-specific result type
pub type SdfResult<T> = EngineResult<T>;

/// Error context for SDF operations
pub trait SdfErrorContext<T> {
    fn sdf_context(self, context: &str) -> SdfResult<T>;
}

impl<T> SdfErrorContext<T> for Option<T> {
    fn sdf_context(self, context: &str) -> SdfResult<T> {
        self.ok_or_else(|| EngineError::ResourceNotFound {
            resource_type: "sdf".to_string(),
            id: context.to_string(),
        })
    }
}

impl<T, E> SdfErrorContext<T> for Result<T, E> 
where 
    E: std::fmt::Display 
{
    fn sdf_context(self, context: &str) -> SdfResult<T> {
        self.map_err(|e| EngineError::SystemError {
            component: "sdf".to_string(),
            error: format!("{}: {}", context, e),
        })
    }
}