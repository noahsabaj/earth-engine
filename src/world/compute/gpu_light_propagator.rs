use crate::{
    memory::BandwidthProfiler,
    world::compute::GpuLighting,
    world::core::{BlockId, ChunkPos, VoxelPos},
    world::lighting::{BlockProvider, LightUpdate, LightingStats},
    world::storage::WorldBuffer,
};
use std::collections::VecDeque;
/// GPU Lighting Propagation
///
/// Provides GPU-accelerated light propagation that replaces the CPU version.
use std::sync::Arc;
use wgpu::{Device, Queue};

/// GPU-accelerated light propagator that replaces the CPU version
pub struct GpuLightPropagator {
    device: Arc<Device>,
    queue: Arc<Queue>,
    gpu_lighting: Arc<GpuLighting>,
    world_buffer: Arc<std::sync::Mutex<WorldBuffer>>,

    /// Pending light updates to be processed
    pending_updates: Arc<parking_lot::Mutex<VecDeque<LightUpdate>>>,

    /// Statistics tracking
    stats: Arc<parking_lot::RwLock<LightingStats>>,

    /// Bandwidth profiler for performance monitoring
    profiler: Option<Arc<parking_lot::Mutex<BandwidthProfiler>>>,
}

impl GpuLightPropagator {
    /// Create a new GPU light propagator
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        world_buffer: Arc<std::sync::Mutex<WorldBuffer>>,
    ) -> Self {
        let gpu_lighting = Arc::new(GpuLighting::new(device.clone()));

        Self {
            device,
            queue,
            gpu_lighting,
            world_buffer,
            pending_updates: Arc::new(parking_lot::Mutex::new(VecDeque::new())),
            stats: Arc::new(parking_lot::RwLock::new(LightingStats::default())),
            profiler: None,
        }
    }

    /// Enable bandwidth profiling
    pub fn with_profiler(mut self, profiler: Arc<parking_lot::Mutex<BandwidthProfiler>>) -> Self {
        self.profiler = Some(profiler);
        self
    }

    /// Add a light update to the queue
    pub fn add_update(&self, update: LightUpdate) {
        self.pending_updates.lock().push_back(update);
    }

    /// Process all pending light updates on the GPU
    pub fn process_updates(&self) -> anyhow::Result<()> {
        let updates = {
            let mut pending = self.pending_updates.lock();
            std::mem::take(&mut *pending)
        };

        if updates.is_empty() {
            return Ok(());
        }

        // Group updates by chunk
        let chunk_size = crate::constants::core::CHUNK_SIZE as i32;
        let mut chunks_to_update = std::collections::HashSet::new();
        for update in updates {
            let chunk_pos = ChunkPos {
                x: update.pos.x / chunk_size,
                y: update.pos.y / chunk_size,
                z: update.pos.z / chunk_size,
            };
            chunks_to_update.insert(chunk_pos);
        }

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Light Propagation Encoder"),
            });

        // Get world buffer
        let world_buffer = self
            .world_buffer
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock world buffer: {}", e))?;

        // Update lighting for affected chunks
        let chunk_positions: Vec<ChunkPos> = chunks_to_update.into_iter().collect();
        self.gpu_lighting
            .batch_update_lighting(&mut encoder, &world_buffer, &chunk_positions);

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Update stats
        let mut stats = self.stats.write();
        stats.chunks_affected += chunk_positions.len();
        stats.updates_processed += 1;

        Ok(())
    }

    /// Force lighting update for a specific chunk
    pub fn update_chunk(&self, chunk_pos: ChunkPos) -> anyhow::Result<()> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Chunk Light Update Encoder"),
            });

        let world_buffer = self
            .world_buffer
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock world buffer: {}", e))?;

        self.gpu_lighting
            .update_chunk_lighting(&mut encoder, &world_buffer, chunk_pos);

        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    /// Get current statistics
    pub fn get_stats(&self) -> LightingStats {
        self.stats.read().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.write() = LightingStats::default();
    }
}

/// Block provider implementation for GPU world buffer
pub struct GpuBlockProvider {
    world_buffer: Arc<std::sync::Mutex<WorldBuffer>>,
}

impl GpuBlockProvider {
    pub fn new(world_buffer: Arc<std::sync::Mutex<WorldBuffer>>) -> Self {
        Self { world_buffer }
    }
}

impl BlockProvider for GpuBlockProvider {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        // This is a placeholder - in practice, GPU light propagation
        // doesn't need CPU-side block queries as all data is on GPU
        // Return AIR as default for GPU-based provider
        BlockId::AIR
    }

    fn is_transparent(&self, pos: VoxelPos) -> bool {
        // Get the block at position and check transparency
        let block = self.get_block(pos);
        // Basic transparency check - could be expanded
        block == BlockId::AIR || block == BlockId::WATER
    }
}
