//! Pure data structures for allocation optimizations - NO METHODS!
//! All operations are in renderer_operations.rs

use crate::renderer::renderer_data::{
    ObjectPoolData, PooledObjectData, MeshingBuffersData,
    StringPoolData, PooledStringData, StaticFormatterData,
    ChunkPositionBufferData, MeshRequestBufferData
};

/// Error type for allocation optimization operations
#[derive(Debug, thiserror::Error)]
pub enum AllocationError {
    #[error("Pooled object is in invalid state: {0}")]
    InvalidState(String),
    #[error("Buffer initialization failed: {0}")]
    InitializationFailed(String),
}

// Type aliases for clarity
pub type ObjectPool<T> = ObjectPoolData<T>;
pub type PooledObject<T> = PooledObjectData<T>;
pub type MeshingBuffers = MeshingBuffersData;
pub type StringPool = StringPoolData;
pub type PooledString = PooledStringData;
pub type StaticFormatter<const N: usize> = StaticFormatterData<N>;
pub type ChunkPositionBuffer = ChunkPositionBufferData;
pub type MeshRequestBuffer = MeshRequestBufferData;

// Thread-local meshing buffer storage
thread_local! {
    pub static MESHING_BUFFERS: std::cell::RefCell<Option<MeshingBuffersData>> = std::cell::RefCell::new(None);
}

// Re-export commonly used types
pub use crate::{BlockId, ChunkPos};