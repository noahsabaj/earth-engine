//! Unified storage systems - GPU WorldBuffer (primary) + CPU Chunks (fallback)
//!
//! This module consolidates both GPU-resident and CPU-based world storage,
//! providing a unified interface that can operate in either mode.

mod world_buffer;
mod cpu_chunks;
mod gpu_chunks;

// GPU-first storage (primary)
pub use world_buffer::{WorldBuffer, WorldBufferDescriptor, VoxelData};

// CPU fallback storage
pub use cpu_chunks::{ChunkSoA, ChunkMemoryStats, ChunkBatchOps};

// GPU chunk management
pub use gpu_chunks::{GpuChunk, GpuChunkManager, GpuChunkStats};

/// Unified storage backend that can operate in GPU or CPU mode
pub enum UnifiedStorage {
    /// GPU-resident storage (primary mode)
    Gpu {
        world_buffer: std::sync::Arc<std::sync::Mutex<WorldBuffer>>,
        device: std::sync::Arc<wgpu::Device>,
    },
    /// CPU-based storage (fallback mode)
    Cpu {
        chunks: std::collections::HashMap<crate::world_unified::core::ChunkPos, ChunkSoA>,
    },
}

impl UnifiedStorage {
    /// Create GPU-based storage
    pub async fn new_gpu(
        device: std::sync::Arc<wgpu::Device>,
        descriptor: &WorldBufferDescriptor,
    ) -> Result<Self, crate::world_unified::storage::StorageError> {
        let world_buffer = WorldBuffer::new(device.clone(), descriptor);
        Ok(UnifiedStorage::Gpu {
            world_buffer: std::sync::Arc::new(std::sync::Mutex::new(world_buffer)),
            device,
        })
    }
    
    /// Create CPU-based storage
    pub fn new_cpu() -> Self {
        UnifiedStorage::Cpu {
            chunks: std::collections::HashMap::new(),
        }
    }
    
    /// Check if this storage uses GPU backend
    pub fn is_gpu(&self) -> bool {
        matches!(self, UnifiedStorage::Gpu { .. })
    }
    
    /// Get GPU world buffer if in GPU mode
    pub fn gpu_world_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<WorldBuffer>>> {
        match self {
            UnifiedStorage::Gpu { world_buffer, .. } => Some(world_buffer.clone()),
            UnifiedStorage::Cpu { .. } => None,
        }
    }
}

/// Storage system errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("GPU initialization failed: {message}")]
    GpuInitFailed { message: String },
    
    #[error("Memory allocation failed: {size} bytes")]
    MemoryAllocationFailed { size: u64 },
    
    #[error("Invalid chunk position: {x}, {y}, {z}")]
    InvalidChunkPosition { x: i32, y: i32, z: i32 },
    
    #[error("Backend mismatch: operation requires {required} but storage is {actual}")]
    BackendMismatch { required: String, actual: String },
}