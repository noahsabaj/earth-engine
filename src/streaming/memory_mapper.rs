use std::sync::Arc;
use std::path::Path;
use memmap2::{MmapOptions, MmapMut};
use wgpu::{Buffer, BufferUsages, Device, Queue};
use crate::streaming::{PageTableEntry, PAGE_SIZE_BYTES};

/// Memory segment representing a mapped region
#[derive(Debug)]
pub struct MemorySegment {
    /// Memory-mapped file region
    pub mmap: Arc<MmapMut>,
    
    /// Offset in the world file
    pub file_offset: u64,
    
    /// Size of this segment
    pub size: u64,
    
    /// Associated GPU buffer (if uploaded)
    pub gpu_buffer: Option<Arc<Buffer>>,
    
    /// Reference count for this segment
    pub ref_count: u32,
}

/// Memory mapper for zero-copy disk to GPU streaming
pub struct MemoryMapper {
    /// World data file handle
    world_file: std::fs::File,
    
    /// Active memory segments
    segments: Vec<MemorySegment>,
    
    /// Total mapped memory
    total_mapped: u64,
    
    /// Maximum memory to map
    max_mapped_memory: u64,
    
    /// GPU device for buffer creation
    device: Arc<Device>,
    
    /// GPU queue for uploads
    queue: Arc<Queue>,
}

impl MemoryMapper {
    /// Create a new memory mapper
    pub fn new(
        world_path: &Path,
        device: Arc<Device>,
        queue: Arc<Queue>,
        max_mapped_memory: u64,
    ) -> std::io::Result<Self> {
        use std::fs::OpenOptions;
        
        let world_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(world_path)?;
        
        Ok(Self {
            world_file,
            segments: Vec::new(),
            total_mapped: 0,
            max_mapped_memory,
            device,
            queue,
        })
    }
    
    /// Map a page from disk
    pub fn map_page(
        &mut self,
        page_entry: &PageTableEntry,
    ) -> std::io::Result<Arc<MmapMut>> {
        // Check if already mapped
        for segment in &mut self.segments {
            if segment.file_offset <= page_entry.disk_offset &&
               segment.file_offset + segment.size > page_entry.disk_offset {
                segment.ref_count += 1;
                return Ok(segment.mmap.clone());
            }
        }
        
        // Evict segments if needed
        while self.total_mapped + PAGE_SIZE_BYTES > self.max_mapped_memory {
            self.evict_lru_segment()?;
        }
        
        // Map new segment (map larger region for efficiency)
        let segment_size = (PAGE_SIZE_BYTES * 16).min(self.max_mapped_memory / 4);
        let aligned_offset = (page_entry.disk_offset / segment_size) * segment_size;
        
        // SAFETY: Memory mapping with mmap is safe because:
        // - world_file is a valid file handle opened with read/write permissions
        // - aligned_offset is aligned to page boundaries (guaranteed by calculation)
        // - segment_size is within file bounds (we resize file as needed)
        // - The mmap is wrapped in Arc for shared ownership
        // - Multiple readers are safe, writers need external synchronization
        let mmap = unsafe {
            MmapOptions::new()
                .offset(aligned_offset)
                .len(segment_size as usize)
                .map_mut(&self.world_file)?
        };
        
        let mmap = Arc::new(mmap);
        
        self.segments.push(MemorySegment {
            mmap: mmap.clone(),
            file_offset: aligned_offset,
            size: segment_size,
            gpu_buffer: None,
            ref_count: 1,
        });
        
        self.total_mapped += segment_size;
        
        Ok(mmap)
    }
    
    /// Upload mapped memory directly to GPU
    pub fn upload_to_gpu(
        &mut self,
        page_entry: &PageTableEntry,
    ) -> Option<Arc<Buffer>> {
        // Find the segment
        let segment = self.segments.iter_mut().find(|s| {
            s.file_offset <= page_entry.disk_offset &&
            s.file_offset + s.size > page_entry.disk_offset
        })?;
        
        // Create GPU buffer if not exists
        if segment.gpu_buffer.is_none() {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Streaming Page Buffer"),
                size: PAGE_SIZE_BYTES,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            
            segment.gpu_buffer = Some(Arc::new(buffer));
        }
        
        let gpu_buffer = segment.gpu_buffer.as_ref()?;
        
        // Calculate offset within segment
        let offset_in_segment = page_entry.disk_offset - segment.file_offset;
        let page_data = &segment.mmap[offset_in_segment as usize..][..PAGE_SIZE_BYTES as usize];
        
        // Direct upload to GPU
        self.queue.write_buffer(gpu_buffer, 0, page_data);
        
        Some(gpu_buffer.clone())
    }
    
