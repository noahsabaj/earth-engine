use std::collections::VecDeque;
use std::time::{Duration, Instant};
use wgpu::Queue;

/// Performance metrics for fluid simulation
#[derive(Debug, Clone)]
pub struct FluidPerformanceMetrics {
    /// Frame time in milliseconds
    pub frame_time_ms: f32,
    
    /// Frames per second
    pub fps: f32,
    
    /// Fluid update time (compute)
    pub fluid_update_ms: f32,
    
    /// Pressure solver time
    pub pressure_solve_ms: f32,
    
    /// Rendering time
    pub render_time_ms: f32,
    
    /// Active fluid voxel count
    pub active_voxels: u32,
    
    /// Memory usage in MB
    pub memory_usage_mb: f32,
}

impl Default for FluidPerformanceMetrics {
    fn default() -> Self {
        Self {
            frame_time_ms: 16.67, // Target 60 FPS
            fps: 60.0,
            fluid_update_ms: 0.0,
            pressure_solve_ms: 0.0,
            render_time_ms: 0.0,
            active_voxels: 0,
            memory_usage_mb: 0.0,
        }
    }
}

/// Performance monitor for fluid system
pub struct FluidPerformanceMonitor {
    /// Frame time history
    frame_times: VecDeque<Duration>,
    
    /// Update time history
    update_times: VecDeque<Duration>,
    
    /// Solver time history
    solver_times: VecDeque<Duration>,
    
    /// Render time history
    render_times: VecDeque<Duration>,
    
    /// History size
    history_size: usize,
    
    /// Last frame timestamp
    last_frame: Instant,
    
    /// Current metrics
    current_metrics: FluidPerformanceMetrics,
    
    /// Performance warnings enabled
    warnings_enabled: bool,
}

impl FluidPerformanceMonitor {
    /// Create new performance monitor
    pub fn new() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(120),
            update_times: VecDeque::with_capacity(120),
            solver_times: VecDeque::with_capacity(120),
            render_times: VecDeque::with_capacity(120),
            history_size: 120, // 2 seconds at 60 FPS
            last_frame: Instant::now(),
            current_metrics: FluidPerformanceMetrics::default(),
            warnings_enabled: true,
        }
    }
    
    /// Get current metrics
    pub fn get_metrics(&self) -> &FluidPerformanceMetrics {
        &self.current_metrics
    }
    
    /// Check if performance is within target
    pub fn check_performance(&self) -> PerformanceStatus {
        if self.current_metrics.fps >= 60.0 {
            PerformanceStatus::Good
        } else if self.current_metrics.fps >= 30.0 {
            PerformanceStatus::Acceptable
        } else {
            PerformanceStatus::Poor
        }
    }
    
    /// Get optimization suggestions
    pub fn get_suggestions(&self) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();
        
        // Check frame time breakdown
        let total_compute = self.current_metrics.fluid_update_ms + self.current_metrics.pressure_solve_ms;
        
        if self.current_metrics.pressure_solve_ms > 8.0 {
            suggestions.push(OptimizationSuggestion::ReducePressureIterations);
        }
        
        if self.current_metrics.active_voxels > 1_000_000 {
            suggestions.push(OptimizationSuggestion::ReduceActiveVolume);
        }
        
        if total_compute > 12.0 {
            suggestions.push(OptimizationSuggestion::EnableGpuOptimizations);
        }
        
        if self.current_metrics.render_time_ms > 4.0 {
            suggestions.push(OptimizationSuggestion::SimplifyRendering);
        }
        
        suggestions
    }
    
}

/// Start frame timing (DOP)
pub fn begin_frame(monitor: &mut FluidPerformanceMonitor) {
    let now = Instant::now();
    let frame_time = now - monitor.last_frame;
    monitor.last_frame = now;
    
    // Add to history
    monitor.frame_times.push_back(frame_time);
    if monitor.frame_times.len() > monitor.history_size {
        monitor.frame_times.pop_front();
    }
    
    // Update metrics
    update_metrics(monitor);
}

/// Record fluid update time (DOP)
pub fn record_update_time(monitor: &mut FluidPerformanceMonitor, duration: Duration) {
    monitor.update_times.push_back(duration);
    if monitor.update_times.len() > monitor.history_size {
        monitor.update_times.pop_front();
    }
}

/// Record pressure solver time (DOP)
pub fn record_solver_time(monitor: &mut FluidPerformanceMonitor, duration: Duration) {
    monitor.solver_times.push_back(duration);
    if monitor.solver_times.len() > monitor.history_size {
        monitor.solver_times.pop_front();
    }
}

/// Record render time (DOP)
pub fn record_render_time(monitor: &mut FluidPerformanceMonitor, duration: Duration) {
    monitor.render_times.push_back(duration);
    if monitor.render_times.len() > monitor.history_size {
        monitor.render_times.pop_front();
    }
}

/// Update active voxel count (DOP)
pub fn set_active_voxels(monitor: &mut FluidPerformanceMonitor, count: u32) {
    monitor.current_metrics.active_voxels = count;
}

