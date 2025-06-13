/// CPU-GPU Synchronization Primitives
/// 
/// Provides barriers and fences for proper synchronization
/// between CPU and GPU operations.

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use wgpu::Device;
use super::{MemoryResult, MemoryErrorContext};

/// Synchronization point in the GPU timeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncPoint {
    /// Fence value for this sync point
    pub fence_value: u64,
    
    /// Timestamp when created (for profiling)
    pub timestamp: std::time::Instant,
}

/// Fence wrapper for synchronization
pub struct Fence {
    /// Unique fence ID
    id: u64,
    
    /// Current fence value
    value: Arc<Mutex<u64>>,
    
    /// Completed value (last known completed)
    completed_value: Arc<Mutex<u64>>,
}

impl Fence {
    fn new(id: u64) -> Self {
        Self {
            id,
            value: Arc::new(Mutex::new(0)),
            completed_value: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Get current fence value
    pub fn value(&self) -> MemoryResult<u64> {
        Ok(*self.value.lock().memory_context("fence_value")?)
    }
    
    /// Increment and return new fence value
    pub fn increment(&self) -> MemoryResult<u64> {
        let mut value = self.value.lock().memory_context("fence_value")?;
        *value += 1;
        Ok(*value)
    }
    
    /// Update completed value
    pub fn update_completed(&self, value: u64) -> MemoryResult<()> {
        let mut completed = self.completed_value.lock()
            .memory_context("completed_value")?;
        *completed = (*completed).max(value);
        Ok(())
    }
    
    /// Check if a fence value has completed
    pub fn is_completed(&self, value: u64) -> MemoryResult<bool> {
        Ok(*self.completed_value.lock().memory_context("completed_value")? >= value)
    }
    
    /// Wait for a fence value to complete
    pub fn wait(&self, value: u64) -> MemoryResult<()> {
        while !self.is_completed(value)? {
            std::thread::yield_now();
        }
        Ok(())
    }
}

/// Pool of reusable fences
pub struct FencePool {
    device: Arc<Device>,
    available: Mutex<VecDeque<Arc<Fence>>>,
    active: Mutex<Vec<Arc<Fence>>>,
    next_id: Mutex<u64>,
}

impl FencePool {
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            available: Mutex::new(VecDeque::new()),
            active: Mutex::new(Vec::new()),
            next_id: Mutex::new(0),
        }
    }
    
    /// Acquire a fence from the pool
    pub fn acquire(&self) -> MemoryResult<Arc<Fence>> {
        let mut available = self.available.lock()
            .memory_context("available_fences")?;
        
        let fence = if let Some(fence) = available.pop_front() {
            fence
        } else {
            // Create new fence
            let mut next_id = self.next_id.lock()
                .memory_context("next_fence_id")?;
            let id = *next_id;
            *next_id += 1;
            Arc::new(Fence::new(id))
        };
        
        self.active.lock()
            .memory_context("active_fences")?
            .push(fence.clone());
        Ok(fence)
    }
    
    /// Release a fence back to the pool
    pub fn release(&self, fence: Arc<Fence>) -> MemoryResult<()> {
        let mut active = self.active.lock()
            .memory_context("active_fences")?;
        if let Some(pos) = active.iter().position(|f| Arc::ptr_eq(f, &fence)) {
            active.remove(pos);
            self.available.lock()
                .memory_context("available_fences")?
                .push_back(fence);
        }
        Ok(())
    }
    
    /// Get number of active fences
    pub fn active_count(&self) -> MemoryResult<usize> {
        Ok(self.active.lock().memory_context("active_fences")?.len())
    }
}

/// Synchronization barrier for CPU-GPU coordination
pub struct SyncBarrier {
    fence: Arc<Fence>,
    sync_points: Mutex<Vec<SyncPoint>>,
}

impl SyncBarrier {
    pub fn new(fence: Arc<Fence>) -> Self {
        Self {
            fence,
            sync_points: Mutex::new(Vec::new()),
        }
    }
    
    /// Insert a synchronization point
    pub fn insert_sync_point(&self) -> MemoryResult<SyncPoint> {
        let fence_value = self.fence.increment()?;
        let sync_point = SyncPoint {
            fence_value,
            timestamp: std::time::Instant::now(),
        };
        
        self.sync_points.lock()
            .memory_context("sync_points")?
            .push(sync_point);
        Ok(sync_point)
    }
    
