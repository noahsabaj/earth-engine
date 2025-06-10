/// Unified Memory Management System
/// 
/// Provides efficient memory allocation, persistent mapped buffers,
/// and CPU-GPU synchronization primitives for the engine.
/// 
/// Part of Sprint 33: Legacy System Migration & Memory Optimization

pub mod persistent_buffer;
pub mod memory_pool;
pub mod sync_barrier;
pub mod bandwidth_profiler;
pub mod performance_metrics;

pub use persistent_buffer::{PersistentBuffer, MappedBuffer, BufferUsage};
pub use memory_pool::{MemoryPool, PoolHandle, AllocationStrategy};
pub use sync_barrier::{SyncBarrier, SyncPoint, FencePool};
pub use bandwidth_profiler::{BandwidthProfiler, TransferMetrics, TransferType};
pub use performance_metrics::{PerformanceMetrics, MetricType, Implementation, ComparisonResult};

use std::sync::Arc;
use wgpu::Device;

/// Memory manager configuration
pub struct MemoryConfig {
    /// Maximum persistent buffer size (bytes)
    pub max_persistent_size: u64,
    
    /// Number of frames to keep mapped
    pub frame_buffer_count: usize,
    
    /// Enable memory profiling
    pub enable_profiling: bool,
    
    /// Pool allocation strategy
    pub allocation_strategy: AllocationStrategy,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_persistent_size: 256 * 1024 * 1024, // 256MB
            frame_buffer_count: 3, // Triple buffering
            enable_profiling: cfg!(debug_assertions),
            allocation_strategy: AllocationStrategy::BestFit,
        }
    }
}

/// Unified memory manager for the engine
pub struct MemoryManager {
    device: Arc<Device>,
    config: MemoryConfig,
    
    /// Pool for general allocations
    general_pool: MemoryPool,
    
    /// Pool for persistent mapped buffers
    persistent_pool: MemoryPool,
    
    /// Synchronization barrier pool
    sync_barriers: FencePool,
    
    /// Bandwidth profiler
    profiler: Option<BandwidthProfiler>,
    
    /// Performance metrics for comparison
    performance_metrics: Option<PerformanceMetrics>,
}

impl MemoryManager {
    pub fn new(device: Arc<Device>, config: MemoryConfig) -> Self {
        let profiler = if config.enable_profiling {
            Some(BandwidthProfiler::new())
        } else {
            None
        };
        
        let performance_metrics = if config.enable_profiling {
            Some(PerformanceMetrics::new())
        } else {
            None
        };
        
        Self {
            device: device.clone(),
            general_pool: MemoryPool::new(device.clone(), 1024 * 1024 * 1024), // 1GB
            persistent_pool: MemoryPool::new(device.clone(), config.max_persistent_size),
            sync_barriers: FencePool::new(device.clone()),
            profiler,
            performance_metrics,
            config,
        }
    }
    
    /// Allocate a general purpose buffer
    pub fn alloc_buffer(&mut self, size: u64, usage: wgpu::BufferUsages) -> PoolHandle {
        self.general_pool.allocate(size, usage)
    }
    
    /// Allocate a persistent mapped buffer
    pub fn alloc_persistent(&mut self, size: u64, usage: BufferUsage) -> PersistentBuffer {
        let handle = self.persistent_pool.allocate(
            size,
            usage.to_wgpu_usage() | wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::MAP_READ,
        );
        
        PersistentBuffer::new(
            self.device.clone(),
            handle,
            size,
            usage,
            self.config.frame_buffer_count,
        )
    }
    
    /// Create a sync barrier
    pub fn create_sync_barrier(&mut self) -> SyncBarrier {
        SyncBarrier::new(self.sync_barriers.acquire())
    }
    
    /// Record a transfer for profiling
    pub fn record_transfer(&mut self, bytes: u64, duration_us: u64) {
        if let Some(profiler) = &mut self.profiler {
            profiler.record_transfer(bytes, duration_us);
        }
    }
    
    /// Get current memory usage stats
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            general_allocated: self.general_pool.allocated_bytes(),
            general_used: self.general_pool.used_bytes(),
            persistent_allocated: self.persistent_pool.allocated_bytes(),
            persistent_used: self.persistent_pool.used_bytes(),
            sync_barriers_active: self.sync_barriers.active_count(),
        }
    }
    
    /// Get bandwidth metrics if profiling is enabled
    pub fn get_bandwidth_metrics(&self) -> Option<TransferMetrics> {
        self.profiler.as_ref().map(|p| p.get_metrics())
    }
    
    /// Get performance metrics handle
    pub fn performance_metrics(&self) -> Option<&PerformanceMetrics> {
        self.performance_metrics.as_ref()
    }
    
    /// Print performance comparison report
    pub fn print_performance_report(&self) {
        if let Some(metrics) = &self.performance_metrics {
            metrics.print_report();
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub general_allocated: u64,
    pub general_used: u64,
    pub persistent_allocated: u64,
    pub persistent_used: u64,
    pub sync_barriers_active: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_config_defaults() {
        let config = MemoryConfig::default();
        assert_eq!(config.max_persistent_size, 256 * 1024 * 1024);
        assert_eq!(config.frame_buffer_count, 3);
    }
}