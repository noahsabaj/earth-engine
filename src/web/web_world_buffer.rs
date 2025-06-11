use wgpu::{Buffer, BufferUsages, util::DeviceExt};
use crate::web::{WebGpuContext, WebError};
use bytemuck::{Pod, Zeroable};

// Re-define types for web module to avoid circular dependencies
pub const CHUNK_SIZE: u32 = 32;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct VoxelData(pub u32);

impl VoxelData {
    pub fn new(block_id: u16, light: u8, sky_light: u8, metadata: u8) -> Self {
        let packed = (block_id as u32) 
            | ((light as u32 & 0xF) << 16)
            | ((sky_light as u32 & 0xF) << 20)
            | ((metadata as u32 & 0xF) << 24);
        Self(packed)
    }
}

pub struct WorldBufferDescriptor {
    pub world_size: u32,
    pub world_height: u32,
}

/// Web-optimized version of WorldBuffer that leverages browser memory architecture
pub struct WebWorldBuffer {
    /// Main voxel data buffer
    pub voxel_buffer: Buffer,
    
    /// Metadata buffer for chunks
    pub metadata_buffer: Buffer,
    
    /// World dimensions
    pub world_size: u32,
    pub world_height: u32,
    
    /// Total buffer sizes for memory tracking
    voxel_buffer_size: u64,
    metadata_buffer_size: u64,
    
    /// Browser-specific optimizations
    supports_buffer_mapping: bool,
    supports_shared_memory: bool,
}

impl WebWorldBuffer {
    /// Create a new web world buffer
    pub fn new(context: &WebGpuContext) -> Result<Self, WebError> {
        let desc = WorldBufferDescriptor {
            world_size: 256,  // Smaller default for web
            world_height: 128, // Reduced height for web performance
        };
        
        Self::with_descriptor(context, &desc)
    }
    
    /// Create with custom descriptor
    pub fn with_descriptor(
        context: &WebGpuContext,
        desc: &WorldBufferDescriptor,
    ) -> Result<Self, WebError> {
        log::info!("Creating WebWorldBuffer: {}x{} world", desc.world_size, desc.world_height);
        
        // Calculate buffer sizes
        let chunks_per_dimension = desc.world_size / CHUNK_SIZE;
        let chunks_per_height = desc.world_height / CHUNK_SIZE;
        let total_chunks = chunks_per_dimension * chunks_per_dimension * chunks_per_height;
        let voxels_per_chunk = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
        
        // Each voxel is 4 bytes (u32)
        let voxel_buffer_size = (total_chunks * voxels_per_chunk * 4) as u64;
        
        // Each chunk has 16 bytes of metadata
        let metadata_buffer_size = (total_chunks * 16) as u64;
        
        // Check if buffers fit within browser limits
        let limits = context.limits();
        if voxel_buffer_size > limits.max_buffer_size {
            return Err(WebError::BufferError(format!(
                "Voxel buffer size {} exceeds browser limit {}",
                voxel_buffer_size, limits.max_buffer_size
            )));
        }
        
        log::info!("Allocating {} MB for voxels", voxel_buffer_size / 1024 / 1024);
        log::info!("Allocating {} KB for metadata", metadata_buffer_size / 1024);
        
        // Create voxel buffer
        let voxel_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Web World Voxel Buffer"),
            size: voxel_buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create metadata buffer
        let metadata_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Web World Metadata Buffer"),
            size: metadata_buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Check browser capabilities
        let supports_buffer_mapping = true; // WebGPU supports buffer mapping
        let supports_shared_memory = check_shared_memory_support();
        
        Ok(Self {
            voxel_buffer,
            metadata_buffer,
            world_size: desc.world_size,
            world_height: desc.world_height,
            voxel_buffer_size,
            metadata_buffer_size,
            supports_buffer_mapping,
            supports_shared_memory,
        })
    }
    
    /// Get total GPU memory usage
    pub fn gpu_memory_usage(&self) -> u64 {
        self.voxel_buffer_size + self.metadata_buffer_size
    }
    
    /// Get the number of loaded chunks (estimated based on buffer size)
    pub fn get_loaded_chunk_count(&self) -> u32 {
        // For now, return a reasonable estimate based on world size
        let chunks_per_dimension = self.world_size / CHUNK_SIZE;
        let chunks_per_height = self.world_height / CHUNK_SIZE;
        // Assume about 1/4 of chunks are loaded in browser
        (chunks_per_dimension * chunks_per_dimension * chunks_per_height) / 4
    }
    
    /// Clear all chunks (placeholder for future implementation)
    pub fn clear_chunks(&self) {
        // In a real implementation, this would clear chunk data
        // For now, just log the action
        log::info!("Clearing chunks in WebWorldBuffer");
    }
    