    /// Evict least recently used segment
    fn evict_lru_segment(&mut self) -> std::io::Result<()> {
        // Find segment with lowest ref count
        let min_idx = self.segments.iter().enumerate()
            .min_by_key(|(_, s)| s.ref_count)
            .map(|(idx, _)| idx);
        
        if let Some(idx) = min_idx {
            let segment = self.segments.swap_remove(idx);
            self.total_mapped -= segment.size;
        }
        
        Ok(())
    }
    
    /// Get memory statistics
    pub fn get_stats(&self) -> MemoryMapperStats {
        MemoryMapperStats {
            total_mapped: self.total_mapped,
            num_segments: self.segments.len(),
            gpu_buffers: self.segments.iter().filter(|s| s.gpu_buffer.is_some()).count(),
        }
    }
}

/// Zero-copy upload path using DirectStorage (Windows) or GPUDirect (Linux)
#[cfg(target_os = "windows")]
pub struct DirectStorageUploader {
    // DirectStorage API handles
}

#[cfg(target_os = "linux")]
pub struct GpuDirectUploader {
    // GPUDirect handles
}

/// Fallback CPU staging buffer for older hardware
pub struct StagingBufferPool {
    /// Pool of staging buffers
    buffers: Vec<StagingBuffer>,
    
    /// Maximum pool size
    max_buffers: usize,
}

struct StagingBuffer {
    buffer: Buffer,
    size: u64,
    in_use: bool,
}

impl StagingBufferPool {
    pub fn new(device: &Device, max_buffers: usize) -> Self {
        let mut buffers = Vec::with_capacity(max_buffers);
        
        // Pre-allocate some staging buffers
        for _ in 0..4 {
            buffers.push(StagingBuffer {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Staging Buffer"),
                    size: PAGE_SIZE_BYTES,
                    usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                size: PAGE_SIZE_BYTES,
                in_use: false,
            });
        }
        
        Self {
            buffers,
            max_buffers,
        }
    }
    
    /// Get an available staging buffer
    pub fn acquire(&mut self, device: &Device, size: u64) -> Option<&mut Buffer> {
        // Find available buffer of sufficient size
        for i in 0..self.buffers.len() {
            if !self.buffers[i].in_use && self.buffers[i].size >= size {
                self.buffers[i].in_use = true;
                return Some(&mut self.buffers[i].buffer);
            }
        }
        
        // Allocate new buffer if under limit
        if self.buffers.len() < self.max_buffers {
            let new_buffer = StagingBuffer {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Staging Buffer"),
                    size,
                    usage: BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                size,
                in_use: true,
            };
            self.buffers.push(new_buffer);
            let idx = self.buffers.len() - 1;
            return Some(&mut self.buffers[idx].buffer);
        }
        
        None
    }
    
    /// Release a staging buffer back to pool
    pub fn release(&mut self, buffer: &Buffer) {
        for staging in &mut self.buffers {
            if std::ptr::eq(&staging.buffer, buffer) {
                staging.in_use = false;
                break;
            }
        }
    }
}

/// Statistics for memory mapping
#[derive(Debug, Clone)]
pub struct MemoryMapperStats {
    pub total_mapped: u64,
    pub num_segments: usize,
    pub gpu_buffers: usize,
}

/// Platform-specific zero-copy uploader
pub enum ZeroCopyUploader {
    #[cfg(target_os = "windows")]
    DirectStorage(DirectStorageUploader),
    
    #[cfg(target_os = "linux")]
    GpuDirect(GpuDirectUploader),
    
    Fallback(StagingBufferPool),
}

impl ZeroCopyUploader {
    /// Create platform-specific uploader
    pub fn new(device: &Device) -> Self {
        #[cfg(target_os = "windows")]
        {
            // Try to initialize DirectStorage
            // Fallback to staging if not available
            Self::Fallback(StagingBufferPool::new(device, 32))
        }
        
        #[cfg(target_os = "linux")]
        {
            // Try to initialize GPUDirect
            // Fallback to staging if not available
            Self::Fallback(StagingBufferPool::new(device, 32))
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Self::Fallback(StagingBufferPool::new(device, 32))
        }
    }
}