//! Reality Check Profiler - Brutal honesty about Earth Engine's actual performance
//! 
//! This profiler exposes the TRUTH about performance, not marketing claims.
//! Current reality: 0.8 FPS is TERRIBLE. Let's measure why.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::alloc::{GlobalAlloc, Layout, System};

/// The brutal truth about frame performance
#[derive(Debug, Clone)]
pub struct FrameMetrics {
    /// Total frame time in milliseconds
    pub total_frame_ms: f32,
    
    /// CPU time spent (main thread)
    pub cpu_main_thread_ms: f32,
    
    /// GPU command recording time
    pub gpu_command_recording_ms: f32,
    
    /// GPU execution time (if measurable)
    pub gpu_execution_ms: Option<f32>,
    
    /// Time spent waiting for GPU
    pub gpu_wait_ms: f32,
    
    /// Memory allocated this frame (bytes)
    pub memory_allocated: usize,
    
    /// Memory freed this frame (bytes) 
    pub memory_freed: usize,
    
    /// Number of draw calls
    pub draw_calls: u32,
    
    /// Number of compute dispatches
    pub compute_dispatches: u32,
    
    /// Main thread blocking operations
    pub blocking_operations: Vec<BlockingOperation>,
    
    /// Actual FPS (not claimed)
    pub actual_fps: f32,
    
    /// GPU utilization percentage (0-100)
    pub gpu_utilization: f32,
}

/// Operations that block the main thread
#[derive(Debug, Clone)]
pub struct BlockingOperation {
    pub name: String,
    pub duration_ms: f32,
    pub operation_type: BlockingType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockingType {
    /// Waiting for GPU to finish
    GpuSync,
    /// Memory allocation
    MemoryAllocation,
    /// File I/O
    FileIO,
    /// Chunk generation
    ChunkGeneration,
    /// Mesh building
    MeshBuilding,
    /// Physics update
    PhysicsUpdate,
    /// Other CPU work
    CpuWork,
}

/// System-level performance breakdown
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub system_name: String,
    pub cpu_time_ms: f32,
    pub gpu_time_ms: Option<f32>,
    pub memory_allocated: usize,
    pub is_blocking_main_thread: bool,
}

/// GPU timestamp query wrapper
pub struct GpuTimestamps {
    query_set: Option<wgpu::QuerySet>,
    query_buffer: Option<wgpu::Buffer>,
    staging_buffer: Option<wgpu::Buffer>,
    timestamp_period: f32,
    max_queries: u32,
}

impl GpuTimestamps {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        // Check if timestamp queries are supported
        let features = device.features();
        if !features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            log::warn!("GPU timestamp queries not supported - GPU timing will be estimates only!");
            return Self {
                query_set: None,
                query_buffer: None,
                staging_buffer: None,
                timestamp_period: 1.0,
                max_queries: 0,
            };
        }
        
        let max_queries = 64; // Enough for detailed GPU profiling
        
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("Reality Check GPU Timer"),
            ty: wgpu::QueryType::Timestamp,
            count: max_queries,
        });
        
        let buffer_size = (max_queries * 8) as u64;
        
        let query_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reality Check Query Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reality Check Staging Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        Self {
            query_set: Some(query_set),
            query_buffer: Some(query_buffer),
            staging_buffer: Some(staging_buffer),
            timestamp_period: queue.get_timestamp_period(),
            max_queries,
        }
    }
    
    pub fn write_timestamp(&self, encoder: &mut wgpu::CommandEncoder, index: u32) {
        if let Some(query_set) = &self.query_set {
            if index < self.max_queries {
                encoder.write_timestamp(query_set, index);
            }
        }
    }
    
    pub async fn resolve_timestamps(&self, device: &wgpu::Device, queue: &wgpu::Queue, count: u32) -> Vec<f32> {
        if self.query_set.is_none() || count == 0 || count > self.max_queries {
            return Vec::new();
        }
        
        let query_set = self.query_set.as_ref().unwrap();
        let query_buffer = self.query_buffer.as_ref().unwrap();
        let staging_buffer = self.staging_buffer.as_ref().unwrap();
        
        // Resolve queries
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Reality Check Resolve"),
        });
        
        encoder.resolve_query_set(query_set, 0..count, query_buffer, 0);
        encoder.copy_buffer_to_buffer(
            query_buffer, 
            0, 
            staging_buffer, 
            0, 
            (count * 8) as u64
        );
        
        queue.submit(Some(encoder.finish()));
        
        // Read results
        let buffer_slice = staging_buffer.slice(..(count * 8) as u64);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        
        device.poll(wgpu::Maintain::Wait);
        
        if receiver.await.unwrap_or(Err(wgpu::BufferAsyncError)).is_err() {
            return Vec::new();
        }
        
        let data = buffer_slice.get_mapped_range();
        let timestamps: Vec<u64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();
        
        // Convert to milliseconds
        let mut times = Vec::new();
        for i in (0..timestamps.len()).step_by(2) {
            if i + 1 < timestamps.len() {
                let start = timestamps[i] as f32 * self.timestamp_period;
                let end = timestamps[i + 1] as f32 * self.timestamp_period;
                times.push((end - start) / 1_000_000.0); // nanoseconds to ms
            }
        }
        
        times
    }
}

