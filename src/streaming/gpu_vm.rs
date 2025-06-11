use wgpu::{Buffer, BufferUsages, Device, Queue};
use crate::streaming::{PageTable, PageTableEntry, MortonPageTableGpuHeader as PageTableGpuHeader, MAX_RESIDENT_PAGES, PAGE_SIZE_BYTES};
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};

/// GPU page fault information
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuPageFault {
    /// Page coordinates that faulted
    pub page_x: u32,
    pub page_y: u32,
    pub page_z: u32,
    
    /// Type of access that caused fault
    pub access_type: u32, // 0=read, 1=write
    
    /// Shader stage that faulted
    pub shader_stage: u32,
    
    /// Priority for loading
    pub priority: u32,
    
    /// Padding for alignment
    pub _padding: [u32; 2],
}

/// GPU virtual memory manager
pub struct GpuVirtualMemory {
    /// GPU-side page table buffer
    page_table_buffer: Buffer,
    
    /// GPU-side page fault buffer
    page_fault_buffer: Buffer,
    
    /// Readback buffer for page faults
    fault_readback_buffer: Buffer,
    
    /// WorldBuffer segments (actual voxel data)
    world_buffer_segments: Vec<Buffer>,
    
    /// Free page list for allocation
    free_pages: Vec<u32>,
    
    /// Device reference
    device: Arc<Device>,
    
    /// Queue reference
    queue: Arc<Queue>,
    
    /// Total GPU memory allocated
    total_gpu_memory: u64,
    
    /// Maximum GPU memory to use
    max_gpu_memory: u64,
}

impl GpuVirtualMemory {
    /// Create new GPU virtual memory system
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        page_table: &PageTable,
        max_gpu_memory: u64,
    ) -> Self {
        // Create GPU page table buffer
        let page_table_size = std::mem::size_of::<PageTableEntry>() * page_table.entries.len();
        let page_table_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Page Table"),
            size: page_table_size as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Upload initial page table
        queue.write_buffer(&page_table_buffer, 0, bytemuck::cast_slice(&page_table.entries));
        
        // Create page fault buffer (for GPU to report faults)
        let page_fault_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Page Fault Buffer"),
            size: std::mem::size_of::<GpuPageFault>() as u64 * 1024, // Up to 1024 faults
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create readback buffer for page faults
        let fault_readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Page Fault Readback"),
            size: std::mem::size_of::<GpuPageFault>() as u64 * 1024,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Allocate WorldBuffer segments
        let segment_size = PAGE_SIZE_BYTES * 1024; // 1024 pages per segment
        let num_segments = (max_gpu_memory / segment_size).min(16) as usize;
        let mut world_buffer_segments = Vec::with_capacity(num_segments);
        
        for i in 0..num_segments {
            world_buffer_segments.push(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("WorldBuffer Segment {}", i)),
                size: segment_size,
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
        }
        
        // Initialize free page list
        let total_pages = (num_segments as u64 * segment_size / PAGE_SIZE_BYTES) as u32;
        let free_pages: Vec<u32> = (0..total_pages).collect();
        
        Self {
            page_table_buffer,
            page_fault_buffer,
            fault_readback_buffer,
            world_buffer_segments,
            free_pages,
            device,
            queue,
            total_gpu_memory: num_segments as u64 * segment_size,
            max_gpu_memory,
        }
    }
    
    /// Allocate a GPU page
    pub fn allocate_page(&mut self) -> Option<(u32, u64)> {
        let page_index = self.free_pages.pop()?;
        let segment_index = (page_index as u64 * PAGE_SIZE_BYTES / (PAGE_SIZE_BYTES * 1024)) as usize;
        let offset_in_segment = (page_index as u64 * PAGE_SIZE_BYTES) % (PAGE_SIZE_BYTES * 1024);
        
        let physical_offset = segment_index as u64 * PAGE_SIZE_BYTES * 1024 + offset_in_segment;
        
        Some((page_index, physical_offset))
    }
    
    /// Free a GPU page
    pub fn free_page(&mut self, page_index: u32) {
        self.free_pages.push(page_index);
    }
    
    /// Upload page data to GPU
    pub fn upload_page(
        &self,
        physical_offset: u64,
        data: &[u8],
    ) {
        let segment_index = (physical_offset / (PAGE_SIZE_BYTES * 1024)) as usize;
        let offset_in_segment = physical_offset % (PAGE_SIZE_BYTES * 1024);
        
        if segment_index < self.world_buffer_segments.len() {
            self.queue.write_buffer(
                &self.world_buffer_segments[segment_index],
                offset_in_segment,
                data,
            );
        }
    }
    
    /// Update page table entry on GPU
    pub fn update_page_table_entry(
        &self,
        index: usize,
        entry: &PageTableEntry,
    ) {
        let offset = index * std::mem::size_of::<PageTableEntry>();
        self.queue.write_buffer(
            &self.page_table_buffer,
            offset as u64,
            bytemuck::bytes_of(entry),
        );
    }
    
    /// Read page faults from GPU
    pub async fn read_page_faults(&self) -> Vec<GpuPageFault> {
        // Copy fault buffer to readback buffer
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Page Fault Readback"),
        });
        
        encoder.copy_buffer_to_buffer(
            &self.page_fault_buffer,
            0,
            &self.fault_readback_buffer,
            0,
            self.fault_readback_buffer.size(),
        );
        
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // Map and read faults
        let buffer_slice = self.fault_readback_buffer.slice(..);
        let (tx, rx) = flume::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).ok();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv_async().await.ok();
        
        let data = buffer_slice.get_mapped_range();
        let faults: Vec<GpuPageFault> = bytemuck::cast_slice(&data).to_vec();
        
        drop(data);
        self.fault_readback_buffer.unmap();
        
        // Filter out invalid faults (where page coords are u32::MAX)
        faults.into_iter()
            .filter(|f| f.page_x != u32::MAX)
            .collect()
    }
    
    /// Get bind group entries for shaders
    pub fn get_bind_group_entries(&self) -> Vec<wgpu::BindGroupEntry> {
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: self.page_table_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.page_fault_buffer.as_entire_binding(),
            },
        ];
        
        // Add world buffer segments
        for (i, segment) in self.world_buffer_segments.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: (2 + i) as u32,
                resource: segment.as_entire_binding(),
            });
        }
        
        entries
    }
    
    /// Get memory statistics
    pub fn get_stats(&self) -> GpuVmStats {
        GpuVmStats {
            total_memory: self.total_gpu_memory,
            free_pages: self.free_pages.len() as u32,
            allocated_pages: MAX_RESIDENT_PAGES - self.free_pages.len() as u32,
        }
    }
}

