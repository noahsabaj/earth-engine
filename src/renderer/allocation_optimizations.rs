/// Memory allocation optimizations for zero-allocation rendering
/// This module provides optimized data structures and patterns to eliminate
/// allocations in hot paths during rendering and updates

use std::sync::Arc;
use parking_lot::RwLock;
use crate::{ChunkPos, BlockId};

/// Error type for allocation optimization operations
#[derive(Debug, thiserror::Error)]
pub enum AllocationError {
    #[error("Pooled object is in invalid state: {0}")]
    InvalidState(String),
    #[error("Buffer initialization failed: {0}")]
    InitializationFailed(String),
}

/// Object pool for reusable allocations
pub struct ObjectPool<T> {
    pool: Arc<RwLock<Vec<T>>>,
    factory: Arc<dyn Fn() -> T + Send + Sync>,
}

impl<T> ObjectPool<T> {
    pub fn new(initial_capacity: usize, factory: impl Fn() -> T + Send + Sync + 'static) -> Self {
        let mut pool = Vec::with_capacity(initial_capacity);
        for _ in 0..initial_capacity {
            pool.push(factory());
        }
        
        Self {
            pool: Arc::new(RwLock::new(pool)),
            factory: Arc::new(factory),
        }
    }
    
    pub fn acquire(&self) -> PooledObject<T> {
        let item = match self.pool.write().pop() {
            Some(item) => item,
            None => (self.factory)(),
        };
        PooledObject {
            item: Some(item),
            pool: Arc::clone(&self.pool),
        }
    }
}

/// RAII wrapper for pooled objects
pub struct PooledObject<T> {
    item: Option<T>,
    pool: Arc<RwLock<Vec<T>>>,
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        match self.item.as_ref() {
            Some(item) => item,
            None => {
                // This should never happen if the API is used correctly
                // Log error and provide a safe fallback that will panic with context
                panic!("PooledObject accessed after being consumed - this is a programmer error");
            }
        }
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.item.as_mut() {
            Some(item) => item,
            None => {
                // This should never happen if the API is used correctly
                panic!("PooledObject accessed after being consumed - this is a programmer error");
            }
        }
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(item) = self.item.take() {
            self.pool.write().push(item);
        }
    }
}

/// Pre-allocated buffer for greedy meshing masks
pub struct MeshingBuffers {
    /// 2D mask for face extraction - reused per face
    pub mask: Vec<Vec<Option<BlockId>>>,
    /// Used flags for rectangle extraction
    pub used: Vec<Vec<bool>>,
    // Removed quad storage - using GPU-driven approach
    /// Vertex buffer for mesh building
    pub vertices: Vec<crate::renderer::Vertex>,
    /// Index buffer for mesh building
    pub indices: Vec<u32>,
}

impl MeshingBuffers {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            mask: vec![vec![None; chunk_size]; chunk_size],
            used: vec![vec![false; chunk_size]; chunk_size],
            vertices: Vec::with_capacity(chunk_size * chunk_size * 24), // 4 verts per quad * 6 faces
            indices: Vec::with_capacity(chunk_size * chunk_size * 36), // 6 indices per quad * 6 faces
        }
    }
    
    pub fn clear(&mut self) {
        // Clear without deallocating
        for row in &mut self.mask {
            row.fill(None);
        }
        for row in &mut self.used {
            row.fill(false);
        }
        self.vertices.clear();
        self.indices.clear();
    }
}

// Thread-local meshing buffer pool
thread_local! {
    static MESHING_BUFFERS: std::cell::RefCell<Option<MeshingBuffers>> = std::cell::RefCell::new(None);
}

pub fn with_meshing_buffers<F, R>(chunk_size: usize, f: F) -> R
where
    F: FnOnce(&mut MeshingBuffers) -> R,
{
    MESHING_BUFFERS.with(|buffers| {
        let mut buffers_ref = buffers.borrow_mut();
        if buffers_ref.is_none() {
            *buffers_ref = Some(MeshingBuffers::new(chunk_size));
        }
        // This is safe because we just ensured the buffer exists
        let buffers = match buffers_ref.as_mut() {
            Some(buffers) => buffers,
            None => {
                panic!("Failed to initialize MeshingBuffers - this should not happen");
            }
        };
        buffers.clear();
        f(buffers)
    })
}

/// Pre-allocated string buffer for format operations
pub struct StringPool {
    pool: Arc<RwLock<Vec<String>>>,
}