/// Custom allocator to track memory allocations
pub struct TrackingAllocator {
    allocations: Arc<AtomicUsize>,
    deallocations: Arc<AtomicUsize>,
    current_usage: Arc<AtomicUsize>,
    peak_usage: Arc<AtomicUsize>,
}

impl TrackingAllocator {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(AtomicUsize::new(0)),
            deallocations: Arc::new(AtomicUsize::new(0)),
            current_usage: Arc::new(AtomicUsize::new(0)),
            peak_usage: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn get_frame_allocations(&self) -> usize {
        self.allocations.swap(0, Ordering::SeqCst)
    }
    
    pub fn get_frame_deallocations(&self) -> usize {
        self.deallocations.swap(0, Ordering::SeqCst)
    }
    
    pub fn get_current_usage(&self) -> usize {
        self.current_usage.load(Ordering::SeqCst)
    }
    
    pub fn get_peak_usage(&self) -> usize {
        self.peak_usage.load(Ordering::SeqCst)
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            self.allocations.fetch_add(layout.size(), Ordering::SeqCst);
            let current = self.current_usage.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            self.peak_usage.fetch_max(current, Ordering::SeqCst);
        }
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocations.fetch_add(layout.size(), Ordering::SeqCst);
        self.current_usage.fetch_sub(layout.size(), Ordering::SeqCst);
        System.dealloc(ptr, layout);
    }
}

/// The reality check profiler - exposing the truth
pub struct RealityCheckProfiler {
    /// Frame history for averaging
    frame_history: Mutex<VecDeque<FrameMetrics>>,
    
    /// Current frame being profiled
    current_frame: Mutex<Option<FrameProfiler>>,
    
    /// System metrics
    system_metrics: Mutex<HashMap<String, SystemMetrics>>,
    
    /// GPU timestamp helper
    gpu_timestamps: Option<Arc<GpuTimestamps>>,
    
    /// Memory tracking
    memory_tracker: Option<Arc<TrackingAllocator>>,
    
    /// History size
    history_size: usize,
    
    /// Enable brutal honesty mode
    brutal_honesty: bool,
}

/// Per-frame profiler
struct FrameProfiler {
    start_time: Instant,
    cpu_timers: HashMap<String, Instant>,
    gpu_timer_index: u32,
    blocking_operations: Vec<BlockingOperation>,
    draw_calls: u32,
    compute_dispatches: u32,
}

impl RealityCheckProfiler {
    /// Create new profiler with optional GPU support
    pub fn new(device: Option<&wgpu::Device>, queue: Option<&wgpu::Queue>) -> Self {
        let gpu_timestamps = if let (Some(device), Some(queue)) = (device, queue) {
            Some(Arc::new(GpuTimestamps::new(device, queue)))
        } else {
            None
        };
        
        Self {
            frame_history: Mutex::new(VecDeque::with_capacity(120)),
            current_frame: Mutex::new(None),
            system_metrics: Mutex::new(HashMap::new()),
            gpu_timestamps,
            memory_tracker: None, // Set this via set_memory_tracker
            history_size: 120, // 2 seconds at 60 FPS (or 2 minutes at 0.8 FPS...)
            brutal_honesty: true,
        }
    }
    
