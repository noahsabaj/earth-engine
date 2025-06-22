//! Performance monitoring for unified world system

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Performance metrics for the world system
#[derive(Debug, Clone)]
pub struct WorldPerformanceMetrics {
    pub generation_stats: GenerationStats,
    pub storage_stats: StorageStats,
    pub compute_stats: ComputeStats,
    pub overall_stats: OverallStats,
}

/// Generation performance statistics
#[derive(Debug, Clone)]
pub struct GenerationStats {
    pub chunks_generated: u64,
    pub avg_generation_time_ms: f64,
    pub peak_generation_time_ms: f64,
    pub backend: String,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Storage performance statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub memory_usage_mb: f64,
    pub peak_memory_mb: f64,
    pub chunks_loaded: usize,
    pub chunks_unloaded: u64,
    pub io_operations: u64,
    pub backend: String,
}

/// Compute performance statistics
#[derive(Debug, Clone)]
pub struct ComputeStats {
    pub compute_passes: u64,
    pub avg_compute_time_ms: f64,
    pub gpu_memory_usage_mb: f64,
    pub shader_compilations: u64,
    pub enabled: bool,
}

/// Overall system statistics
#[derive(Debug, Clone)]
pub struct OverallStats {
    pub uptime: Duration,
    pub avg_frame_time_ms: f64,
    pub peak_frame_time_ms: f64,
    pub memory_allocations: u64,
    pub memory_deallocations: u64,
}

/// Performance monitor for the unified world system
pub struct PerformanceMonitor {
    start_time: Instant,
    last_update: Instant,
    generation_times: VecDeque<Duration>,
    compute_times: VecDeque<Duration>,
    frame_times: VecDeque<Duration>,
    metrics: WorldPerformanceMetrics,
    config: MonitorConfig,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(config: MonitorConfig) -> Self {
        let now = Instant::now();

        Self {
            start_time: now,
            last_update: now,
            generation_times: VecDeque::with_capacity(config.sample_size),
            compute_times: VecDeque::with_capacity(config.sample_size),
            frame_times: VecDeque::with_capacity(config.sample_size),
            metrics: WorldPerformanceMetrics::default(),
            config,
        }
    }

    /// Record a chunk generation time
    pub fn record_generation_time(&mut self, duration: Duration) {
        self.generation_times.push_back(duration);
        if self.generation_times.len() > self.config.sample_size {
            self.generation_times.pop_front();
        }

        self.update_generation_stats();
    }

    /// Record a compute pass time
    pub fn record_compute_time(&mut self, duration: Duration) {
        self.compute_times.push_back(duration);
        if self.compute_times.len() > self.config.sample_size {
            self.compute_times.pop_front();
        }

        self.update_compute_stats();
    }

    /// Record a frame time
    pub fn record_frame_time(&mut self, duration: Duration) {
        self.frame_times.push_back(duration);
        if self.frame_times.len() > self.config.sample_size {
            self.frame_times.pop_front();
        }

        self.update_overall_stats();
    }

    /// Update storage statistics
    pub fn update_storage_stats(&mut self, stats: StorageStats) {
        self.metrics.storage_stats = stats;
    }

    /// Get current metrics
    pub fn metrics(&self) -> &WorldPerformanceMetrics {
        &self.metrics
    }

    /// Get metrics with auto-update
    pub fn get_current_metrics(&mut self) -> &WorldPerformanceMetrics {
        self.update_all_stats();
        &self.metrics
    }

    /// Update all statistics
    fn update_all_stats(&mut self) {
        self.update_generation_stats();
        self.update_compute_stats();
        self.update_overall_stats();
    }

    /// Update generation statistics
    fn update_generation_stats(&mut self) {
        if !self.generation_times.is_empty() {
            let total_ms: f64 = self
                .generation_times
                .iter()
                .map(|d| d.as_secs_f64() * 1000.0)
                .sum();

            let avg_ms = total_ms / self.generation_times.len() as f64;
            let peak_ms = self
                .generation_times
                .iter()
                .map(|d| d.as_secs_f64() * 1000.0)
                .fold(0.0, f64::max);

            self.metrics.generation_stats.avg_generation_time_ms = avg_ms;
            self.metrics.generation_stats.peak_generation_time_ms = peak_ms;
        }
    }

