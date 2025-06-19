//! Unified world and chunk management
//!
//! This module provides unified managers that can operate with either
//! GPU or CPU backends, presenting a consistent interface regardless
//! of the underlying implementation.

mod world_manager;
mod chunk_manager;
mod performance;

pub use world_manager::{UnifiedWorldManager, WorldManagerConfig, WorldError};
pub use chunk_manager::{UnifiedChunkManager, ChunkManagerConfig, ChunkManagerInterface, ChunkStats};
pub use performance::{WorldPerformanceMetrics, GenerationStats, PerformanceMonitor};

/// Backend selection for unified managers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// GPU-accelerated backend (primary)
    Gpu,
    /// CPU-based backend (fallback)
    Cpu,
    /// Automatic selection based on capabilities
    Auto,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Auto
    }
}

/// Capability requirements for backend selection
#[derive(Debug, Clone)]
pub struct BackendRequirements {
    pub requires_compute_shaders: bool,
    pub min_memory_mb: u64,
    pub max_latency_ms: u32,
    pub prefer_gpu: bool,
}

impl Default for BackendRequirements {
    fn default() -> Self {
        Self {
            requires_compute_shaders: false,
            min_memory_mb: 512,
            max_latency_ms: 16, // 60 FPS
            prefer_gpu: true,
        }
    }
}

/// Select the best backend based on requirements and capabilities
pub async fn select_backend(
    device: Option<&wgpu::Device>,
    requirements: &BackendRequirements,
) -> Backend {
    match device {
        Some(device) if requirements.prefer_gpu => {
            // Check GPU capabilities
            let limits = device.limits();
            let features = device.features();
            
            if features.contains(wgpu::Features::TIMESTAMP_QUERY) 
                && limits.max_compute_workgroup_size_x >= 256 {
                Backend::Gpu
            } else {
                log::warn!("GPU doesn't meet requirements, falling back to CPU");
                Backend::Cpu
            }
        }
        Some(_) => Backend::Gpu,
        None => Backend::Cpu,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_default() {
        assert_eq!(Backend::default(), Backend::Auto);
    }
    
    #[test]
    fn test_requirements_default() {
        let req = BackendRequirements::default();
        assert_eq!(req.min_memory_mb, 512);
        assert_eq!(req.max_latency_ms, 16);
        assert_eq!(req.prefer_gpu, true);
    }
    
    #[tokio::test]
    async fn test_backend_selection_no_gpu() {
        let backend = select_backend(None, &BackendRequirements::default()).await;
        assert_eq!(backend, Backend::Cpu);
    }
}