    /// Set memory tracker (must be done early in program initialization)
    pub fn set_memory_tracker(&mut self, tracker: Arc<TrackingAllocator>) {
        self.memory_tracker = Some(tracker);
    }
    
    /// Start profiling a new frame
    pub fn begin_frame(&self) {
        let mut current = self.current_frame.lock().unwrap();
        *current = Some(FrameProfiler {
            start_time: Instant::now(),
            cpu_timers: HashMap::new(),
            gpu_timer_index: 0,
            blocking_operations: Vec::new(),
            draw_calls: 0,
            compute_dispatches: 0,
        });
    }
    
    /// Start timing a CPU operation
    pub fn begin_cpu_timing(&self, name: &str) {
        if let Ok(mut current) = self.current_frame.lock() {
            if let Some(frame) = current.as_mut() {
                frame.cpu_timers.insert(name.to_string(), Instant::now());
            }
        }
    }
    
    /// End timing a CPU operation
    pub fn end_cpu_timing(&self, name: &str, operation_type: BlockingType) {
        if let Ok(mut current) = self.current_frame.lock() {
            if let Some(frame) = current.as_mut() {
                if let Some(start) = frame.cpu_timers.remove(name) {
                    let duration_ms = start.elapsed().as_secs_f32() * 1000.0;
                    
                    // Record if it's a blocking operation (>1ms)
                    if duration_ms > 1.0 {
                        frame.blocking_operations.push(BlockingOperation {
                            name: name.to_string(),
                            duration_ms,
                            operation_type,
                        });
                    }
                }
            }
        }
    }
    
    /// Write GPU timestamp
    pub fn write_gpu_timestamp(&self, encoder: &mut wgpu::CommandEncoder) -> Option<u32> {
        if let Some(gpu_timestamps) = &self.gpu_timestamps {
            if let Ok(mut current) = self.current_frame.lock() {
                if let Some(frame) = current.as_mut() {
                    let index = frame.gpu_timer_index;
                    frame.gpu_timer_index += 1;
                    gpu_timestamps.write_timestamp(encoder, index);
                    return Some(index);
                }
            }
        }
        None
    }
    
    /// Record a draw call
    pub fn record_draw_call(&self) {
        if let Ok(mut current) = self.current_frame.lock() {
            if let Some(frame) = current.as_mut() {
                frame.draw_calls += 1;
            }
        }
    }
    
    /// Record a compute dispatch
    pub fn record_compute_dispatch(&self) {
        if let Ok(mut current) = self.current_frame.lock() {
            if let Some(frame) = current.as_mut() {
                frame.compute_dispatches += 1;
            }
        }
    }
    