    /// Wait for a sync point to complete
    pub fn wait_for_sync_point(&self, sync_point: SyncPoint) -> MemoryResult<()> {
        self.fence.wait(sync_point.fence_value)
    }
    
    /// Check if a sync point has completed
    pub fn is_sync_point_complete(&self, sync_point: SyncPoint) -> MemoryResult<bool> {
        self.fence.is_completed(sync_point.fence_value)
    }
    
    /// Update completion status (called from GPU timeline)
    pub fn signal_completion(&self, fence_value: u64) -> MemoryResult<()> {
        self.fence.update_completed(fence_value)
    }
    
    /// Get all pending sync points
    pub fn pending_sync_points(&self) -> MemoryResult<Vec<SyncPoint>> {
        let sync_points = self.sync_points.lock()
            .memory_context("sync_points")?;
        
        let mut pending = Vec::new();
        for sp in sync_points.iter() {
            if !self.fence.is_completed(sp.fence_value)? {
                pending.push(*sp);
            }
        }
        Ok(pending)
    }
}

/// Frame synchronization helper
pub struct FrameSync {
    /// Sync barriers for each frame in flight
    frame_barriers: Vec<SyncBarrier>,
    
    /// Current frame index
    current_frame: usize,
    
    /// Maximum frames in flight
    max_frames_in_flight: usize,
}

impl FrameSync {
    pub fn new(fence_pool: &FencePool, max_frames_in_flight: usize) -> MemoryResult<Self> {
        let mut frame_barriers = Vec::new();
        for _ in 0..max_frames_in_flight {
            frame_barriers.push(SyncBarrier::new(fence_pool.acquire()?));
        }
        
        Ok(Self {
            frame_barriers,
            current_frame: 0,
            max_frames_in_flight,
        })
    }
    
    /// Begin a new frame
    pub fn begin_frame(&mut self) -> &mut SyncBarrier {
        // Wait for this frame's previous submission to complete
        let frame_barrier = &self.frame_barriers[self.current_frame];
        if let Ok(sync_points) = frame_barrier.pending_sync_points() {
            if let Some(oldest_sync) = sync_points.first() {
                if let Err(e) = frame_barrier.wait_for_sync_point(*oldest_sync) {
                    log::warn!("[FrameSynchronizer] Failed to wait for sync point: {:?}", e);
                }
            }
        }
        
        &mut self.frame_barriers[self.current_frame]
    }
    
    /// End current frame and advance
    pub fn end_frame(&mut self) -> MemoryResult<SyncPoint> {
        let sync_point = self.frame_barriers[self.current_frame].insert_sync_point()?;
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
        Ok(sync_point)
    }
    
    /// Get current frame index
    pub fn current_frame_index(&self) -> usize {
        self.current_frame
    }
}

/// Timeline synchronization for complex GPU operations
pub struct Timeline {
    name: String,
    sync_barrier: SyncBarrier,
    stages: Mutex<Vec<TimelineStage>>,
}

#[derive(Debug, Clone)]
struct TimelineStage {
    name: String,
    sync_point: SyncPoint,
    dependencies: Vec<SyncPoint>,
}

impl Timeline {
    pub fn new(name: String, fence: Arc<Fence>) -> Self {
        Self {
            name,
            sync_barrier: SyncBarrier::new(fence),
            stages: Mutex::new(Vec::new()),
        }
    }
    
    /// Add a stage to the timeline
    pub fn add_stage(&self, name: String, dependencies: Vec<SyncPoint>) -> MemoryResult<SyncPoint> {
        // Wait for all dependencies
        for dep in &dependencies {
            self.sync_barrier.wait_for_sync_point(*dep)?;
        }
        
        let sync_point = self.sync_barrier.insert_sync_point()?;
        
        self.stages.lock()
            .memory_context("timeline_stages")?
            .push(TimelineStage {
                name,
                sync_point,
                dependencies,
            });
        
        Ok(sync_point)
    }
    
    /// Wait for entire timeline to complete
    pub fn wait_all(&self) -> MemoryResult<()> {
        let stages = self.stages.lock()
            .memory_context("timeline_stages")?;
        if let Some(last_stage) = stages.last() {
            self.sync_barrier.wait_for_sync_point(last_stage.sync_point)?;
        }
        Ok(())
    }
}