impl StringPool {
    pub fn new(capacity: usize) -> Self {
        let mut pool = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            pool.push(String::with_capacity(128));
        }
        Self {
            pool: Arc::new(RwLock::new(pool)),
        }
    }
    
    pub fn acquire(&self) -> PooledString {
        let mut string = match self.pool.write().pop() {
            Some(string) => string,
            None => String::with_capacity(128),
        };
        string.clear();
        PooledString {
            string: Some(string),
            pool: Arc::clone(&self.pool),
        }
    }
}

pub struct PooledString {
    string: Option<String>,
    pool: Arc<RwLock<Vec<String>>>,
}

impl std::ops::Deref for PooledString {
    type Target = String;
    
    fn deref(&self) -> &Self::Target {
        match self.string.as_ref() {
            Some(string) => string,
            None => {
                panic!("PooledString accessed after being consumed - this is a programmer error");
            }
        }
    }
}

impl std::ops::DerefMut for PooledString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.string.as_mut() {
            Some(string) => string,
            None => {
                panic!("PooledString accessed after being consumed - this is a programmer error");
            }
        }
    }
}

impl Drop for PooledString {
    fn drop(&mut self) {
        if let Some(string) = self.string.take() {
            self.pool.write().push(string);
        }
    }
}

/// Static string formatting without allocation
pub struct StaticFormatter<const N: usize> {
    buffer: [u8; N],
    len: usize,
}

impl<const N: usize> StaticFormatter<N> {
    pub const fn new() -> Self {
        Self {
            buffer: [0; N],
            len: 0,
        }
    }
    
    pub fn format_chunk_label(&mut self, pos: ChunkPos) -> &str {
        use std::io::Write;
        self.len = 0;
        let mut cursor = std::io::Cursor::new(&mut self.buffer[..]);
        write!(&mut cursor, "Chunk ({}, {}, {})", pos.x, pos.y, pos.z).ok();
        self.len = cursor.position() as usize;
        std::str::from_utf8(&self.buffer[..self.len]).unwrap_or("")
    }
}

/// Pre-allocated collection for chunk positions
pub struct ChunkPositionBuffer {
    positions: Vec<ChunkPos>,
}

impl ChunkPositionBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
        }
    }
    
    pub fn clear(&mut self) {
        self.positions.clear();
    }
    
    pub fn push(&mut self, pos: ChunkPos) {
        self.positions.push(pos);
    }
    
    pub fn iter(&self) -> std::slice::Iter<ChunkPos> {
        self.positions.iter()
    }
}

/// Reusable mesh request buffer
pub struct MeshRequestBuffer {
    pub chunk_positions: Vec<ChunkPos>,
    pub priorities: Vec<i32>,
}

impl MeshRequestBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            chunk_positions: Vec::with_capacity(capacity),
            priorities: Vec::with_capacity(capacity),
        }
    }
    
    pub fn clear(&mut self) {
        self.chunk_positions.clear();
        self.priorities.clear();
    }
    
    pub fn push(&mut self, pos: ChunkPos, priority: i32) {
        self.chunk_positions.push(pos);
        self.priorities.push(priority);
    }
}

// Global pools for commonly allocated objects
lazy_static::lazy_static! {
    pub static ref STRING_POOL: StringPool = StringPool::new(64);
    pub static ref MESH_REQUEST_POOL: ObjectPool<MeshRequestBuffer> = 
        ObjectPool::new(16, || MeshRequestBuffer::with_capacity(256));
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_object_pool() {
        let pool = ObjectPool::new(2, || vec![0u8; 1024]);
        
        let mut obj1 = pool.acquire();
        obj1[0] = 1;
        
        let mut obj2 = pool.acquire();
        obj2[0] = 2;
        
        // Pool should be empty now
        let obj3 = pool.acquire(); // This will create a new object
        assert_eq!(obj3[0], 0); // New object should be initialized
        
        drop(obj1);
        drop(obj2);
        
        // Objects should be returned to pool
        let obj4 = pool.acquire();
        // Could be either obj1 or obj2
        assert!(obj4[0] == 1 || obj4[0] == 2);
    }
    
    #[test] 
    fn test_static_formatter() {
        let mut formatter = StaticFormatter::<64>::new();
        let pos = ChunkPos::new(10, -5, 20);
        let label = formatter.format_chunk_label(pos);
        assert_eq!(label, "Chunk (10, -5, 20)");
    }
}