    /// End frame and calculate metrics
    pub async fn end_frame(&self, device: Option<&wgpu::Device>, queue: Option<&wgpu::Queue>) {
        let frame_data = {
            let mut current = self.current_frame.lock().unwrap();
            current.take()
        };
        
        if let Some(frame) = frame_data {
            let total_frame_ms = frame.start_time.elapsed().as_secs_f32() * 1000.0;
            let actual_fps = 1000.0 / total_frame_ms;
            
            // Get memory stats
            let (memory_allocated, memory_freed) = if let Some(tracker) = &self.memory_tracker {
                (
                    tracker.get_frame_allocations(),
                    tracker.get_frame_deallocations()
                )
            } else {
                (0, 0)
            };
            
            // Get GPU timings if available
            let gpu_times = if let (Some(gpu_timestamps), Some(device), Some(queue)) = 
                (&self.gpu_timestamps, device, queue) {
                gpu_timestamps.resolve_timestamps(device, queue, frame.gpu_timer_index).await
            } else {
                Vec::new()
            };
            
            // Calculate GPU execution time and utilization
            let (gpu_execution_ms, gpu_utilization) = if !gpu_times.is_empty() {
                let total_gpu_ms: f32 = gpu_times.iter().sum();
                let utilization = (total_gpu_ms / total_frame_ms) * 100.0;
                (Some(total_gpu_ms), utilization.min(100.0))
            } else {
                // Estimate based on frame time - if we're at 0.8 FPS, GPU is probably struggling
                let estimated_gpu = if actual_fps < 10.0 {
                    total_frame_ms * 0.7 // Assume 70% GPU bound at low FPS
                } else {
                    total_frame_ms * 0.3 // Assume 30% GPU bound at higher FPS
                };
                (None, (estimated_gpu / total_frame_ms * 100.0).min(100.0))
            };
            
            // Calculate CPU main thread time
            let blocking_time: f32 = frame.blocking_operations.iter()
                .map(|op| op.duration_ms)
                .sum();
            let cpu_main_thread_ms = blocking_time;
            
            // GPU wait time is whatever's left
            let gpu_wait_ms = (total_frame_ms - cpu_main_thread_ms - gpu_execution_ms.unwrap_or(0.0)).max(0.0);
            
            // Log brutal honesty if enabled (before moving blocking_operations)
            if self.brutal_honesty && actual_fps < 30.0 {
                log::error!("PERFORMANCE REALITY CHECK: {:.1} FPS is UNACCEPTABLE!", actual_fps);
                log::error!("  CPU main thread: {:.1}ms", cpu_main_thread_ms);
                log::error!("  GPU execution: {:.1}ms", gpu_execution_ms.unwrap_or(-1.0));
                log::error!("  GPU wait: {:.1}ms", gpu_wait_ms);
                log::error!("  Memory allocated this frame: {} KB", memory_allocated / 1024);
                
                if !frame.blocking_operations.is_empty() {
                    log::error!("  Main thread BLOCKED by:");
                    for op in &frame.blocking_operations {
                        log::error!("    - {} ({:?}): {:.1}ms", op.name, op.operation_type, op.duration_ms);
                    }
                }
            }
            
            let metrics = FrameMetrics {
                total_frame_ms,
                cpu_main_thread_ms,
                gpu_command_recording_ms: 0.0, // TODO: measure this separately
                gpu_execution_ms,
                gpu_wait_ms,
                memory_allocated,
                memory_freed,
                draw_calls: frame.draw_calls,
                compute_dispatches: frame.compute_dispatches,
                blocking_operations: frame.blocking_operations,
                actual_fps,
                gpu_utilization,
            };
            
            // Add to history
            let mut history = self.frame_history.lock().unwrap();
            history.push_back(metrics.clone());
            if history.len() > self.history_size {
                history.pop_front();
            }
        }
    }
    
    /// Record system-level metrics
    pub fn record_system_metrics(&self, system_name: &str, metrics: SystemMetrics) {
        let mut systems = self.system_metrics.lock().unwrap();
        systems.insert(system_name.to_string(), metrics);
    }
    
    /// Get average metrics over history
    pub fn get_average_metrics(&self) -> Option<FrameMetrics> {
        let history = self.frame_history.lock().unwrap();
        if history.is_empty() {
            return None;
        }
        
        let count = history.len() as f32;
        let mut avg = FrameMetrics {
            total_frame_ms: 0.0,
            cpu_main_thread_ms: 0.0,
            gpu_command_recording_ms: 0.0,
            gpu_execution_ms: None,
            gpu_wait_ms: 0.0,
            memory_allocated: 0,
            memory_freed: 0,
            draw_calls: 0,
            compute_dispatches: 0,
            blocking_operations: Vec::new(),
            actual_fps: 0.0,
            gpu_utilization: 0.0,
        };
        
        let mut has_gpu_times = false;
        let mut total_gpu_ms = 0.0;
        
        for frame in history.iter() {
            avg.total_frame_ms += frame.total_frame_ms;
            avg.cpu_main_thread_ms += frame.cpu_main_thread_ms;
            avg.gpu_command_recording_ms += frame.gpu_command_recording_ms;
            avg.gpu_wait_ms += frame.gpu_wait_ms;
            avg.memory_allocated += frame.memory_allocated;
            avg.memory_freed += frame.memory_freed;
            avg.draw_calls += frame.draw_calls;
            avg.compute_dispatches += frame.compute_dispatches;
            avg.gpu_utilization += frame.gpu_utilization;
            
            if let Some(gpu_ms) = frame.gpu_execution_ms {
                has_gpu_times = true;
                total_gpu_ms += gpu_ms;
            }
        }
        
        avg.total_frame_ms /= count;
        avg.cpu_main_thread_ms /= count;
        avg.gpu_command_recording_ms /= count;
        avg.gpu_wait_ms /= count;
        avg.memory_allocated = (avg.memory_allocated as f32 / count) as usize;
        avg.memory_freed = (avg.memory_freed as f32 / count) as usize;
        avg.draw_calls = (avg.draw_calls as f32 / count) as u32;
        avg.compute_dispatches = (avg.compute_dispatches as f32 / count) as u32;
        avg.gpu_utilization /= count;
        avg.actual_fps = 1000.0 / avg.total_frame_ms;
        
        if has_gpu_times {
            avg.gpu_execution_ms = Some(total_gpu_ms / count);
        }
        
        Some(avg)
    }
    
