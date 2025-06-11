/// Memory Pool Implementation
/// 
/// Provides efficient GPU memory allocation with recycling and defragmentation.

use std::sync::{Arc, Mutex};
use std::collections::{HashMap, BTreeMap};
use wgpu::{Device, Buffer};
use super::{MemoryResult, MemoryErrorContext, allocation_error};

/// Allocation strategy for the memory pool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    /// First block that fits
    FirstFit,
    /// Best fitting block (least waste)
    BestFit,
    /// Largest block first (reduces fragmentation)
    WorstFit,
}

/// Handle to an allocated buffer in the pool
#[derive(Clone)]
pub struct PoolHandle {
    pool_id: u64,
    buffer: Arc<Buffer>,
    offset: u64,
    size: u64,
}

impl PoolHandle {
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    
    pub fn buffer_arc(&self) -> Arc<Buffer> {
        self.buffer.clone()
    }
    
    pub fn offset(&self) -> u64 {
        self.offset
    }
    
    pub fn size(&self) -> u64 {
        self.size
    }
}

/// Memory block within a pool
struct MemoryBlock {
    offset: u64,
    size: u64,
    free: bool,
}

/// Large buffer that contains many allocations
struct PoolBuffer {
    buffer: Arc<Buffer>,
    capacity: u64,
    blocks: Vec<MemoryBlock>,
    free_space: u64,
}

/// GPU memory pool for efficient allocation
pub struct MemoryPool {
    device: Arc<Device>,
    
    /// All pool buffers
    buffers: Mutex<Vec<PoolBuffer>>,
    
    /// Total capacity across all buffers
    total_capacity: u64,
    
    /// Allocation strategy
    strategy: AllocationStrategy,
    
    /// Next pool ID
    next_id: Mutex<u64>,
    
    /// Active allocations
    allocations: Mutex<HashMap<u64, (usize, usize)>>, // pool_id -> (buffer_index, block_index)
}

impl MemoryPool {
    pub fn new(device: Arc<Device>, initial_capacity: u64) -> Self {
        Self {
            device,
            buffers: Mutex::new(Vec::new()),
            total_capacity: initial_capacity,
            strategy: AllocationStrategy::BestFit,
            next_id: Mutex::new(0),
            allocations: Mutex::new(HashMap::new()),
        }
    }
    
