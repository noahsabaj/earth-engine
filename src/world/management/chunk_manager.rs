//! Unified chunk management interface

use crate::world::core::{ChunkPos, VoxelPos, BlockId};
use crate::world::storage::ChunkSoA;

/// Unified chunk manager interface
pub trait ChunkManagerInterface: Send + Sync {
    /// Load a chunk at the specified position
    fn load_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), ChunkManagerError>;
    
    /// Unload a chunk at the specified position
    fn unload_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), ChunkManagerError>;
    
    /// Check if a chunk is loaded
    fn is_chunk_loaded(&self, chunk_pos: ChunkPos) -> bool;
    
    /// Get all loaded chunk positions
    fn loaded_chunks(&self) -> Vec<ChunkPos>;
    
    /// Get chunk statistics
    fn chunk_stats(&self) -> ChunkStats;
}

/// Unified chunk manager that works with both GPU and CPU backends
pub struct UnifiedChunkManager {
    backend: ChunkManagerBackend,
    config: ChunkManagerConfig,
}

impl UnifiedChunkManager {
    /// Create a new GPU-based chunk manager
    pub fn new_gpu(
        device: std::sync::Arc<wgpu::Device>,
        config: ChunkManagerConfig,
    ) -> Result<Self, ChunkManagerError> {
        let backend = ChunkManagerBackend::Gpu {
            device,
            loaded_chunks: std::collections::HashMap::new(),
        };
        
        Ok(Self { backend, config })
    }
    
    /// Create a new CPU-based chunk manager
    pub fn new_cpu(config: ChunkManagerConfig) -> Self {
        let backend = ChunkManagerBackend::Cpu {
            chunks: std::collections::HashMap::new(),
        };
        
        Self { backend, config }
    }
    
    /// Check if using GPU backend
    pub fn is_gpu(&self) -> bool {
        matches!(self.backend, ChunkManagerBackend::Gpu { .. })
    }
}

impl ChunkManagerInterface for UnifiedChunkManager {
    fn load_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), ChunkManagerError> {
        match &mut self.backend {
            ChunkManagerBackend::Gpu { loaded_chunks, .. } => {
                if !loaded_chunks.contains_key(&chunk_pos) {
                    // TODO: Implement GPU chunk loading
                    loaded_chunks.insert(chunk_pos, GpuChunkHandle::new(chunk_pos));
                }
                Ok(())
            }
            ChunkManagerBackend::Cpu { chunks } => {
                if !chunks.contains_key(&chunk_pos) {
                    // Create empty chunk for now
                    let chunk = ChunkSoA::new(chunk_pos, self.config.chunk_size);
                    chunks.insert(chunk_pos, chunk);
                }
                Ok(())
            }
        }
    }
    
    fn unload_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), ChunkManagerError> {
        match &mut self.backend {
            ChunkManagerBackend::Gpu { loaded_chunks, .. } => {
                loaded_chunks.remove(&chunk_pos);
                Ok(())
            }
            ChunkManagerBackend::Cpu { chunks } => {
                chunks.remove(&chunk_pos);
                Ok(())
            }
        }
    }
    
    fn is_chunk_loaded(&self, chunk_pos: ChunkPos) -> bool {
        match &self.backend {
            ChunkManagerBackend::Gpu { loaded_chunks, .. } => {
                loaded_chunks.contains_key(&chunk_pos)
            }
            ChunkManagerBackend::Cpu { chunks } => {
                chunks.contains_key(&chunk_pos)
            }
        }
    }
    
    fn loaded_chunks(&self) -> Vec<ChunkPos> {
        match &self.backend {
            ChunkManagerBackend::Gpu { loaded_chunks, .. } => {
                loaded_chunks.keys().copied().collect()
            }
            ChunkManagerBackend::Cpu { chunks } => {
                chunks.keys().copied().collect()
            }
        }
    }
    
    fn chunk_stats(&self) -> ChunkStats {
        match &self.backend {
            ChunkManagerBackend::Gpu { loaded_chunks, .. } => {
                ChunkStats {
                    loaded_count: loaded_chunks.len(),
                    memory_usage_mb: loaded_chunks.len() as f64 * 0.5, // Estimate
                    backend: "GPU".to_string(),
                }
            }
            ChunkManagerBackend::Cpu { chunks } => {
                ChunkStats {
                    loaded_count: chunks.len(),
                    memory_usage_mb: chunks.len() as f64 * 2.0, // Estimate
                    backend: "CPU".to_string(),
                }
            }
        }
    }
}

/// Chunk manager backend implementation
enum ChunkManagerBackend {
    Gpu {
        device: std::sync::Arc<wgpu::Device>,
        loaded_chunks: std::collections::HashMap<ChunkPos, GpuChunkHandle>,
    },
    Cpu {
        chunks: std::collections::HashMap<ChunkPos, ChunkSoA>,
    },
}

/// Handle to a GPU-resident chunk
#[derive(Debug)]
struct GpuChunkHandle {
    chunk_pos: ChunkPos,
    // TODO: Add GPU buffer handles
}

impl GpuChunkHandle {
    fn new(chunk_pos: ChunkPos) -> Self {
        Self { chunk_pos }
    }
}

/// Configuration for chunk manager
#[derive(Debug, Clone)]
pub struct ChunkManagerConfig {
    pub chunk_size: u32,
    pub max_loaded_chunks: usize,
    pub unload_distance: u32,
    pub memory_limit_mb: u64,
}

impl Default for ChunkManagerConfig {
    fn default() -> Self {
        Self {
            chunk_size: 32,
            max_loaded_chunks: 1024,
            unload_distance: 12,
            memory_limit_mb: 512,
        }
    }
}

/// Chunk statistics
#[derive(Debug, Clone)]
pub struct ChunkStats {
    pub loaded_count: usize,
    pub memory_usage_mb: f64,
    pub backend: String,
}

/// Chunk manager errors
#[derive(Debug, thiserror::Error)]
pub enum ChunkManagerError {
    #[error("Chunk loading failed at position {x}, {y}, {z}: {message}")]
    LoadingFailed { x: i32, y: i32, z: i32, message: String },
    
    #[error("Memory limit exceeded: {current_mb}MB > {limit_mb}MB")]
    MemoryLimitExceeded { current_mb: u64, limit_mb: u64 },
    
    #[error("Invalid chunk position: {x}, {y}, {z}")]
    InvalidPosition { x: i32, y: i32, z: i32 },
    
    #[error("Backend error: {message}")]
    BackendError { message: String },
}