/// GPU VM statistics
#[derive(Debug, Clone)]
pub struct GpuVmStats {
    pub total_memory: u64,
    pub free_pages: u32,
    pub allocated_pages: u32,
}

/// Create bind group layout for GPU VM
pub fn create_gpu_vm_bind_group_layout(device: &Device) -> wgpu::BindGroupLayout {
    let mut entries = vec![
        // Page table
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Page fault buffer
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ];
    
    // World buffer segments (up to 16)
    for i in 0..16 {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 2 + i,
            visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
    }
    
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("GPU VM Bind Group Layout"),
        entries: &entries,
    })
}

/// Page eviction candidate
#[derive(Debug)]
pub struct EvictionCandidate {
    pub page_index: usize,
    pub score: f32, // Lower score = better candidate for eviction
}

/// Calculate eviction candidates based on various metrics
pub fn calculate_eviction_candidates(
    page_table: &PageTable,
    camera_pos: (f32, f32, f32),
    max_candidates: usize,
) -> Vec<EvictionCandidate> {
    let mut candidates = Vec::new();
    
    for (index, entry) in page_table.entries.iter().enumerate() {
        if !entry.is_resident() || entry.is_locked() {
            continue;
        }
        
        // Calculate page center position
        let page_coords = index_to_page_coords(index, &page_table.world_size_pages);
        let page_center = (
            (page_coords.0 as f32 + 0.5) * page_table.page_size as f32,
            (page_coords.1 as f32 + 0.5) * page_table.page_size as f32,
            (page_coords.2 as f32 + 0.5) * page_table.page_size as f32,
        );
        
        // Calculate distance from camera
        let distance = distance_squared(camera_pos, page_center).sqrt();
        
        // Calculate eviction score (lower = better candidate)
        let mut score = 0.0;
        
        // Distance factor (farther = higher score)
        score += distance;
        
        // Access count factor (less accessed = higher score)
        score += 1000.0 / (entry.access_count as f32 + 1.0);
        
        // Dirty factor (dirty pages have lower score to avoid write-back)
        if entry.is_dirty() {
            score *= 0.5;
        }
        
        candidates.push(EvictionCandidate {
            page_index: index,
            score,
        });
    }
    
    // Sort by score (highest first) and take top candidates
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(max_candidates);
    
    candidates
}

fn index_to_page_coords(index: usize, world_size_pages: &(u32, u32, u32)) -> (u32, u32, u32) {
    let x = index as u32 % world_size_pages.0;
    let y = (index as u32 / world_size_pages.0) % world_size_pages.1;
    let z = index as u32 / (world_size_pages.0 * world_size_pages.1);
    (x, y, z)
}

fn distance_squared(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    dx * dx + dy * dy + dz * dz
}