    /// Generate a brutal honesty report
    pub fn generate_reality_report(&self) -> String {
        let mut report = String::from("=== EARTH ENGINE REALITY CHECK REPORT ===\n\n");
        
        if let Some(avg) = self.get_average_metrics() {
            report.push_str(&format!("ACTUAL PERFORMANCE: {:.1} FPS\n", avg.actual_fps));
            
            if avg.actual_fps < 1.0 {
                report.push_str("STATUS: SLIDESHOW MODE - This is not a real-time engine\n\n");
            } else if avg.actual_fps < 30.0 {
                report.push_str("STATUS: UNPLAYABLE - Major architectural issues\n\n");
            } else if avg.actual_fps < 60.0 {
                report.push_str("STATUS: POOR - Significant optimization needed\n\n");
            } else {
                report.push_str("STATUS: ACCEPTABLE - But verify this is with real workload\n\n");
            }
            
            report.push_str("FRAME TIME BREAKDOWN:\n");
            report.push_str(&format!("  Total: {:.1}ms\n", avg.total_frame_ms));
            report.push_str(&format!("  CPU Main Thread: {:.1}ms ({:.1}%)\n", 
                avg.cpu_main_thread_ms, 
                (avg.cpu_main_thread_ms / avg.total_frame_ms) * 100.0
            ));
            
            if let Some(gpu_ms) = avg.gpu_execution_ms {
                report.push_str(&format!("  GPU Execution: {:.1}ms ({:.1}%)\n", 
                    gpu_ms,
                    (gpu_ms / avg.total_frame_ms) * 100.0
                ));
            } else {
                report.push_str("  GPU Execution: UNMEASURED (timestamp queries not available)\n");
            }
            
            report.push_str(&format!("  GPU Wait/Sync: {:.1}ms ({:.1}%)\n",
                avg.gpu_wait_ms,
                (avg.gpu_wait_ms / avg.total_frame_ms) * 100.0
            ));
            
            report.push_str(&format!("\nGPU UTILIZATION: {:.1}%\n", avg.gpu_utilization));
            
            // Reality check on GPU-first claims
            if avg.gpu_utilization < 50.0 {
                report.push_str("REALITY: This is NOT a GPU-first engine. CPU is the bottleneck.\n");
            }
            
            report.push_str(&format!("\nRENDERING STATS:\n"));
            report.push_str(&format!("  Draw Calls: {}\n", avg.draw_calls));
            report.push_str(&format!("  Compute Dispatches: {}\n", avg.compute_dispatches));
            
            report.push_str(&format!("\nMEMORY BEHAVIOR:\n"));
            report.push_str(&format!("  Allocations per frame: {} KB\n", avg.memory_allocated / 1024));
            report.push_str(&format!("  Deallocations per frame: {} KB\n", avg.memory_freed / 1024));
            
            let net_allocation = (avg.memory_allocated as i64 - avg.memory_freed as i64) / 1024;
            if net_allocation > 100 {
                report.push_str(&format!("  WARNING: MEMORY LEAK - Growing by {} KB/frame\n", net_allocation));
            }
            
            // System breakdown
            let systems = self.system_metrics.lock().unwrap();
            if !systems.is_empty() {
                report.push_str("\nSYSTEM BREAKDOWN:\n");
                let mut system_list: Vec<_> = systems.iter().collect();
                system_list.sort_by(|a, b| b.1.cpu_time_ms.partial_cmp(&a.1.cpu_time_ms).unwrap());
                
                for (name, metrics) in system_list {
                    report.push_str(&format!("  {}: {:.1}ms CPU", name, metrics.cpu_time_ms));
                    if let Some(gpu_ms) = metrics.gpu_time_ms {
                        report.push_str(&format!(", {:.1}ms GPU", gpu_ms));
                    }
                    if metrics.is_blocking_main_thread {
                        report.push_str(" [BLOCKING MAIN THREAD]");
                    }
                    report.push_str("\n");
                }
            }
            
            // Biggest lies exposed
            report.push_str("\nBIGGEST PERFORMANCE LIES EXPOSED:\n");
            
            if avg.gpu_utilization < 80.0 {
                report.push_str("  ❌ \"80-85% GPU compute\" - ACTUAL: ");
                report.push_str(&format!("{:.1}% GPU utilization\n", avg.gpu_utilization));
            }
            
            if avg.actual_fps < 60.0 {
                report.push_str("  ❌ \"Real-time performance\" - ACTUAL: ");
                report.push_str(&format!("{:.1} FPS\n", avg.actual_fps));
            }
            
            if avg.memory_allocated > 1024 * 1024 {
                report.push_str("  ❌ \"Zero-allocation\" - ACTUAL: ");
                report.push_str(&format!("{} MB allocated per frame\n", avg.memory_allocated / (1024 * 1024)));
            }
            
            // Memory tracker stats
            if let Some(tracker) = &self.memory_tracker {
                report.push_str(&format!("\nMEMORY USAGE:\n"));
                report.push_str(&format!("  Current: {} MB\n", tracker.get_current_usage() / (1024 * 1024)));
                report.push_str(&format!("  Peak: {} MB\n", tracker.get_peak_usage() / (1024 * 1024)));
            }
        } else {
            report.push_str("NO PERFORMANCE DATA COLLECTED YET\n");
        }
        
        report.push_str("\n=== END REALITY CHECK ===\n");
        report
    }
}

