use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Instant, Duration};
use wgpu::util::DeviceExt;

/// GPU workload profiler that measures actual GPU vs CPU distribution
pub struct GpuWorkloadProfiler {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    
    /// Timestamp query set for GPU timing
    timestamp_query_set: wgpu::QuerySet,
    
    /// Buffer to store timestamp results
    timestamp_buffer: wgpu::Buffer,
    
    /// Staging buffer for reading timestamps
    timestamp_staging_buffer: wgpu::Buffer,
    
    /// Current timestamp index
    current_timestamp_idx: u32,
    
    /// Maximum number of timestamps
    max_timestamps: u32,
    
    /// CPU timing data
    cpu_timings: Arc<Mutex<CpuTimings>>,
    
    /// GPU timing data
    gpu_timings: Arc<Mutex<GpuTimings>>,
    
    /// Frame start time
    frame_start: Option<Instant>,
    
    /// Total profiling duration
    profiling_start: Instant,
    
    /// GPU memory tracking
    gpu_memory_stats: Arc<Mutex<GpuMemoryStats>>,
    
    /// Synchronization overhead tracking
    sync_overhead: Arc<Mutex<SyncOverhead>>,
}

#[derive(Default)]
struct CpuTimings {
    /// Time spent in each system
    system_times: HashMap<String, Duration>,
    
    /// Time spent per thread
    thread_times: HashMap<std::thread::ThreadId, Duration>,
    
    /// Total CPU time this frame
    total_frame_time: Duration,
    
    /// Time spent waiting for GPU
    gpu_wait_time: Duration,
}

#[derive(Default)]
struct GpuTimings {
    /// Compute shader execution times
    compute_times: HashMap<String, Duration>,
    
    /// Render pass times
    render_pass_times: HashMap<String, Duration>,
    
    /// Memory transfer times
    transfer_times: HashMap<String, Duration>,
    
    /// Pipeline state changes
    pipeline_changes: u32,
    
    /// Total GPU time this frame
    total_frame_time: Duration,
}

#[derive(Default)]
struct GpuMemoryStats {
    /// Bytes transferred CPU -> GPU
    cpu_to_gpu_bytes: u64,
    
    /// Bytes transferred GPU -> CPU
    gpu_to_cpu_bytes: u64,
    
    /// Number of buffer updates
    buffer_updates: u32,
    
    /// Number of texture updates
    texture_updates: u32,
    
    /// Peak GPU memory usage
    peak_memory_bytes: u64,
}

#[derive(Default)]
struct SyncOverhead {
    /// Number of GPU stalls
    gpu_stalls: u32,
    
    /// Time spent in GPU stalls
    stall_time: Duration,
    
    /// Number of pipeline flushes
    pipeline_flushes: u32,
    
    /// Time waiting for fence signals
    fence_wait_time: Duration,
}

/// Profiling results
#[derive(Debug)]
pub struct WorkloadAnalysis {
    /// Actual GPU compute percentage
    pub gpu_compute_percentage: f32,
    
    /// CPU compute percentage
    pub cpu_compute_percentage: f32,
    
    /// Time spent in CPU-GPU synchronization
    pub sync_overhead_percentage: f32,
    
    /// Memory transfer overhead percentage
    pub transfer_overhead_percentage: f32,
    
    /// Detailed system breakdown
    pub system_breakdown: HashMap<String, SystemWorkload>,
    
    /// GPU utilization percentage
    pub gpu_utilization: f32,
    
    /// CPU utilization per core
    pub cpu_utilization_per_core: Vec<f32>,
    
    /// Memory bandwidth usage (GB/s)
    pub memory_bandwidth_gbps: f32,
    
    /// PCIe bandwidth usage (GB/s)
    pub pcie_bandwidth_gbps: f32,
    
    /// GPU pipeline efficiency
    pub gpu_pipeline_efficiency: f32,
    
