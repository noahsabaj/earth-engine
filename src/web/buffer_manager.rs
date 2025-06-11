use wgpu::{Buffer, BufferUsages, Device};
use std::collections::{HashMap, VecDeque};
use crate::web::WebError;

/// Handle to a managed buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(u64);

/// Buffer allocation info
struct BufferAllocation {
    buffer: Buffer,
    size: u64,
    usage: BufferUsages,
    last_used: f64,
    ref_count: u32,
}

/// Memory pool for efficient buffer reuse
struct BufferPool {
    /// Available buffers by size
    available: HashMap<u64, VecDeque<Buffer>>,
    
    /// Maximum buffers per size
    max_buffers_per_size: usize,
}

impl BufferPool {
    fn new() -> Self {
        Self {
            available: HashMap::new(),
            max_buffers_per_size: 10,
        }
    }
    
    /// Get a buffer from the pool or None if not available
    fn get(&mut self, size: u64, usage: BufferUsages) -> Option<Buffer> {
        self.available.get_mut(&size)
            .and_then(|buffers| buffers.pop_front())
    }
    
    /// Return a buffer to the pool
    fn return_buffer(&mut self, buffer: Buffer, size: u64) {
        let buffers = self.available.entry(size).or_insert_with(VecDeque::new);
        
        if buffers.len() < self.max_buffers_per_size {
            buffers.push_back(buffer);
        }
        // Otherwise, let the buffer be dropped
    }
    
    /// Clear old buffers from the pool
    fn clear_old_buffers(&mut self, age_threshold: f64) {
        // In a real implementation, track buffer ages
        // For now, just limit pool size
        for buffers in self.available.values_mut() {
            while buffers.len() > self.max_buffers_per_size / 2 {
                buffers.pop_front();
            }
        }
    }
}

/// Browser-optimized buffer manager with memory pooling
pub struct BufferManager {
    /// Device reference
    device: Device,
    
    /// Active allocations
    allocations: HashMap<BufferHandle, BufferAllocation>,
    
    /// Buffer pools by usage type
    pools: HashMap<BufferUsages, BufferPool>,
    
    /// Next handle ID
    next_handle: u64,
    
    /// Total allocated memory
    total_allocated: u64,
    
    /// Memory limit
    memory_limit: u64,
    
    /// Performance tracking
    allocation_count: u32,
    reuse_count: u32,
}

impl BufferManager {
    /// Create a new buffer manager
    pub fn new(device: Device, memory_limit: u64) -> Self {
        log::info!("Creating BufferManager with {} MB limit", memory_limit / 1024 / 1024);
        
        Self {
            device,
            allocations: HashMap::new(),
            pools: HashMap::new(),
            next_handle: 1,
            total_allocated: 0,
            memory_limit,
            allocation_count: 0,
            reuse_count: 0,
        }
    }
    
    /// Allocate a new buffer
    pub fn allocate(
        &mut self,
        label: &str,
        size: u64,
        usage: BufferUsages,
    ) -> Result<BufferHandle, WebError> {
        // Check memory limit
        if self.total_allocated + size > self.memory_limit {
            self.garbage_collect();
            
            if self.total_allocated + size > self.memory_limit {
                return Err(WebError::BufferError(format!(
                    "Memory limit exceeded: {} + {} > {}",
                    self.total_allocated, size, self.memory_limit
                )));
            }
        }
        
        // Try to get buffer from pool
        let buffer = if let Some(buffer) = self.pools
            .entry(usage)
            .or_insert_with(BufferPool::new)
            .get(size, usage) {
            
            self.reuse_count += 1;
            log::debug!("Reusing buffer from pool for {}", label);
            buffer
        } else {
            // Allocate new buffer
            self.allocation_count += 1;
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        
        // Create allocation
        let handle = BufferHandle(self.next_handle);
        self.next_handle += 1;
        
        let allocation = BufferAllocation {
            buffer,
            size,
            usage,
            last_used: get_current_time(),
            ref_count: 1,
        };
        
        self.allocations.insert(handle, allocation);
        self.total_allocated += size;
        
        Ok(handle)
    }
    
    /// Get a buffer by handle
    pub fn get(&mut self, handle: BufferHandle) -> Option<&Buffer> {
        self.allocations.get_mut(&handle).map(|alloc| {
            alloc.last_used = get_current_time();
            &alloc.buffer
        })
    }
    
    /// Increase reference count
    pub fn retain(&mut self, handle: BufferHandle) {
        if let Some(alloc) = self.allocations.get_mut(&handle) {
            alloc.ref_count += 1;
        }
    }
    
    /// Decrease reference count and potentially free
    pub fn release(&mut self, handle: BufferHandle) {
        let should_free = if let Some(alloc) = self.allocations.get_mut(&handle) {
            alloc.ref_count = alloc.ref_count.saturating_sub(1);
            alloc.ref_count == 0
        } else {
            false
        };
        
        if should_free {
            self.free_buffer(handle);
        }
    }
    
    /// Free a buffer immediately
    fn free_buffer(&mut self, handle: BufferHandle) {
        if let Some(alloc) = self.allocations.remove(&handle) {
            self.total_allocated -= alloc.size;
            
            // Return to pool
            self.pools
                .entry(alloc.usage)
                .or_insert_with(BufferPool::new)
                .return_buffer(alloc.buffer, alloc.size);
        }
    }
    
    /// Garbage collect unused buffers
    pub fn garbage_collect(&mut self) {
        let current_time = get_current_time();
        let age_threshold = 30.0; // 30 seconds
        
        // Find buffers to free
        let to_free: Vec<BufferHandle> = self.allocations
            .iter()
            .filter(|(_, alloc)| {
                alloc.ref_count == 0 && 
                current_time - alloc.last_used > age_threshold
            })
            .map(|(handle, _)| *handle)
            .collect();
        
        // Free old buffers
        for handle in to_free {
            self.free_buffer(handle);
        }
        
        // Clean up pools
        for pool in self.pools.values_mut() {
            pool.clear_old_buffers(age_threshold);
        }
        
        log::info!("GC: Freed {} buffers, {} MB now allocated",
            to_free.len(),
            self.total_allocated / 1024 / 1024
        );
    }
    
    /// Get memory statistics
    pub fn get_stats(&self) -> BufferManagerStats {
        BufferManagerStats {
            total_allocated: self.total_allocated,
            allocation_count: self.allocation_count,
            reuse_count: self.reuse_count,
            active_buffers: self.allocations.len() as u32,
            pooled_buffers: self.pools.values()
                .map(|p| p.available.values().map(|v| v.len()).sum::<usize>())
                .sum::<usize>() as u32,
        }
    }
    
    /// Create a staging buffer for uploads
    pub fn create_staging_buffer(
        &mut self,
        data: &[u8],
    ) -> Result<BufferHandle, WebError> {
        use wgpu::util::DeviceExt;
        
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging Buffer"),
            contents: data,
            usage: BufferUsages::COPY_SRC,
        });
        