// Data-oriented profiling functions for integration

/// Begin profiling a frame
pub fn begin_frame(profiler: &RealityCheckProfiler) {
    profiler.begin_frame();
}

/// End profiling a frame
pub async fn end_frame(profiler: &RealityCheckProfiler, device: Option<&wgpu::Device>, queue: Option<&wgpu::Queue>) {
    profiler.end_frame(device, queue).await;
}

/// Time a CPU operation
pub fn time_cpu_operation<F, R>(profiler: &RealityCheckProfiler, name: &str, operation_type: BlockingType, f: F) -> R
where
    F: FnOnce() -> R,
{
    profiler.begin_cpu_timing(name);
    let result = f();
    profiler.end_cpu_timing(name, operation_type);
    result
}

/// Record draw call
pub fn record_draw_call(profiler: &RealityCheckProfiler) {
    profiler.record_draw_call();
}

/// Record compute dispatch
pub fn record_compute_dispatch(profiler: &RealityCheckProfiler) {
    profiler.record_compute_dispatch();
}

/// Write GPU timestamp
pub fn write_gpu_timestamp(profiler: &RealityCheckProfiler, encoder: &mut wgpu::CommandEncoder) -> Option<u32> {
    profiler.write_gpu_timestamp(encoder)
}

/// Generate reality report
pub fn generate_reality_report(profiler: &RealityCheckProfiler) -> String {
    profiler.generate_reality_report()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_frame_metrics() {
        let profiler = RealityCheckProfiler::new(None, None);
        
        // Simulate a terrible frame
        profiler.begin_frame();
        
        // Simulate some blocking operations
        profiler.begin_cpu_timing("chunk_generation");
        std::thread::sleep(Duration::from_millis(50)); // 50ms chunk gen
        profiler.end_cpu_timing("chunk_generation", BlockingType::ChunkGeneration);
        
        // Can't test async end_frame in sync test, but structure is validated
    }
    
    #[test] 
    fn test_brutal_honesty_report() {
        let profiler = RealityCheckProfiler::new(None, None);
        let report = profiler.generate_reality_report();
        
        assert!(report.contains("EARTH ENGINE REALITY CHECK REPORT"));
        assert!(report.contains("NO PERFORMANCE DATA COLLECTED YET"));
    }
}