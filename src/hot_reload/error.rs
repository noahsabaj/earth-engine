//! Hot reload error handling
//! 
//! This module provides type aliases and helper functions for hot reload operations
//! to replace unwrap() calls with proper error handling.

use crate::error::{EngineError, EngineResult};

/// Type alias for hot reload results
pub type HotReloadResult<T> = EngineResult<T>;

/// Helper trait for hot reload error contexts
pub trait HotReloadErrorContext<T> {
    fn hot_reload_context(self, context: &str) -> HotReloadResult<T>
    where
        Self: Sized;
}


impl<T, E> HotReloadErrorContext<T> for Result<T, E>
where
    E: Into<EngineError>,
{
    fn hot_reload_context(self, context: &str) -> HotReloadResult<T> {
        self.map_err(|e| {
            let base_error = e.into();
            match base_error {
                EngineError::LockPoisoned { .. } => EngineError::LockPoisoned {
                    resource: format!("hot_reload::{}", context),
                },
                other => other,
            }
        })
    }
}

/// Create an asset reload error
pub fn asset_reload_error(asset: &str, error: impl std::fmt::Display) -> EngineError {
    EngineError::AssetWatchError {
        path: asset.to_string(),
        error: error.to_string(),
    }
}

/// Create a shader reload error
pub fn shader_reload_error(shader: &str, error: impl std::fmt::Display) -> EngineError {
    EngineError::ShaderReloadFailed {
        name: shader.to_string(),
        error: error.to_string(),
    }
}