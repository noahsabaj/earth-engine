/// Persistent Mapped Buffer Implementation
/// 
/// Provides buffers that stay mapped for efficient CPU-GPU communication
/// with proper synchronization and multi-frame buffering.

use std::sync::{Arc, Mutex};
use wgpu::{Device, Buffer};
use crate::memory::memory_pool::PoolHandle;
use crate::memory::{MemoryResult, MemoryErrorContext};
use crate::error::EngineError;

/// Buffer usage patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    /// Uniform data updated per frame
    Uniform,
    /// Vertex data that changes frequently
    DynamicVertex,
    /// Instance data for dynamic objects
    DynamicInstance,
    /// Staging for texture uploads
    TextureStaging,
    /// General staging buffer
    Staging,
}

impl BufferUsage {
    pub fn to_wgpu_usage(&self) -> wgpu::BufferUsages {
        match self {
            BufferUsage::Uniform => wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            BufferUsage::DynamicVertex => wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            BufferUsage::DynamicInstance => wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            BufferUsage::TextureStaging => wgpu::BufferUsages::COPY_SRC,
            BufferUsage::Staging => wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        }
    }
}

/// A persistently mapped buffer with multi-frame support
pub struct PersistentBuffer {
    device: Arc<Device>,
    
    /// Current frame buffer index
    current_frame: usize,
    
    /// All frame buffers (for multi-buffering)
    frame_buffers: Vec<FrameBuffer>,
    
    /// Buffer size
    size: u64,
    
    /// Usage pattern
    usage: BufferUsage,
}

/// Single frame's buffer data
struct FrameBuffer {
    /// Pool handle for memory management
    handle: PoolHandle,
    
    /// Whether this buffer is currently mapped
    is_mapped: Mutex<bool>,
    
    /// Fence for synchronization
    fence: Option<u64>,
}

impl PersistentBuffer {
    pub fn new(
        device: Arc<Device>,
        handle: PoolHandle,
        size: u64,
        usage: BufferUsage,
        frame_count: usize,
    ) -> Self {
        let mut frame_buffers = Vec::with_capacity(frame_count);
        
        // First buffer uses the provided handle
        frame_buffers.push(FrameBuffer {
            handle,
            is_mapped: Mutex::new(false),
            fence: None,
        });
        
        // Create additional buffers for multi-buffering
        for _ in 1..frame_count {
            // In a real implementation, we'd allocate from the pool
            // For now, using placeholder
            frame_buffers.push(FrameBuffer {
                handle: handle.clone(), // Placeholder - should allocate new
                    is_mapped: Mutex::new(false),
                fence: None,
            });
        }
        
        Self {
            device,
            current_frame: 0,
            frame_buffers,
            size,
            usage,
        }
    }
    
    /// Get the current frame's buffer
    pub fn current_buffer(&self) -> &Buffer {
        &self.frame_buffers[self.current_frame].handle.buffer()
    }
    
    /// Map the buffer for writing
    pub async fn map_write(&self) -> MemoryResult<MappedBuffer> {
        let frame = &self.frame_buffers[self.current_frame];
        
        // Check if already mapped
        if *frame.is_mapped.lock().memory_context("is_mapped")? {
            return Err(EngineError::BufferError {
                buffer: "persistent".to_string(),
                error: "Buffer already mapped".to_string(),
            });
        }
        
        let buffer = frame.handle.buffer();
        let buffer_slice = buffer.slice(..);
        
        // Request mapping
        let (tx, rx) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Write, move |result| {
            let _ = tx.send(result);
        });
        
        // Wait for mapping
        self.device.poll(wgpu::Maintain::Wait);
        rx.await.map_err(|_| EngineError::BufferError {
            buffer: "persistent".to_string(),
            error: "Channel closed while waiting for buffer map".to_string(),
        })?.map_err(|e| EngineError::BufferError {
            buffer: "persistent".to_string(),
            error: format!("Failed to map buffer: {:?}", e),
        })?;
        
        // Get mapped view
        let view = buffer_slice.get_mapped_range_mut();
        
        *frame.is_mapped.lock().memory_context("is_mapped")? = true;
        
        Ok(MappedBuffer {
            buffer: self,
            view,
            frame_index: self.current_frame,
        })
    }
    
    /// Advance to next frame buffer
    pub fn advance_frame(&mut self) -> MemoryResult<()> {
        // Unmap current buffer if mapped
        let frame = &self.frame_buffers[self.current_frame];
        if *frame.is_mapped.lock().memory_context("is_mapped")? {
            frame.handle.buffer().unmap();
            *frame.is_mapped.lock().memory_context("is_mapped")? = false;
        }
        
        // Move to next frame
        self.current_frame = (self.current_frame + 1) % self.frame_buffers.len();
        Ok(())
    }
    
    /// Get buffer size
    pub fn size(&self) -> u64 {
        self.size
    }
    
    /// Get usage pattern
    pub fn usage(&self) -> BufferUsage {
        self.usage
    }
}

/// A mapped buffer view for writing
pub struct MappedBuffer<'a> {
    buffer: &'a PersistentBuffer,
    view: wgpu::BufferViewMut<'a>,
    frame_index: usize,
}

impl<'a> MappedBuffer<'a> {
    /// Write data to the mapped buffer
    pub fn write(&mut self, offset: u64, data: &[u8]) -> MemoryResult<()> {
        if offset + data.len() as u64 > self.buffer.size {
            return Err(EngineError::BufferAccess {
                index: offset as usize + data.len(),
                size: self.buffer.size as usize,
            });
        }
        
        self.view[offset as usize..offset as usize + data.len()].copy_from_slice(data);
        Ok(())
    }
    
    /// Get mutable slice of the buffer
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.view[..]
    }
}

impl<'a> Drop for MappedBuffer<'a> {
    fn drop(&mut self) {
        // Buffer will be unmapped when advancing frames
    }
}

/// Ring buffer optimized for streaming data
pub struct StreamingRingBuffer {
    persistent_buffer: PersistentBuffer,
    write_offset: u64,
    capacity: u64,
}

impl StreamingRingBuffer {
    pub fn new(device: Arc<Device>, handle: PoolHandle, capacity: u64) -> Self {
        let persistent_buffer = PersistentBuffer::new(
            device,
            handle,
            capacity,
            BufferUsage::Staging,
            3, // Triple buffering
        );
        
        Self {
            persistent_buffer,
            write_offset: 0,
            capacity,
        }
    }
    
    /// Write data to the ring buffer
    pub async fn write(&mut self, data: &[u8]) -> MemoryResult<u64> {
        let data_size = data.len() as u64;
        
        if data_size > self.capacity {
            return Err(EngineError::BufferError {
                buffer: "ring".to_string(),
                error: "Data too large for ring buffer".to_string(),
            });
        }
        
        // Wrap around if needed
        if self.write_offset + data_size > self.capacity {
            self.write_offset = 0;
        }
        
        let offset = self.write_offset;
        
        // Map and write
        let mut mapped = self.persistent_buffer.map_write().await?;
        mapped.write(offset, data)?;
        drop(mapped);
        
        self.write_offset = (self.write_offset + data_size) % self.capacity;
        
        Ok(offset)
    }
    
    /// Get the underlying buffer
    pub fn buffer(&self) -> &Buffer {
        self.persistent_buffer.current_buffer()
    }
    
    /// Advance to next frame
    pub fn advance_frame(&mut self) -> MemoryResult<()> {
        self.persistent_buffer.advance_frame()
    }
}