/// Update memory usage (DOP)
pub fn set_memory_usage(monitor: &mut FluidPerformanceMonitor, bytes: usize) {
    monitor.current_metrics.memory_usage_mb = bytes as f32 / (1024.0 * 1024.0);
}

/// Update internal metrics (DOP)
fn update_metrics(monitor: &mut FluidPerformanceMonitor) {
    // Calculate averages
    if !monitor.frame_times.is_empty() {
        let avg_frame_time: Duration = monitor.frame_times.iter().sum::<Duration>() / monitor.frame_times.len() as u32;
        monitor.current_metrics.frame_time_ms = avg_frame_time.as_secs_f32() * 1000.0;
        monitor.current_metrics.fps = 1000.0 / monitor.current_metrics.frame_time_ms;
    }
    
    if !monitor.update_times.is_empty() {
        let avg_update_time: Duration = monitor.update_times.iter().sum::<Duration>() / monitor.update_times.len() as u32;
        monitor.current_metrics.fluid_update_ms = avg_update_time.as_secs_f32() * 1000.0;
    }
    
    if !monitor.solver_times.is_empty() {
        let avg_solver_time: Duration = monitor.solver_times.iter().sum::<Duration>() / monitor.solver_times.len() as u32;
        monitor.current_metrics.pressure_solve_ms = avg_solver_time.as_secs_f32() * 1000.0;
    }
    
    if !monitor.render_times.is_empty() {
        let avg_render_time: Duration = monitor.render_times.iter().sum::<Duration>() / monitor.render_times.len() as u32;
        monitor.current_metrics.render_time_ms = avg_render_time.as_secs_f32() * 1000.0;
    }
    
    // Log warnings if enabled
    if monitor.warnings_enabled && monitor.current_metrics.fps < 60.0 {
        log::warn!("Fluid performance below target: {:.1} FPS", monitor.current_metrics.fps);
    }
}

/// Performance status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PerformanceStatus {
    Good,       // 60+ FPS
    Acceptable, // 30-60 FPS
    Poor,       // <30 FPS
}

/// Optimization suggestions
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationSuggestion {
    ReducePressureIterations,
    ReduceActiveVolume,
    EnableGpuOptimizations,
    SimplifyRendering,
    LowerFluidResolution,
}

/// GPU timer queries for precise timing
pub struct GpuTimer {
    query_set: Option<wgpu::QuerySet>,
    query_buffer: Option<wgpu::Buffer>,
    staging_buffer: Option<wgpu::Buffer>,
    timestamp_period: f32,
}

impl GpuTimer {
    /// Create new GPU timer
    pub fn new(device: &wgpu::Device, queue: &Queue) -> Self {
        // Check if timestamp queries are supported
        let features = device.features();
        if !features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            return Self {
                query_set: None,
                query_buffer: None,
                staging_buffer: None,
                timestamp_period: 1.0,
            };
        }
        
        // Create query set
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("Fluid GPU Timer"),
            ty: wgpu::QueryType::Timestamp,
            count: 8, // Start/end for each stage
        });
        
        // Create buffers
        let query_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Timer Query Buffer"),
            size: 8 * 8, // 8 queries * 8 bytes
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Timer Staging Buffer"),
            size: 8 * 8,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        Self {
            query_set: Some(query_set),
            query_buffer: Some(query_buffer),
            staging_buffer: Some(staging_buffer),
            timestamp_period: queue.get_timestamp_period(),
        }
    }
    
    /// Write timestamp
    pub fn write_timestamp(&self, encoder: &mut wgpu::CommandEncoder, index: u32) {
        if let Some(query_set) = &self.query_set {
            encoder.write_timestamp(query_set, index);
        }
    }
    
    /// Resolve queries and get results
    pub async fn get_results(&self, device: &wgpu::Device, queue: &Queue) -> Option<Vec<f32>> {
        let query_set = self.query_set.as_ref()?;
        let query_buffer = self.query_buffer.as_ref()?;
        let staging_buffer = self.staging_buffer.as_ref()?;
        
        // Resolve queries
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GPU Timer Resolve"),
        });
        
        encoder.resolve_query_set(query_set, 0..8, query_buffer, 0);
        encoder.copy_buffer_to_buffer(query_buffer, 0, staging_buffer, 0, 8 * 8);
        
        queue.submit(Some(encoder.finish()));
        
        // Read results
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            // Ignore send errors - receiver may have been dropped
            let _ = sender.send(result);
        });
        
        device.poll(wgpu::Maintain::Wait);
        receiver.await.ok()?.ok()?;
        
        let data = buffer_slice.get_mapped_range();
        let timestamps: Vec<u64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();
        
        // Convert to milliseconds
        let mut times = Vec::new();
        for i in (0..timestamps.len()).step_by(2) {
            let start = timestamps[i] as f32 * self.timestamp_period;
            let end = timestamps[i + 1] as f32 * self.timestamp_period;
            times.push((end - start) / 1_000_000.0); // Convert to ms
        }
        
        Some(times)
    }
}