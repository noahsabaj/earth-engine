use std::sync::Arc;
use std::path::{Path, PathBuf};
use wgpu::{Device, Queue};
use crate::streaming::{
    PageTable, StreamPipeline, StreamRequest, RequestSource,
    create_page_table, MAX_WORLD_SIZE_X, MAX_WORLD_SIZE_Y, MAX_WORLD_SIZE_Z,
    PAGE_SIZE, MAX_RESIDENT_PAGES,
};
use crate::world_gpu::{WorldBuffer, WorldBufferDescriptor};

/// Streaming world buffer supporting planet-scale worlds
pub struct StreamingWorldBuffer {
    /// Base world buffer for resident pages
    world_buffer: WorldBuffer,
    
    /// Page table for virtual memory
    page_table: Arc<tokio::sync::RwLock<PageTable>>,
    
    /// Streaming pipeline
    stream_pipeline: Option<tokio::sync::mpsc::UnboundedSender<StreamRequest>>,
    
    /// World file path
    world_path: PathBuf,
    
    /// Actual world size in voxels
    world_size: (u32, u32, u32),
    
    /// Maximum GPU memory for world data
    max_gpu_memory: u64,
}

impl StreamingWorldBuffer {
    /// Create a new streaming world buffer
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        world_size: (u32, u32, u32),
        world_path: PathBuf,
        max_gpu_memory: u64,
    ) -> std::io::Result<Self> {
        // Validate world size
        assert!(world_size.0 <= MAX_WORLD_SIZE_X);
        assert!(world_size.1 <= MAX_WORLD_SIZE_Y);
        assert!(world_size.2 <= MAX_WORLD_SIZE_Z);
        
        // Create page table
        let page_table = create_page_table(world_size, PAGE_SIZE);
        let page_table = Arc::new(tokio::sync::RwLock::new(page_table));
        
        // Create base world buffer for resident pages
        let buffer_desc = WorldBufferDescriptor {
            world_size: MAX_RESIDENT_PAGES * PAGE_SIZE, // Sized for max resident pages
            world_height: PAGE_SIZE,
        };
        
        let world_buffer = WorldBuffer::new(&device, &buffer_desc);
        
        // Create streaming pipeline
        let page_table_clone = page_table.try_read()
            .map_err(|_| std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to acquire read lock on page table"
            ))?
            .clone();
        
        let pipeline = StreamPipeline::new(
            &world_path,
            device.clone(),
            queue.clone(),
            page_table_clone,
            max_gpu_memory,
        )?;
        
        let stream_pipeline = Some(pipeline.start());
        
        Ok(Self {
            world_buffer,
            page_table,
            stream_pipeline,
            world_path,
            world_size,
            max_gpu_memory,
        })
    }
    
    /// Request a page to be loaded
    pub fn request_page(&self, x: u32, y: u32, z: u32, priority: f32) {
        if let Some(pipeline) = &self.stream_pipeline {
            let request = StreamRequest {
                page_x: x / PAGE_SIZE,
                page_y: y / PAGE_SIZE,
                page_z: z / PAGE_SIZE,
                priority,
                source: RequestSource::Manual,
            };
            
            pipeline.send(request).ok();
        }
    }
    
    /// Update player position for predictive loading
    pub async fn update_player_position(
        &self,
        player_id: usize,
        position: (f32, f32, f32),
        timestamp: f64,
    ) {
        // This would integrate with the stream pipeline's predictive loader
        // For now, just request nearby pages
        let page_pos = (
            position.0 as u32 / PAGE_SIZE,
            position.1 as u32 / PAGE_SIZE,
            position.2 as u32 / PAGE_SIZE,
        );
        
        // Request pages in a radius around player
        let radius = 4u32;
        for dx in 0..=radius {
            for dy in 0..=radius {
                for dz in 0..=radius {
                    let dist_sq = dx * dx + dy * dy + dz * dz;
                    if dist_sq <= radius * radius {
                        let priority = 100.0 / (dist_sq as f32 + 1.0);
                        
                        // Request in all directions
                        for &sign_x in &[-1i32, 1] {
                            for &sign_y in &[-1i32, 1] {
                                for &sign_z in &[-1i32, 1] {
                                    let px = (page_pos.0 as i32 + sign_x * dx as i32).max(0) as u32;
                                    let py = (page_pos.1 as i32 + sign_y * dy as i32).max(0) as u32;
                                    let pz = (page_pos.2 as i32 + sign_z * dz as i32).max(0) as u32;
                                    
                                    self.request_page(
                                        px * PAGE_SIZE,
                                        py * PAGE_SIZE,
                                        pz * PAGE_SIZE,
                                        priority,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Get world statistics
    pub async fn get_stats(&self) -> WorldStats {
        let page_table = self.page_table.read().await;
        
        WorldStats {
            total_pages: page_table.total_pages,
            resident_pages: page_table.resident_pages,
            world_size_voxels: self.world_size,
            world_size_bytes: page_table.total_pages * PAGE_SIZE as u64 * PAGE_SIZE as u64 * PAGE_SIZE as u64 * 4,
            resident_bytes: page_table.resident_pages as u64 * PAGE_SIZE as u64 * PAGE_SIZE as u64 * PAGE_SIZE as u64 * 4,
        }
    }
    
    /// Get the underlying world buffer
    pub fn get_world_buffer(&self) -> &WorldBuffer {
        &self.world_buffer
    }
}

/// World statistics
#[derive(Debug, Clone)]
pub struct WorldStats {
    pub total_pages: u64,
    pub resident_pages: u32,
    pub world_size_voxels: (u32, u32, u32),
    pub world_size_bytes: u64,
    pub resident_bytes: u64,
}

/// Create a planet-scale world
pub fn create_planet_world(
    device: Arc<Device>,
    queue: Arc<Queue>,
    world_path: PathBuf,
    max_gpu_memory: u64,
) -> std::io::Result<StreamingWorldBuffer> {
    // Create a world with 1 billion voxels (1024x1024x1024)
    let world_size = (1024 * 1024, 1024, 1024 * 1024);
    
    StreamingWorldBuffer::new(
        device,
        queue,
        world_size,
        world_path,
        max_gpu_memory,
    )
}

/// Integration with existing WorldBuffer
impl StreamingWorldBuffer {
    /// Convert virtual voxel position to physical buffer offset
    pub async fn voxel_to_physical(&self, x: u32, y: u32, z: u32) -> Option<u64> {
        let page_table = self.page_table.read().await;
        
        let (page_x, page_y, page_z) = page_table.voxel_to_page(x, y, z);
        let page_idx = page_table.page_index(page_x, page_y, page_z)?;
        
        if page_idx >= page_table.entries.len() {
            return None;
        }
        
        let entry = &page_table.entries[page_idx];
        if !entry.is_resident() {
            return None;
        }
        
        let (local_x, local_y, local_z) = page_table.voxel_offset_in_page(x, y, z);
        let local_offset = local_x + local_y * PAGE_SIZE + local_z * PAGE_SIZE * PAGE_SIZE;
        
        Some(entry.physical_offset + local_offset as u64 * 4)
    }
    
    /// Check if a region is loaded
    pub async fn is_region_loaded(&self, min: (u32, u32, u32), max: (u32, u32, u32)) -> bool {
        let page_table = self.page_table.read().await;
        
        let min_page = page_table.voxel_to_page(min.0, min.1, min.2);
        let max_page = page_table.voxel_to_page(max.0, max.1, max.2);
        
        for px in min_page.0..=max_page.0 {
            for py in min_page.1..=max_page.1 {
                for pz in min_page.2..=max_page.2 {
                    if let Some(idx) = page_table.page_index(px, py, pz) {
                        if idx < page_table.entries.len() && !page_table.entries[idx].is_resident() {
                            return false;
                        }
                    }
                }
            }
        }
        
        true
    }
}

/// Shader integration for streaming worlds
pub fn create_streaming_bind_group_layout(device: &Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Streaming World Bind Group Layout"),
        entries: &[
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
            // World buffer segments (handled by GPU VM)
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}