        let handle = BufferHandle(self.next_handle);
        self.next_handle += 1;
        
        let allocation = BufferAllocation {
            buffer,
            size: data.len() as u64,
            usage: BufferUsages::COPY_SRC,
            last_used: get_current_time(),
            ref_count: 1,
        };
        
        self.allocations.insert(handle, allocation);
        self.total_allocated += data.len() as u64;
        
        Ok(handle)
    }
    
    /// Batch allocate multiple buffers
    pub fn batch_allocate(
        &mut self,
        requests: &[(String, u64, BufferUsages)],
    ) -> Result<Vec<BufferHandle>, WebError> {
        let mut handles = Vec::with_capacity(requests.len());
        
        for (label, size, usage) in requests {
            handles.push(self.allocate(label, *size, *usage)?);
        }
        
        Ok(handles)
    }
}

/// Buffer manager statistics
#[derive(Debug, Clone)]
pub struct BufferManagerStats {
    pub total_allocated: u64,
    pub allocation_count: u32,
    pub reuse_count: u32,
    pub active_buffers: u32,
    pub pooled_buffers: u32,
}

/// Get current time in seconds (browser)
fn get_current_time() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now() / 1000.0)
        .unwrap_or(0.0)
}

/// Suballocator for small allocations within larger buffers
pub struct SubAllocator {
    /// Parent buffer handle
    buffer: BufferHandle,
    
    /// Buffer size
    size: u64,
    
    /// Free regions
    free_regions: Vec<(u64, u64)>, // (offset, size)
    
    /// Allocated regions
    allocations: HashMap<u64, (u64, u64)>, // id -> (offset, size)
    
    /// Next allocation ID
    next_id: u64,
}

impl SubAllocator {
    /// Create a new suballocator
    pub fn new(buffer: BufferHandle, size: u64) -> Self {
        Self {
            buffer,
            size,
            free_regions: vec![(0, size)],
            allocations: HashMap::new(),
            next_id: 1,
        }
    }
    
    /// Allocate a region
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Option<(u64, u64)> {
        // Find first fit
        for i in 0..self.free_regions.len() {
            let (offset, region_size) = match self.free_regions.get(i) {
                Some(&region) => region,
                None => continue,
            };
            
            // Align offset
            let aligned_offset = (offset + alignment - 1) / alignment * alignment;
            let padding = aligned_offset - offset;
            
            if region_size >= size + padding {
                // Remove this free region
                self.free_regions.remove(i);
                
                // Add back unused parts
                if padding > 0 {
                    self.free_regions.push((offset, padding));
                }
                
                let remaining = region_size - size - padding;
                if remaining > 0 {
                    self.free_regions.push((aligned_offset + size, remaining));
                }
                
                // Track allocation
                let id = self.next_id;
                self.next_id += 1;
                self.allocations.insert(id, (aligned_offset, size));
                
                return Some((id, aligned_offset));
            }
        }
        
        None
    }
    
    /// Free a region
    pub fn free(&mut self, id: u64) {
        if let Some((offset, size)) = self.allocations.remove(&id) {
            // Add back to free list and merge adjacent regions
            self.free_regions.push((offset, size));
            self.merge_free_regions();
        }
    }
    
    /// Merge adjacent free regions
    fn merge_free_regions(&mut self) {
        self.free_regions.sort_by_key(|(offset, _)| *offset);
        
        let mut merged = Vec::new();
        let mut current = match self.free_regions.get(0) {
            Some(&region) => region,
            None => return,
        };
        
        for i in 1..self.free_regions.len() {
            let next = match self.free_regions.get(i) {
                Some(&region) => region,
                None => continue,
            };
            
            if current.0 + current.1 == next.0 {
                // Adjacent, merge
                current.1 += next.1;
            } else {
                // Not adjacent, keep separate
                merged.push(current);
                current = next;
            }
        }
        
        merged.push(current);
        self.free_regions = merged;
    }
    
    /// Get fragmentation ratio
    pub fn fragmentation(&self) -> f32 {
        let total_free: u64 = self.free_regions.iter().map(|(_, size)| size).sum();
        let largest_free = self.free_regions.iter().map(|(_, size)| size).max().unwrap_or(&0);
        
        if total_free == 0 {
            0.0
        } else {
            1.0 - (*largest_free as f32 / total_free as f32)
        }
    }
}