    /// Create bind group entries for compute shaders
    pub fn create_bind_group_entries(&self) -> Vec<wgpu::BindGroupEntry> {
        vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: self.voxel_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.metadata_buffer.as_entire_binding(),
            },
        ]
    }
    
    /// Get chunk index from position
    pub fn chunk_index(&self, chunk_x: u32, chunk_y: u32, chunk_z: u32) -> u32 {
        let chunks_per_dimension = self.world_size / CHUNK_SIZE;
        chunk_x + chunk_y * chunks_per_dimension + chunk_z * chunks_per_dimension * chunks_per_dimension
    }
    
    /// Calculate voxel offset within the buffer
    pub fn voxel_offset(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<u64> {
        // Bounds check
        if world_x < 0 || world_y < 0 || world_z < 0 ||
           world_x >= self.world_size as i32 ||
           world_y >= self.world_height as i32 ||
           world_z >= self.world_size as i32 {
            return None;
        }
        
        let chunk_x = (world_x as u32) / CHUNK_SIZE;
        let chunk_y = (world_y as u32) / CHUNK_SIZE;
        let chunk_z = (world_z as u32) / CHUNK_SIZE;
        
        let local_x = (world_x as u32) % CHUNK_SIZE;
        let local_y = (world_y as u32) % CHUNK_SIZE;
        let local_z = (world_z as u32) % CHUNK_SIZE;
        
        let chunk_index = self.chunk_index(chunk_x, chunk_y, chunk_z);
        let voxels_per_chunk = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
        let chunk_offset = (chunk_index * voxels_per_chunk) as u64;
        
        let local_index = local_x + local_y * CHUNK_SIZE + local_z * CHUNK_SIZE * CHUNK_SIZE;
        let voxel_offset = chunk_offset + local_index as u64;
        
        Some(voxel_offset * 4) // 4 bytes per voxel
    }
    
    /// Upload voxel data using browser-optimized path
    pub async fn upload_voxels_async(
        &self,
        context: &WebGpuContext,
        offset: u64,
        data: &[VoxelData],
    ) -> Result<(), WebError> {
        // For large uploads, use staging buffer
        if data.len() > 1024 {
            self.upload_voxels_staged(context, offset, data)
        } else {
            // For small uploads, use direct write
            let bytes: Vec<u8> = data.iter()
                .flat_map(|v| v.0.to_le_bytes())
                .collect();
            context.queue.write_buffer(&self.voxel_buffer, offset, &bytes);
            Ok(())
        }
    }
    
    /// Staged upload for large data
    fn upload_voxels_staged(
        &self,
        context: &WebGpuContext,
        offset: u64,
        data: &[VoxelData],
    ) -> Result<(), WebError> {
        let bytes: Vec<u8> = data.iter()
            .flat_map(|v| v.0.to_le_bytes())
            .collect();
        
        let staging_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxel Staging Buffer"),
            contents: &bytes,
            usage: BufferUsages::COPY_SRC,
        });
        
        let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Voxel Upload"),
        });
        
        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &self.voxel_buffer,
            offset,
            bytes.len() as u64,
        );
        
        context.queue.submit(std::iter::once(encoder.finish()));
        
        Ok(())
    }
    
    /// Enable shared memory if available (for future use with SharedArrayBuffer)
    pub fn enable_shared_memory(&mut self) -> bool {
        if self.supports_shared_memory {
            log::info!("SharedArrayBuffer support detected - enabling zero-copy optimizations");
            true
        } else {
            log::warn!("SharedArrayBuffer not available - using standard buffer copies");
            false
        }
    }
}

/// Check if browser supports SharedArrayBuffer
fn check_shared_memory_support() -> bool {
    // In a real implementation, this would check for SharedArrayBuffer availability
    // For now, we assume it's not available due to security restrictions
    false
}

/// Browser-optimized chunk upload
pub struct WebChunkUploader {
    /// Pending uploads
    pending: Vec<(u64, Vec<VoxelData>)>,
    
    /// Maximum batch size
    batch_size: usize,
}

impl WebChunkUploader {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            batch_size: 10, // Process 10 chunks at a time
        }
    }
    
    /// Queue a chunk for upload
    pub fn queue_chunk(&mut self, offset: u64, data: Vec<VoxelData>) {
        self.pending.push((offset, data));
    }
    
    /// Process pending uploads
    pub async fn flush(
        &mut self,
        context: &WebGpuContext,
        world_buffer: &WebWorldBuffer,
    ) -> Result<(), WebError> {
        // Process in batches to avoid blocking the browser
        for batch in self.pending.chunks(self.batch_size) {
            for (offset, data) in batch {
                world_buffer.upload_voxels_async(context, *offset, data).await?;
            }
            
            // Yield to browser event loop
            yield_to_browser().await;
        }
        
        self.pending.clear();
        Ok(())
    }
}

/// Yield control back to the browser event loop
async fn yield_to_browser() {
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::js_sys;
    
    let promise = js_sys::Promise::resolve(&JsValue::NULL);
    // Ignore the result of yielding - if it fails, we continue anyway
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}