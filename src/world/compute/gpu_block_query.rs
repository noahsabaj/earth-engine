use crate::world::core::{BlockId, ChunkPos, VoxelPos};
use crate::world::storage::WorldBuffer;
use bytemuck::{Pod, Zeroable};
/// GPU-accelerated block queries
///
/// This module provides high-performance block queries that run entirely on GPU,
/// eliminating the CPU bottleneck of transferring data for every block access.
use std::sync::Arc;
use wgpu::util::DeviceExt;

// Include constants from root constants.rs
include!("../../../constants.rs");

/// A batch query request for multiple blocks
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BlockQueryRequest {
    /// World position to query
    pub position: [i32; 3],
    /// Query type (0 = get block, 1 = get light, 2 = get metadata)
    pub query_type: u32,
}

/// Query result returned from GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BlockQueryResult {
    /// The requested position (for matching with request)
    pub position: [i32; 3],
    /// Query type that was executed
    pub query_type: u32,
    /// Result value (block ID, light level, etc.)
    pub value: u32,
    /// Success flag (0 = out of bounds, 1 = success)
    pub success: u32,
    /// Padding for alignment
    pub _padding: [u32; 2],
}

/// GPU block query system
pub struct GpuBlockQuery {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    /// Query pipeline
    query_pipeline: wgpu::ComputePipeline,

    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,

    /// Staging buffers for queries
    query_staging_buffer: wgpu::Buffer,
    result_staging_buffer: wgpu::Buffer,

    /// Maximum queries per batch
    max_batch_size: u32,
}

impl GpuBlockQuery {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Create shader module using unified GPU system
        let shader_source = include_str!("../../shaders/compute/block_query.wgsl");
        let validated_shader = match crate::gpu::automation::create_gpu_shader(
            &device,
            "block_query",
            shader_source,
        ) {
            Ok(shader) => shader,
            Err(e) => {
                log::error!("Failed to create block query shader: {}", e);
                panic!("Failed to create block query shader: {}", e);
            }
        };

        // Create bind group layout using macro
        let bind_group_layout = crate::create_bind_group_layout!(
            &device,
            "Block Query Bind Group Layout",
            0 => buffer(storage_read),  // World buffer (read-only)
            1 => buffer(storage_read),  // Query requests
            2 => buffer(storage)        // Query results
        );

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Block Query Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::COMPUTE,
                range: 0..8, // query_count, chunk_size
            }],
        });

        // Create compute pipeline
        let query_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Block Query Pipeline"),
            layout: Some(&pipeline_layout),
            module: &validated_shader.module,
            entry_point: "query_blocks",
        });

        const MAX_BATCH_SIZE: u32 = 65536; // 64K queries per batch

        // Create staging buffers
        let query_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Query Staging Buffer"),
            size: (std::mem::size_of::<BlockQueryRequest>() * MAX_BATCH_SIZE as usize) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let result_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Result Staging Buffer"),
            size: (std::mem::size_of::<BlockQueryResult>() * MAX_BATCH_SIZE as usize) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            query_pipeline,
            bind_group_layout,
            query_staging_buffer,
            result_staging_buffer,
            max_batch_size: MAX_BATCH_SIZE,
        }
    }

    /// Execute a batch of block queries on GPU
    pub async fn query_blocks(
        &self,
        world_buffer: &WorldBuffer,
        queries: &[BlockQueryRequest],
    ) -> Result<Vec<BlockQueryResult>, wgpu::BufferAsyncError> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }

        let query_count = queries.len().min(self.max_batch_size as usize);
        let queries = &queries[..query_count];

        // Upload queries to GPU
        self.queue
            .write_buffer(&self.query_staging_buffer, 0, bytemuck::cast_slice(queries));

        // Create bind group using macro
        let bind_group = crate::create_bind_group!(
            &self.device,
            "Block Query Bind Group",
            &self.bind_group_layout,
            0 => world_buffer.voxel_buffer().as_entire_binding(),
            1 => self.query_staging_buffer.as_entire_binding(),
            2 => self.result_staging_buffer.as_entire_binding()
        );

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Block Query Encoder"),
            });

        // Execute queries
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Block Query Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.query_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.set_push_constants(
                0,
                bytemuck::cast_slice(&[query_count as u32, crate::core::CHUNK_SIZE]),
            );

            // One workgroup per MAX_WORKGROUP_SIZE queries
            let workgroups = (query_count as u32 + gpu_limits::MAX_WORKGROUP_SIZE - 1)
                / gpu_limits::MAX_WORKGROUP_SIZE;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Copy results to mappable buffer
        let result_size = (std::mem::size_of::<BlockQueryResult>() * query_count) as u64;
        let download_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Query Download Buffer"),
            size: result_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(
            &self.result_staging_buffer,
            0,
            &download_buffer,
            0,
            result_size,
        );

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map buffer and read results
        let buffer_slice = download_buffer.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            // If we can't send the result, the receiver was dropped
            // This is logged but doesn't crash
            if tx.send(result).is_err() {
                log::error!("[GpuBlockQuery] Failed to send map_async result - receiver dropped");
            }
        });

        self.device.poll(wgpu::Maintain::Wait);

        // Handle the channel receive
        match rx.await {
            Ok(map_result) => map_result?,
            Err(_) => {
                log::error!("[GpuBlockQuery] Failed to receive map_async result - sender dropped");
                return Err(wgpu::BufferAsyncError);
            }
        };

        let data = buffer_slice.get_mapped_range();
        let results = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        download_buffer.unmap();

        Ok(results)
    }

    /// Query a single block (convenience method)
    pub async fn query_block(
        &self,
        world_buffer: &WorldBuffer,
        position: VoxelPos,
    ) -> Result<Option<BlockId>, wgpu::BufferAsyncError> {
        let request = BlockQueryRequest {
            position: [position.x, position.y, position.z],
            query_type: 0, // Get block
        };

        let results = self.query_blocks(world_buffer, &[request]).await?;

        if let Some(result) = results.first() {
            if result.success != 0 {
                return Ok(Some(BlockId(result.value as u16)));
            }
        }

        Ok(None)
    }
}

/// Async block query handle for batching
pub struct BlockQueryHandle {
    query_system: Arc<GpuBlockQuery>,
    pending_queries: parking_lot::Mutex<
        Vec<(
            BlockQueryRequest,
            tokio::sync::oneshot::Sender<BlockQueryResult>,
        )>,
    >,
}

impl BlockQueryHandle {
    pub fn new(query_system: Arc<GpuBlockQuery>) -> Self {
        Self {
            query_system,
            pending_queries: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// Queue a block query
    pub async fn query_block(&self, position: VoxelPos) -> Option<BlockId> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let request = BlockQueryRequest {
            position: [position.x, position.y, position.z],
            query_type: 0,
        };

        self.pending_queries.lock().push((request, tx));

        // In a real implementation, this would trigger batch processing
        // when enough queries accumulate or after a timeout

        if let Ok(result) = rx.await {
            if result.success != 0 {
                return Some(BlockId(result.value as u16));
            }
        }

        None
    }
}