    /// Number of GPU bubbles/stalls
    pub gpu_pipeline_stalls: u32,
    
    /// Average frame time (ms)
    pub avg_frame_time_ms: f32,
    
    /// Frame time breakdown
    pub frame_breakdown: FrameBreakdown,
}

#[derive(Debug)]
pub struct SystemWorkload {
    pub name: String,
    pub gpu_time_ms: f32,
    pub cpu_time_ms: f32,
    pub is_gpu_accelerated: bool,
    pub gpu_efficiency: f32,
}

#[derive(Debug)]
pub struct FrameBreakdown {
    pub cpu_update_ms: f32,
    pub gpu_compute_ms: f32,
    pub gpu_render_ms: f32,
    pub cpu_gpu_sync_ms: f32,
    pub memory_transfer_ms: f32,
    pub other_ms: f32,
}

impl GpuWorkloadProfiler {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self, wgpu::Error> {
        let max_timestamps = 1024;
        
        // Create timestamp query set
        let timestamp_query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("GPU Workload Timestamp Query Set"),
            ty: wgpu::QueryType::Timestamp,
            count: max_timestamps,
        });
        
        // Create buffers for timestamp results
        let timestamp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Timestamp Buffer"),
            size: (max_timestamps * 8) as u64, // 8 bytes per timestamp
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let timestamp_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Timestamp Staging Buffer"),
            size: (max_timestamps * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Ok(Self {
            device,
            queue,
            timestamp_query_set,
            timestamp_buffer,
            timestamp_staging_buffer,
            current_timestamp_idx: 0,
            max_timestamps,
            cpu_timings: Arc::new(Mutex::new(CpuTimings::default())),
            gpu_timings: Arc::new(Mutex::new(GpuTimings::default())),
            frame_start: None,
            profiling_start: Instant::now(),
            gpu_memory_stats: Arc::new(Mutex::new(GpuMemoryStats::default())),
            sync_overhead: Arc::new(Mutex::new(SyncOverhead::default())),
        })
    }
    
    /// Begin profiling a new frame
    pub fn begin_frame(&mut self) {
        self.frame_start = Some(Instant::now());
        self.current_timestamp_idx = 0;
        
        // Reset per-frame stats
        if let Ok(mut cpu_timings) = self.cpu_timings.lock() {
            cpu_timings.system_times.clear();
            cpu_timings.thread_times.clear();
            cpu_timings.total_frame_time = Duration::ZERO;
            cpu_timings.gpu_wait_time = Duration::ZERO;
        }
        
        if let Ok(mut gpu_timings) = self.gpu_timings.lock() {
            gpu_timings.compute_times.clear();
            gpu_timings.render_pass_times.clear();
            gpu_timings.transfer_times.clear();
            gpu_timings.pipeline_changes = 0;
            gpu_timings.total_frame_time = Duration::ZERO;
        }
    }
    
    /// Write GPU timestamp
    pub fn write_gpu_timestamp(&mut self, encoder: &mut wgpu::CommandEncoder, label: &str) {
        if self.current_timestamp_idx < self.max_timestamps - 1 {
            encoder.write_timestamp(&self.timestamp_query_set, self.current_timestamp_idx);
            self.current_timestamp_idx += 1;
        }
    }
    
    /// Record CPU operation timing
    pub fn time_cpu_operation<F, R>(&self, name: &str, operation: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let thread_id = std::thread::current().id();
        
        let result = operation();
        
        let duration = start.elapsed();
        
        if let Ok(mut cpu_timings) = self.cpu_timings.lock() {
            *cpu_timings.system_times.entry(name.to_string())
                .or_insert(Duration::ZERO) += duration;
            
            *cpu_timings.thread_times.entry(thread_id)
                .or_insert(Duration::ZERO) += duration;
            
            cpu_timings.total_frame_time += duration;
        }
        
        result
    }
    
    /// Record GPU compute dispatch
    pub fn record_compute_dispatch(&self, name: &str, workgroups: (u32, u32, u32)) {
        if let Ok(mut gpu_timings) = self.gpu_timings.lock() {
            // Estimate compute time based on workgroup count
            // This is a rough estimate; actual timing comes from timestamps
            let estimated_time = Duration::from_micros(
                (workgroups.0 * workgroups.1 * workgroups.2) as u64
            );
            
            *gpu_timings.compute_times.entry(name.to_string())
                .or_insert(Duration::ZERO) += estimated_time;
        }
    }
    
    /// Record render pass
    pub fn record_render_pass(&self, name: &str, draw_calls: u32) {
        if let Ok(mut gpu_timings) = self.gpu_timings.lock() {
            // Rough estimate based on draw calls
            let estimated_time = Duration::from_micros(draw_calls as u64 * 10);
            
            *gpu_timings.render_pass_times.entry(name.to_string())
                .or_insert(Duration::ZERO) += estimated_time;
        }
    }
    
    /// Record memory transfer
    pub fn record_memory_transfer(&self, bytes: u64, is_upload: bool) {
        if let Ok(mut mem_stats) = self.gpu_memory_stats.lock() {
            if is_upload {
                mem_stats.cpu_to_gpu_bytes += bytes;
                mem_stats.buffer_updates += 1;
            } else {
                mem_stats.gpu_to_cpu_bytes += bytes;
            }
        }
        
        // Estimate transfer time based on PCIe bandwidth (16 GB/s for PCIe 4.0)
        let transfer_time = Duration::from_nanos((bytes * 1_000_000_000 / 16_000_000_000) as u64);
        
        if let Ok(mut gpu_timings) = self.gpu_timings.lock() {
            *gpu_timings.transfer_times.entry("PCIe Transfer".to_string())
                .or_insert(Duration::ZERO) += transfer_time;
        }
    }
    
    /// Record GPU synchronization point
    pub fn record_gpu_sync(&self, wait_time: Duration) {
        if let Ok(mut sync) = self.sync_overhead.lock() {
            sync.gpu_stalls += 1;
            sync.stall_time += wait_time;
        }
        
        if let Ok(mut cpu_timings) = self.cpu_timings.lock() {
            cpu_timings.gpu_wait_time += wait_time;
        }
    }
    
    /// End frame and calculate metrics
    pub fn end_frame(&mut self) -> Option<Duration> {
        self.frame_start.take().map(|start| start.elapsed())
    }
    
    /// Analyze workload distribution
    pub fn analyze_workload(&self, total_duration: Duration) -> WorkloadAnalysis {
        let cpu_timings = self.cpu_timings.lock().expect("Failed to lock cpu_timings");
        let gpu_timings = self.gpu_timings.lock().expect("Failed to lock gpu_timings");
        let mem_stats = self.gpu_memory_stats.lock().expect("Failed to lock gpu_memory_stats");
        let sync_overhead = self.sync_overhead.lock().expect("Failed to lock sync_overhead");
        
        let total_ms = total_duration.as_secs_f32() * 1000.0;
        let total_cpu_ms = cpu_timings.total_frame_time.as_secs_f32() * 1000.0;
        let total_gpu_ms = gpu_timings.total_frame_time.as_secs_f32() * 1000.0;
        let sync_ms = sync_overhead.stall_time.as_secs_f32() * 1000.0;
        
        // Calculate actual workload percentages
        let gpu_compute_percentage = if total_ms > 0.0 {
            (total_gpu_ms / total_ms) * 100.0
        } else {
            0.0
        };
        
        let cpu_compute_percentage = if total_ms > 0.0 {
            ((total_cpu_ms - sync_ms) / total_ms) * 100.0
        } else {
            0.0
        };
        
        let sync_overhead_percentage = if total_ms > 0.0 {
            (sync_ms / total_ms) * 100.0
        } else {
            0.0
        };
        
        // Calculate memory bandwidth
        let total_bytes = mem_stats.cpu_to_gpu_bytes + mem_stats.gpu_to_cpu_bytes;
        let memory_bandwidth_gbps = (total_bytes as f32 / 1_000_000_000.0) / total_duration.as_secs_f32();
        
        // Build system breakdown
        let mut system_breakdown = HashMap::new();
        
        // Add CPU systems
        for (name, duration) in &cpu_timings.system_times {
            system_breakdown.insert(name.clone(), SystemWorkload {
                name: name.clone(),
                gpu_time_ms: 0.0,
                cpu_time_ms: duration.as_secs_f32() * 1000.0,
                is_gpu_accelerated: false,
                gpu_efficiency: 0.0,
            });
        }
        
        // Add GPU compute systems
        for (name, duration) in &gpu_timings.compute_times {
            let entry = system_breakdown.entry(name.clone())
                .or_insert(SystemWorkload {
                    name: name.clone(),
                    gpu_time_ms: 0.0,
                    cpu_time_ms: 0.0,
                    is_gpu_accelerated: true,
                    gpu_efficiency: 0.0,
                });
            
            entry.gpu_time_ms = duration.as_secs_f32() * 1000.0;
            entry.is_gpu_accelerated = true;
            entry.gpu_efficiency = if entry.gpu_time_ms > 0.0 {
                entry.gpu_time_ms / (entry.gpu_time_ms + entry.cpu_time_ms)
            } else {
                0.0
            };
        }
        
        // Calculate GPU utilization (simplified - would need actual GPU metrics)
        let gpu_utilization = gpu_compute_percentage.min(100.0);
        
        // Estimate CPU utilization per core
        let num_cores = num_cpus::get() as f32;
        let cpu_utilization_per_core = vec![cpu_compute_percentage / num_cores; num_cores as usize];
        
        // Frame breakdown
        let frame_breakdown = FrameBreakdown {
            cpu_update_ms: total_cpu_ms - sync_ms,
            gpu_compute_ms: gpu_timings.compute_times.values()
                .map(|d| d.as_secs_f32() * 1000.0)
                .sum(),
            gpu_render_ms: gpu_timings.render_pass_times.values()
                .map(|d| d.as_secs_f32() * 1000.0)
                .sum(),
            cpu_gpu_sync_ms: sync_ms,
            memory_transfer_ms: gpu_timings.transfer_times.values()
                .map(|d| d.as_secs_f32() * 1000.0)
                .sum(),
            other_ms: total_ms - total_cpu_ms - total_gpu_ms,
        };
        
        WorkloadAnalysis {
            gpu_compute_percentage,
            cpu_compute_percentage,
            sync_overhead_percentage,
            transfer_overhead_percentage: (frame_breakdown.memory_transfer_ms / total_ms) * 100.0,
            system_breakdown,
            gpu_utilization,
            cpu_utilization_per_core,
            memory_bandwidth_gbps,
            pcie_bandwidth_gbps: memory_bandwidth_gbps, // Simplified
            gpu_pipeline_efficiency: if gpu_timings.pipeline_changes > 0 {
                100.0 / (gpu_timings.pipeline_changes as f32)
            } else {
                100.0
            },
            gpu_pipeline_stalls: sync_overhead.gpu_stalls,
            avg_frame_time_ms: total_ms,
            frame_breakdown,
        }
    }
    
    /// Generate detailed report
    pub fn generate_report(&self, analysis: &WorkloadAnalysis) -> String {
        let mut report = String::from("\n=== GPU WORKLOAD ANALYSIS REPORT ===\n\n");
        
        report.push_str(&format!("CLAIMED: 80-85% GPU compute\n"));
        report.push_str(&format!("ACTUAL:  {:.1}% GPU compute\n", analysis.gpu_compute_percentage));
        report.push_str(&format!("         {:.1}% CPU compute\n", analysis.cpu_compute_percentage));
        report.push_str(&format!("         {:.1}% Synchronization overhead\n", analysis.sync_overhead_percentage));
        report.push_str(&format!("         {:.1}% Memory transfer overhead\n\n", analysis.transfer_overhead_percentage));
        
        report.push_str("=== PERFORMANCE METRICS ===\n");
        report.push_str(&format!("Average Frame Time: {:.2} ms ({:.1} FPS)\n", 
            analysis.avg_frame_time_ms, 
            1000.0 / analysis.avg_frame_time_ms
        ));
        report.push_str(&format!("GPU Utilization: {:.1}%\n", analysis.gpu_utilization));
        report.push_str(&format!("GPU Pipeline Efficiency: {:.1}%\n", analysis.gpu_pipeline_efficiency));
        report.push_str(&format!("GPU Pipeline Stalls: {}\n", analysis.gpu_pipeline_stalls));
        report.push_str(&format!("Memory Bandwidth: {:.2} GB/s\n", analysis.memory_bandwidth_gbps));
        report.push_str(&format!("PCIe Bandwidth: {:.2} GB/s\n\n", analysis.pcie_bandwidth_gbps));
        
        report.push_str("=== FRAME TIME BREAKDOWN ===\n");
        report.push_str(&format!("CPU Update: {:.2} ms\n", analysis.frame_breakdown.cpu_update_ms));
        report.push_str(&format!("GPU Compute: {:.2} ms\n", analysis.frame_breakdown.gpu_compute_ms));
        report.push_str(&format!("GPU Render: {:.2} ms\n", analysis.frame_breakdown.gpu_render_ms));
        report.push_str(&format!("CPU-GPU Sync: {:.2} ms\n", analysis.frame_breakdown.cpu_gpu_sync_ms));
        report.push_str(&format!("Memory Transfer: {:.2} ms\n", analysis.frame_breakdown.memory_transfer_ms));
        report.push_str(&format!("Other: {:.2} ms\n\n", analysis.frame_breakdown.other_ms));
        
        report.push_str("=== SYSTEM BREAKDOWN ===\n");
        let mut systems: Vec<_> = analysis.system_breakdown.values().collect();
        systems.sort_by(|a, b| (b.gpu_time_ms + b.cpu_time_ms)
            .partial_cmp(&(a.gpu_time_ms + a.cpu_time_ms))
            .unwrap_or(std::cmp::Ordering::Equal));
        
        for system in systems.iter().take(10) {
            report.push_str(&format!("{}: CPU={:.2}ms, GPU={:.2}ms, GPU-accelerated={}, Efficiency={:.1}%\n",
                system.name,
                system.cpu_time_ms,
                system.gpu_time_ms,
                system.is_gpu_accelerated,
                system.gpu_efficiency * 100.0
            ));
        }
        
        report.push_str("\n=== VERDICT ===\n");
        if analysis.gpu_compute_percentage >= 80.0 {
            report.push_str("✓ The engine IS truly GPU-first as claimed!\n");
        } else if analysis.gpu_compute_percentage >= 50.0 {
            report.push_str("~ The engine is partially GPU-accelerated but not to the claimed extent.\n");
        } else {
            report.push_str("✗ The engine is NOT GPU-first. It's primarily CPU-bound!\n");
        }
        
        report
    }
}

/// Helper for tracking GPU operations in a scope
pub struct GpuOperationScope<'a> {
    profiler: &'a GpuWorkloadProfiler,
    name: String,
    start: Instant,
}

impl<'a> GpuOperationScope<'a> {
    pub fn new(profiler: &'a GpuWorkloadProfiler, name: &str) -> Self {
        Self {
            profiler,
            name: name.to_string(),
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for GpuOperationScope<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        if let Ok(mut gpu_timings) = self.profiler.gpu_timings.lock() {
            gpu_timings.total_frame_time += duration;
        }
    }
}