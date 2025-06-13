/// GPU Lighting Migration Module
/// 
/// Provides a compatibility layer to migrate from CPU-based parallel
/// light propagation to GPU compute-based lighting.

use std::sync::Arc;
use std::collections::VecDeque;
use wgpu::{Device, Queue};
use crate::{
    world::{ChunkPos, VoxelPos, BlockId},
    lighting::{LightUpdate, LightingStats, BlockProvider},
    world_gpu::{WorldBuffer, GpuLighting},
    memory::BandwidthProfiler,
};

/// GPU-accelerated light propagator that replaces the CPU version
pub struct GpuLightPropagator {
    device: Arc<Device>,
    queue: Arc<Queue>,
    gpu_lighting: Arc<GpuLighting>,
    world_buffer: Arc<WorldBuffer>,
    
    /// Pending light updates to be processed
    pending_updates: Arc<parking_lot::Mutex<VecDeque<LightUpdate>>>,
    
    /// Statistics tracking
    stats: Arc<parking_lot::RwLock<LightingStats>>,
    
    /// Bandwidth profiler for performance monitoring
    profiler: Option<Arc<parking_lot::Mutex<BandwidthProfiler>>>,
}

impl GpuLightPropagator {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        world_buffer: Arc<WorldBuffer>,
        enable_profiling: bool,
    ) -> Self {
        let gpu_lighting = Arc::new(GpuLighting::new(device.clone()));
        
        let profiler = if enable_profiling {
            Some(Arc::new(parking_lot::Mutex::new(BandwidthProfiler::new())))
        } else {
            None
        };
        
        Self {
            device,
            queue,
            gpu_lighting,
            world_buffer,
            pending_updates: Arc::new(parking_lot::Mutex::new(VecDeque::new())),
            stats: Arc::new(parking_lot::RwLock::new(LightingStats::default())),
            profiler,
        }
    }
    
    /// Queue a light update for GPU processing
    pub fn queue_update(&self, update: LightUpdate) {
        self.pending_updates.lock().push_back(update);
    }
    
    /// Process all pending light updates on the GPU
    pub fn process_updates(&self) -> Result<(), wgpu::Error> {
        let updates: Vec<LightUpdate> = {
            let mut pending = self.pending_updates.lock();
            pending.drain(..).collect()
        };
        
        if updates.is_empty() {
            return Ok(());
        }
        
        let start_time = std::time::Instant::now();
        
        // Group updates by chunk for efficient processing
        let mut chunks_to_update = std::collections::HashSet::new();
        for update in &updates {
            let chunk_pos = self.world_to_chunk_pos(update.pos);
            chunks_to_update.insert(chunk_pos);
            
            // Add neighboring chunks for smooth lighting transitions
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        chunks_to_update.insert(ChunkPos {
                            x: chunk_pos.x + dx,
                            y: chunk_pos.y + dy,
                            z: chunk_pos.z + dz,
                        });
                    }
                }
            }
        }
        
        // Convert to vec for GPU processing
        let chunk_positions: Vec<ChunkPos> = chunks_to_update.into_iter().collect();
        
        // Create command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GPU Light Update Encoder"),
        });
        
        // Process lighting updates on GPU
        self.gpu_lighting.batch_update_lighting(
            &mut encoder,
            &self.world_buffer,
            &chunk_positions,
        );
        
        // Submit GPU commands
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // Update statistics
        let duration = start_time.elapsed();
        let mut stats = self.stats.write();
        stats.updates_processed += updates.len();
        stats.chunks_affected += chunk_positions.len();
        stats.total_propagation_time += duration;
        stats.updates_per_second = if duration.as_secs_f32() > 0.0 {
            updates.len() as f32 / duration.as_secs_f32()
        } else {
            0.0
        };
        
        // Record bandwidth if profiling enabled
        if let Some(profiler) = &self.profiler {
            let bytes_transferred = chunk_positions.len() as u64 * 32 * 32 * 32 * 4; // Approximate
            let duration_us = duration.as_micros() as u64;
            profiler.lock().record_typed_transfer(
                bytes_transferred,
                duration_us,
                crate::memory::TransferType::Copy,
            );
        }
        
        Ok(())
    }
    
    /// Convert world position to chunk position
    fn world_to_chunk_pos(&self, pos: VoxelPos) -> ChunkPos {
        let chunk_size = 32; // TODO: Get from world config
        ChunkPos {
            x: pos.x.div_euclid(chunk_size),
            y: pos.y.div_euclid(chunk_size),
            z: pos.z.div_euclid(chunk_size),
        }
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> LightingStats {
        self.stats.read().clone()
    }
    
    /// Clear statistics
    pub fn clear_stats(&self) {
        *self.stats.write() = LightingStats::default();
    }
    
    /// Get bandwidth metrics if profiling is enabled
    pub fn get_bandwidth_metrics(&self) -> Option<crate::memory::TransferMetrics> {
        self.profiler.as_ref().map(|p| p.lock().get_metrics())
    }
}

/// Compatibility adapter for existing code that expects BlockProvider
pub struct GpuBlockProvider {
    world_buffer: Arc<WorldBuffer>,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl GpuBlockProvider {
    pub fn new(world_buffer: Arc<WorldBuffer>, device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            world_buffer,
            device,
            queue,
        }
    }
}

impl BlockProvider for GpuBlockProvider {
    fn get_block(&self, _pos: VoxelPos) -> BlockId {
        // In GPU mode, block data is accessed directly on GPU
        // This is a compatibility stub
        BlockId::default()
    }
    
    fn is_transparent(&self, _pos: VoxelPos) -> bool {
        // In GPU mode, transparency is determined in shaders
        true
    }
}

/// Migration helper to convert CPU light propagator usage to GPU
pub fn migrate_to_gpu_lighting(
    device: Arc<Device>,
    queue: Arc<Queue>,
    world_buffer: Arc<WorldBuffer>,
) -> GpuLightPropagator {
    GpuLightPropagator::new(device, queue, world_buffer, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gpu_light_propagator_creation() {
        // Test would require GPU device setup
    }
}