    /// Allocate memory from the pool
    pub fn allocate(&self, size: u64, usage: wgpu::BufferUsages) -> MemoryResult<PoolHandle> {
        let mut buffers = self.buffers.lock().memory_context("buffers")?;
        
        // Try to find space in existing buffers
        for (buffer_idx, pool_buffer) in buffers.iter_mut().enumerate() {
            if let Some(block_idx) = self.find_free_block(pool_buffer, size) {
                // Mark block as used
                pool_buffer.blocks[block_idx].free = false;
                pool_buffer.free_space -= size;
                
                let pool_id = self.get_next_id()?;
                self.allocations.lock().memory_context("allocations")?.insert(pool_id, (buffer_idx, block_idx));
                
                return Ok(PoolHandle {
                    pool_id,
                    buffer: pool_buffer.buffer.clone(),
                    offset: pool_buffer.blocks[block_idx].offset,
                    size,
                });
            }
        }
        
        // No space found, create new buffer
        let new_capacity = (size * 2).max(64 * 1024 * 1024); // At least 64MB
        let new_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Memory Pool Buffer"),
            size: new_capacity,
            usage,
            mapped_at_creation: false,
        });
        
        let mut pool_buffer = PoolBuffer {
            buffer: Arc::new(new_buffer),
            capacity: new_capacity,
            blocks: vec![MemoryBlock {
                offset: 0,
                size,
                free: false,
            }],
            free_space: new_capacity - size,
        };
        
        // Add remaining space as free block
        if size < new_capacity {
            pool_buffer.blocks.push(MemoryBlock {
                offset: size,
                size: new_capacity - size,
                free: true,
            });
        }
        
        let buffer_idx = buffers.len();
        let pool_id = self.get_next_id()?;
        
        self.allocations.lock().memory_context("allocations")?.insert(pool_id, (buffer_idx, 0));
        
        let handle = PoolHandle {
            pool_id,
            buffer: pool_buffer.buffer.clone(),
            offset: 0,
            size,
        };
        
        buffers.push(pool_buffer);
        
        Ok(handle)
    }
    
    /// Free an allocation
    pub fn free(&self, handle: PoolHandle) -> MemoryResult<()> {
        let mut allocations = self.allocations.lock().memory_context("allocations")?;
        
        if let Some((buffer_idx, block_idx)) = allocations.remove(&handle.pool_id) {
            let mut buffers = self.buffers.lock().memory_context("buffers")?;
            if let Some(pool_buffer) = buffers.get_mut(buffer_idx) {
                pool_buffer.blocks[block_idx].free = true;
                pool_buffer.free_space += handle.size;
                
                // Attempt to merge adjacent free blocks
                self.merge_free_blocks(pool_buffer);
            }
        }
        Ok(())
    }
    
    /// Find a free block using the configured strategy
    fn find_free_block(&self, pool_buffer: &mut PoolBuffer, size: u64) -> Option<usize> {
        let mut candidates: Vec<(usize, u64)> = pool_buffer.blocks.iter()
            .enumerate()
            .filter(|(_, block)| block.free && block.size >= size)
            .map(|(idx, block)| (idx, block.size))
            .collect();
        
        if candidates.is_empty() {
            return None;
        }
        
        match self.strategy {
            AllocationStrategy::FirstFit => Some(candidates[0].0),
            AllocationStrategy::BestFit => {
                candidates.sort_by_key(|(_, block_size)| *block_size);
                Some(candidates[0].0)
            }
            AllocationStrategy::WorstFit => {
                candidates.sort_by_key(|(_, block_size)| std::cmp::Reverse(*block_size));
                Some(candidates[0].0)
            }
        }
    }
    
    /// Merge adjacent free blocks to reduce fragmentation
    fn merge_free_blocks(&self, pool_buffer: &mut PoolBuffer) {
        let mut i = 0;
        while i < pool_buffer.blocks.len() - 1 {
            if pool_buffer.blocks[i].free && pool_buffer.blocks[i + 1].free {
                // Merge blocks
                pool_buffer.blocks[i].size += pool_buffer.blocks[i + 1].size;
                pool_buffer.blocks.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
    
    /// Get next unique pool ID
    fn get_next_id(&self) -> MemoryResult<u64> {
        let mut next_id = self.next_id.lock().memory_context("next_id")?;
        let id = *next_id;
        *next_id += 1;
        Ok(id)
    }
    
    /// Get total allocated bytes
    pub fn allocated_bytes(&self) -> u64 {
        self.buffers.lock()
            .ok()
            .map(|buffers| buffers.iter().map(|b| b.capacity).sum())
            .unwrap_or(0)
    }
    
    /// Get total used bytes
    pub fn used_bytes(&self) -> u64 {
        self.buffers.lock()
            .ok()
            .map(|buffers| buffers.iter().map(|b| b.capacity - b.free_space).sum())
            .unwrap_or(0)
    }
    
    /// Defragment the pool (expensive operation)
    pub fn defragment(&self) {
        // In a real implementation, this would:
        // 1. Create new compacted buffers
        // 2. Copy all active allocations
        // 3. Update all handles
        // 4. Free old buffers
        // For now, just a placeholder
    }
}

/// Specialized pool for small, uniform allocations
pub struct UniformPool {
    device: Arc<Device>,
    block_size: u64,
    free_blocks: Mutex<Vec<PoolHandle>>,
    active_blocks: Mutex<HashMap<u64, PoolHandle>>,
}

impl UniformPool {
    pub fn new(device: Arc<Device>, block_size: u64) -> Self {
        Self {
            device,
            block_size,
            free_blocks: Mutex::new(Vec::new()),
            active_blocks: Mutex::new(HashMap::new()),
        }
    }
    
    /// Allocate a uniform block
    pub fn allocate(&self) -> MemoryResult<PoolHandle> {
        // Try to reuse a free block
        if let Some(handle) = self.free_blocks.lock().memory_context("free_blocks")?.pop() {
            self.active_blocks.lock().memory_context("active_blocks")?.insert(handle.pool_id, handle.clone());
            return Ok(handle);
        }
        
        // Create new block
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Block"),
            size: self.block_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let handle = PoolHandle {
            pool_id: rand::random(),
            buffer: Arc::new(buffer),
            offset: 0,
            size: self.block_size,
        };
        
        self.active_blocks.lock().memory_context("active_blocks")?.insert(handle.pool_id, handle.clone());
        Ok(handle)
    }
    
    /// Free a uniform block
    pub fn free(&self, handle: PoolHandle) -> MemoryResult<()> {
        if self.active_blocks.lock().memory_context("active_blocks")?.remove(&handle.pool_id).is_some() {
            self.free_blocks.lock().memory_context("free_blocks")?.push(handle);
        }
        Ok(())
    }
}