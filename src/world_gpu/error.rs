/// World GPU Error Handling
/// 
/// Provides error types and utilities for the GPU world subsystem.

use crate::error::{EngineError, EngineResult};

/// World GPU-specific result type
pub type WorldGpuResult<T> = EngineResult<T>;

/// Error context for world GPU operations
pub trait WorldGpuErrorContext<T> {
    fn world_gpu_context(self, context: &str) -> WorldGpuResult<T>;
}

impl<T> WorldGpuErrorContext<T> for Option<T> {
    fn world_gpu_context(self, context: &str) -> WorldGpuResult<T> {
        self.ok_or_else(|| EngineError::ResourceNotFound {
            resource_type: "world_gpu".to_string(),
            id: context.to_string(),
        })
    }
}

impl<T, E> WorldGpuErrorContext<T> for Result<T, E> 
where 
    E: std::fmt::Display 
{
    fn world_gpu_context(self, context: &str) -> WorldGpuResult<T> {
        self.map_err(|e| EngineError::SystemError {
            component: "world_gpu".to_string(),
            error: format!("{}: {}", context, e),
        })
    }
}