    /// Update compute statistics
    fn update_compute_stats(&mut self) {
        if !self.compute_times.is_empty() {
            let total_ms: f64 = self
                .compute_times
                .iter()
                .map(|d| d.as_secs_f64() * 1000.0)
                .sum();

            let avg_ms = total_ms / self.compute_times.len() as f64;

            self.metrics.compute_stats.avg_compute_time_ms = avg_ms;
            self.metrics.compute_stats.compute_passes += 1;
        }
    }

    /// Update overall statistics
    fn update_overall_stats(&mut self) {
        let now = Instant::now();
        self.metrics.overall_stats.uptime = now - self.start_time;

        if !self.frame_times.is_empty() {
            let total_ms: f64 = self
                .frame_times
                .iter()
                .map(|d| d.as_secs_f64() * 1000.0)
                .sum();

            let avg_ms = total_ms / self.frame_times.len() as f64;
            let peak_ms = self
                .frame_times
                .iter()
                .map(|d| d.as_secs_f64() * 1000.0)
                .fold(0.0, f64::max);

            self.metrics.overall_stats.avg_frame_time_ms = avg_ms;
            self.metrics.overall_stats.peak_frame_time_ms = peak_ms;
        }

        self.last_update = now;
    }

    /// Generate a performance report
    pub fn generate_report(&mut self) -> String {
        self.update_all_stats();

        format!(
            "World Performance Report\n\
             ========================\n\
             Uptime: {:.2}s\n\
             Generation: {:.2}ms avg, {:.2}ms peak ({})\n\
             Compute: {:.2}ms avg, {} passes\n\
             Memory: {:.1}MB / {:.1}MB peak\n\
             Chunks: {} loaded\n\
             Frame: {:.2}ms avg, {:.2}ms peak\n",
            self.metrics.overall_stats.uptime.as_secs_f64(),
            self.metrics.generation_stats.avg_generation_time_ms,
            self.metrics.generation_stats.peak_generation_time_ms,
            self.metrics.generation_stats.backend,
            self.metrics.compute_stats.avg_compute_time_ms,
            self.metrics.compute_stats.compute_passes,
            self.metrics.storage_stats.memory_usage_mb,
            self.metrics.storage_stats.peak_memory_mb,
            self.metrics.storage_stats.chunks_loaded,
            self.metrics.overall_stats.avg_frame_time_ms,
            self.metrics.overall_stats.peak_frame_time_ms,
        )
    }
}

/// Configuration for performance monitoring
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub sample_size: usize,
    pub update_interval_ms: u64,
    pub enable_detailed_stats: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            sample_size: 60, // 1 second at 60 FPS
            update_interval_ms: 1000,
            enable_detailed_stats: true,
        }
    }
}

impl Default for WorldPerformanceMetrics {
    fn default() -> Self {
        Self {
            generation_stats: GenerationStats::default(),
            storage_stats: StorageStats::default(),
            compute_stats: ComputeStats::default(),
            overall_stats: OverallStats::default(),
        }
    }
}

impl Default for GenerationStats {
    fn default() -> Self {
        Self {
            chunks_generated: 0,
            avg_generation_time_ms: 0.0,
            peak_generation_time_ms: 0.0,
            backend: "Unknown".to_string(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}

impl Default for StorageStats {
    fn default() -> Self {
        Self {
            memory_usage_mb: 0.0,
            peak_memory_mb: 0.0,
            chunks_loaded: 0,
            chunks_unloaded: 0,
            io_operations: 0,
            backend: "Unknown".to_string(),
        }
    }
}

impl Default for ComputeStats {
    fn default() -> Self {
        Self {
            compute_passes: 0,
            avg_compute_time_ms: 0.0,
            gpu_memory_usage_mb: 0.0,
            shader_compilations: 0,
            enabled: false,
        }
    }
}

impl Default for OverallStats {
    fn default() -> Self {
        Self {
            uptime: Duration::ZERO,
            avg_frame_time_ms: 0.0,
            peak_frame_time_ms: 0.0,
            memory_allocations: 0,
            memory_deallocations: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new(MonitorConfig::default());
        assert_eq!(monitor.metrics().generation_stats.chunks_generated, 0);
    }

    #[test]
    fn test_generation_time_recording() {
        let mut monitor = PerformanceMonitor::new(MonitorConfig::default());
        monitor.record_generation_time(Duration::from_millis(10));
        monitor.record_generation_time(Duration::from_millis(20));

        let metrics = monitor.get_current_metrics();
        assert_eq!(metrics.generation_stats.avg_generation_time_ms, 15.0);
        assert_eq!(metrics.generation_stats.peak_generation_time_ms, 20.0);
    }
}
