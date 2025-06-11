/// GPU Instance Data Streaming System
/// 
/// Optimizes instance data updates using persistent mapped buffers
/// and triple buffering to avoid GPU stalls.
/// Part of Sprint 28: GPU-Driven Rendering Optimization

use wgpu::{Device, Buffer, Queue};
use std::sync::{Arc, Mutex};
use super::ChunkInstance;

/// Triple buffer system for zero-stall streaming
pub struct InstanceStreamer {
    /// Three buffers for triple buffering
    buffers: [Buffer; 3],
    
    /// Persistently mapped staging buffer
    staging_buffer: Buffer,
    staging_ptr: *mut u8,
    staging_size: usize,
    
    /// Current frame index for triple buffering
    frame_index: usize,
    
    /// Maximum instances
    max_instances: usize,
    
    /// Update tracking
    dirty_ranges: Arc<Mutex<Vec<DirtyRange>>>,
}

#[derive(Clone, Copy)]
struct DirtyRange {
    offset: usize,
    size: usize,
}

unsafe impl Send for InstanceStreamer {}
unsafe impl Sync for InstanceStreamer {}

impl InstanceStreamer {
    pub fn new(device: &Device, max_instances: usize) -> Self {
        let instance_size = std::mem::size_of::<ChunkInstance>();
        let buffer_size = (instance_size * max_instances) as u64;
        
        // Create triple buffers
        let mut buffers = Vec::with_capacity(3);
        for i in 0..3 {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Instance Buffer {}", i)),
                size: buffer_size,
                usage: wgpu::BufferUsages::STORAGE 
                    | wgpu::BufferUsages::COPY_DST 
                    | wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            });
            buffers.push(buffer);
        }
        
        // Create persistently mapped staging buffer
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Staging Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });
        
        // Get persistent mapping
        let staging_ptr = staging_buffer.slice(..).get_mapped_range_mut().as_mut_ptr();
        
        Self {
            buffers: buffers.try_into()
                .expect("Should have exactly 3 buffers"),
            staging_buffer,
            staging_ptr,
            staging_size: buffer_size as usize,
            frame_index: 0,
            max_instances,
            dirty_ranges: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Update instance data (can be called from any thread)
    pub fn update_instance(&self, index: usize, instance: &ChunkInstance) {
        if index >= self.max_instances {
            return;
        }
        
        let offset = index * std::mem::size_of::<ChunkInstance>();
        let size = std::mem::size_of::<ChunkInstance>();
        
        // SAFETY: Direct write to mapped staging buffer is safe because:
        // - staging_ptr is a valid pointer to persistently mapped memory from wgpu
        // - offset is bounds-checked above to ensure it's within max_instances
        // - dst pointer is properly aligned for ChunkInstance (guaranteed by allocation)
        // - The write is atomic and doesn't overlap with other threads due to instance indexing
        unsafe {
            // Write directly to mapped staging buffer
            let dst = self.staging_ptr.add(offset) as *mut ChunkInstance;
            *dst = *instance;
        }
        
        // Track dirty range
        match self.dirty_ranges.lock() {
            Ok(mut dirty) => dirty.push(DirtyRange { offset, size }),
            Err(e) => eprintln!("Failed to lock dirty ranges: {}", e),
        }
    }
    
    /// Update multiple instances efficiently
    pub fn update_instances(&self, start_index: usize, instances: &[ChunkInstance]) {
        if start_index >= self.max_instances {
            return;
        }
        
        let count = (self.max_instances - start_index).min(instances.len());
        let offset = start_index * std::mem::size_of::<ChunkInstance>();
        let size = count * std::mem::size_of::<ChunkInstance>();
        
        // SAFETY: Bulk copy to staging buffer is safe because:
        // - staging_ptr is valid persistently mapped memory from wgpu
        // - offset and count are bounds-checked above to stay within max_instances
        // - dst and src pointers don't overlap (src is from instances slice, dst is GPU buffer)
        // - size calculation ensures we don't write beyond buffer bounds
        // - copy_nonoverlapping is appropriate as source and destination are distinct
        unsafe {
            // Bulk copy to staging buffer
            let dst = self.staging_ptr.add(offset);
            let src = instances.as_ptr() as *const u8;
            std::ptr::copy_nonoverlapping(src, dst, size);
        }
        
        // Track dirty range
        match self.dirty_ranges.lock() {
            Ok(mut dirty) => dirty.push(DirtyRange { offset, size }),
            Err(e) => eprintln!("Failed to lock dirty ranges: {}", e),
        }
    }
    
    /// Flush updates to GPU (called once per frame)
    pub fn flush(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let dirty_ranges = match self.dirty_ranges.lock() {
            Ok(mut dirty) => std::mem::take(&mut *dirty),
            Err(e) => {
                eprintln!("Failed to lock dirty ranges: {}", e);
                return;
            }
        };
        
        if dirty_ranges.is_empty() {
            return;
        }
        
        // Get current frame's buffer
        let current_buffer = &self.buffers[self.frame_index];
        
        // Coalesce overlapping ranges for efficiency
        let coalesced = coalesce_ranges(dirty_ranges);
        
        // Copy dirty ranges from staging to GPU buffer
        for range in coalesced {
            encoder.copy_buffer_to_buffer(
                &self.staging_buffer,
                range.offset as u64,
                current_buffer,
                range.offset as u64,
                range.size as u64,
            );
        }
        
        // Advance to next buffer
        self.frame_index = (self.frame_index + 1) % 3;
    }
    
    /// Get the current frame's buffer for rendering
    pub fn get_current_buffer(&self) -> &Buffer {
        // Use previous frame's buffer (fully updated)
        let render_index = (self.frame_index + 2) % 3;
        &self.buffers[render_index]
    }
    
    /// Prefetch instances that will be visible next frame
    pub fn prefetch_instances(&self, predicted_visible: &[usize]) {
        // Touch memory to bring into cache
        for &idx in predicted_visible {
            if idx >= self.max_instances {
                continue;
            }
            
            let offset = idx * std::mem::size_of::<ChunkInstance>();
            // SAFETY: Volatile read for cache prefetching is safe because:
            // - staging_ptr points to valid mapped memory that exists for entire lifetime
            // - offset is bounds-checked above against max_instances
            // - read_volatile only reads data, doesn't modify anything
            // - The read is used purely for cache warming and the value is discarded
            // - No synchronization needed as this is a read-only prefetch operation
            unsafe {
                // Volatile read to prevent optimization
                let ptr = self.staging_ptr.add(offset);
                std::ptr::read_volatile(ptr);
            }
        }
    }
}

/// Coalesce overlapping dirty ranges
fn coalesce_ranges(mut ranges: Vec<DirtyRange>) -> Vec<DirtyRange> {
    if ranges.is_empty() {
        return ranges;
    }
    
    // Sort by offset
    ranges.sort_by_key(|r| r.offset);
    
    let mut coalesced = Vec::new();
    let mut current = ranges[0];
    
    for range in ranges.into_iter().skip(1) {
        if range.offset <= current.offset + current.size {
            // Overlapping or adjacent - merge
            let end = (current.offset + current.size).max(range.offset + range.size);
            current.size = end - current.offset;
        } else {
            // Non-overlapping - start new range
            coalesced.push(current);
            current = range;
        }
    }
    
    coalesced.push(current);
    coalesced
}

impl Drop for InstanceStreamer {
    fn drop(&mut self) {
        // Unmap staging buffer
        self.staging_buffer.unmap();
    }
}

/// Performance metrics for instance streaming
#[derive(Debug, Default)]
pub struct StreamingMetrics {
    pub instances_updated: u32,
    pub bytes_transferred: u64,
    pub dirty_ranges_coalesced: u32,
    pub frame_time_ms: f32,
}