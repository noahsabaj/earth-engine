/// System Monitor
///
/// Comprehensive system health monitoring and diagnostics for the Hearth Engine.
/// Tracks performance metrics, resource usage, error rates, and system coordination.
///
/// This provides observability into system integration bottlenecks and helps
/// identify performance issues before they impact gameplay.
use crate::error::{EngineError, EngineResult};
use crate::thread_pool::{PoolCategory, ThreadPoolManager};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// System identifier for monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonitoredSystem {
    WorldGeneration,
    Physics,
    Renderer,
    Lighting,
    Network,
    Persistence,
    Audio,
    Input,
    UI,
    Particles,
    Weather,
    ThreadPool,
    Memory,
    GPU,
}

/// System health status levels
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HealthStatus {
    Critical = 0,  // System failure or severe degradation
    Warning = 1,   // Performance issues or concerning trends
    Good = 2,      // Normal operation with minor issues
    Excellent = 3, // Optimal performance
}

/// Performance metric types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    FrameTime,
    MemoryUsage,
    CPUUsage,
    GPUUsage,
    ThreadUtilization,
    ErrorRate,
    Throughput,
    Latency,
    QueueDepth,
    CacheHitRatio,
}

/// Real-time performance metric
#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    pub metric_type: MetricType,
    pub value: f64,
    pub unit: String,
    pub timestamp: Instant,
    pub system: MonitoredSystem,
}

/// Historical performance data
#[derive(Debug, Clone)]
pub struct MetricHistory {
    pub values: VecDeque<f64>,
    pub timestamps: VecDeque<Instant>,
    pub max_samples: usize,
    pub average: f64,
    pub min: f64,
    pub max: f64,
    pub trend: TrendDirection,
}

/// Trend analysis for metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Volatile,
}

/// System alert levels
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertLevel {
    Info = 0,
    Warning = 1,
    Error = 2,
    Critical = 3,
}

/// System alert
#[derive(Debug, Clone)]
pub struct SystemAlert {
    pub id: u64,
    pub level: AlertLevel,
    pub system: MonitoredSystem,
    pub message: String,
    pub timestamp: Instant,
    pub metric_type: Option<MetricType>,
    pub metric_value: Option<f64>,
    pub threshold: Option<f64>,
}

/// Comprehensive system health report
#[derive(Debug, Clone)]
pub struct SystemHealthReport {
    pub overall_status: HealthStatus,
    pub system_statuses: HashMap<MonitoredSystem, HealthStatus>,
    pub active_alerts: Vec<SystemAlert>,
    pub performance_summary: PerformanceSummary,
    pub resource_usage: ResourceUsage,
    pub trend_analysis: TrendAnalysis,
    pub recommendations: Vec<String>,
    pub report_timestamp: Instant,
}

/// Performance summary across all systems
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub average_frame_time_ms: f64,
    pub frame_rate: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub gpu_usage_percent: f64,
    pub thread_pool_utilization: f64,
    pub error_rate_per_minute: f64,
}

/// Resource usage tracking
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub total_memory_mb: f64,
    pub available_memory_mb: f64,
    pub gpu_memory_mb: f64,
    pub thread_count: usize,
    pub file_handles: usize,
    pub network_connections: usize,
}

/// Trend analysis across metrics
#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    pub performance_trend: TrendDirection,
    pub memory_trend: TrendDirection,
    pub error_trend: TrendDirection,
    pub stability_score: f64,       // 0.0 to 1.0
    pub prediction_confidence: f64, // 0.0 to 1.0
}

/// Performance thresholds for alerting
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    pub frame_time_warning_ms: f64,
    pub frame_time_critical_ms: f64,
    pub memory_warning_percent: f64,
    pub memory_critical_percent: f64,
    pub cpu_warning_percent: f64,
    pub cpu_critical_percent: f64,
    pub error_rate_warning: f64,
    pub error_rate_critical: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            frame_time_warning_ms: 20.0,  // 50 FPS
            frame_time_critical_ms: 33.3, // 30 FPS
            memory_warning_percent: 75.0,
            memory_critical_percent: 90.0,
            cpu_warning_percent: 80.0,
            cpu_critical_percent: 95.0,
            error_rate_warning: 1.0,  // 1 error per minute
            error_rate_critical: 5.0, // 5 errors per minute
        }
    }
}

/// Main system monitor
pub struct SystemMonitor {
    /// Performance metrics history
    metrics: RwLock<HashMap<(MonitoredSystem, MetricType), MetricHistory>>,

    /// Active alerts
    alerts: RwLock<Vec<SystemAlert>>,

    /// Alert counter for unique IDs
    next_alert_id: std::sync::atomic::AtomicU64,

    /// Performance thresholds
    thresholds: PerformanceThresholds,

    /// System health status
    system_health: RwLock<HashMap<MonitoredSystem, HealthStatus>>,

    /// Error tracking
    error_history: RwLock<HashMap<MonitoredSystem, VecDeque<Instant>>>,

    /// Performance profiler
    profiler: Arc<PerformanceProfiler>,

    /// Resource monitor
    resource_monitor: Arc<ResourceMonitor>,

    /// Monitoring enabled flag
    enabled: std::sync::atomic::AtomicBool,

    /// Last health check timestamp
    last_health_check: RwLock<Instant>,
}

/// Performance profiler for detailed timing analysis
pub struct PerformanceProfiler {
    /// Active timing sessions
    active_sessions: RwLock<HashMap<String, ProfilingSession>>,

    /// Completed sessions history
    session_history: RwLock<VecDeque<CompletedSession>>,

    /// Maximum sessions to keep in history
    max_history: usize,
}

/// Active profiling session
#[derive(Debug)]
pub struct ProfilingSession {
    pub name: String,
    pub start_time: Instant,
    pub system: MonitoredSystem,
    pub nested_sessions: Vec<ProfilingSession>,
}

/// Completed profiling session
#[derive(Debug, Clone)]
pub struct CompletedSession {
    pub name: String,
    pub duration: Duration,
    pub system: MonitoredSystem,
    pub timestamp: Instant,
    pub nested_durations: Vec<(String, Duration)>,
}

/// Resource monitor for system resources
pub struct ResourceMonitor {
    /// System memory information
    memory_info: RwLock<MemoryInfo>,

    /// Thread pool statistics
    thread_stats: RwLock<ThreadPoolStats>,

    /// GPU information (if available)
    gpu_info: RwLock<Option<GpuInfo>>,

    /// File system usage
    filesystem_info: RwLock<FileSystemInfo>,
}

/// Memory usage information
#[derive(Debug, Clone, Default)]
pub struct MemoryInfo {
    pub total_system_mb: f64,
    pub available_system_mb: f64,
    pub process_usage_mb: f64,
    pub heap_usage_mb: f64,
    pub stack_usage_mb: f64,
}

/// Thread pool statistics
#[derive(Debug, Clone, Default)]
pub struct ThreadPoolStats {
    pub total_threads: usize,
    pub active_threads: usize,
    pub queued_tasks: usize,
    pub completed_tasks: u64,
    pub average_task_time_ms: f64,
}

/// GPU information
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub total_memory_mb: f64,
    pub used_memory_mb: f64,
    pub utilization_percent: f64,
    pub temperature_celsius: f64,
}

/// File system information
#[derive(Debug, Clone, Default)]
pub struct FileSystemInfo {
    pub total_space_gb: f64,
    pub available_space_gb: f64,
    pub open_file_handles: usize,
}

impl SystemMonitor {
    /// Create a new system monitor
    pub fn new() -> Self {
        Self {
            metrics: RwLock::new(HashMap::new()),
            alerts: RwLock::new(Vec::new()),
            next_alert_id: std::sync::atomic::AtomicU64::new(1),
            thresholds: PerformanceThresholds::default(),
            system_health: RwLock::new(HashMap::new()),
            error_history: RwLock::new(HashMap::new()),
            profiler: Arc::new(PerformanceProfiler::new(1000)),
            resource_monitor: Arc::new(ResourceMonitor::new()),
            enabled: std::sync::atomic::AtomicBool::new(true),
            last_health_check: RwLock::new(Instant::now()),
        }
    }

    /// Record a performance metric
    pub fn record_metric(&self, system: MonitoredSystem, metric_type: MetricType, value: f64) {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let mut metrics = self.metrics.write();
        let key = (system, metric_type);

        let history = metrics
            .entry(key)
            .or_insert_with(|| MetricHistory::new(300)); // 5 minutes at 60fps
        history.add_value(value, Instant::now());

        // Check for threshold violations
        self.check_thresholds(system, metric_type, value);

        // Update system health based on this metric
        self.update_system_health(system, metric_type, value);
    }

    /// Start a profiling session
    pub fn start_profiling(&self, name: String, system: MonitoredSystem) {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let session = ProfilingSession {
            name: name.clone(),
            start_time: Instant::now(),
            system,
            nested_sessions: Vec::new(),
        };

        self.profiler.active_sessions.write().insert(name, session);
    }

    /// End a profiling session
    pub fn end_profiling(&self, name: &str) -> Option<Duration> {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }

        let mut active_sessions = self.profiler.active_sessions.write();
        if let Some(session) = active_sessions.remove(name) {
            let duration = session.start_time.elapsed();

            let completed = CompletedSession {
                name: session.name.clone(),
                duration,
                system: session.system,
                timestamp: Instant::now(),
                nested_durations: session
                    .nested_sessions
                    .iter()
                    .map(|nested| (nested.name.clone(), nested.start_time.elapsed()))
                    .collect(),
            };

            let mut history = self.profiler.session_history.write();
            history.push_back(completed);
            if history.len() > self.profiler.max_history {
                history.pop_front();
            }

            // Record as metric
            self.record_metric(
                session.system,
                MetricType::FrameTime,
                duration.as_secs_f64() * 1000.0,
            );

            Some(duration)
        } else {
            None
        }
    }

    /// Record a system error
    pub fn record_error(&self, system: MonitoredSystem, error: &EngineError) {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let now = Instant::now();

        // Add to error history
        {
            let mut error_history = self.error_history.write();
            let errors = error_history.entry(system).or_insert_with(VecDeque::new);
            errors.push_back(now);

            // Keep only last 10 minutes of errors
            let cutoff = now - Duration::from_secs(600);
            while errors.front().map_or(false, |&time| time < cutoff) {
                errors.pop_front();
            }
        }

        // Create alert
        let alert_id = self
            .next_alert_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let alert = SystemAlert {
            id: alert_id,
            level: AlertLevel::Error,
            system,
            message: format!("System error: {}", error),
            timestamp: now,
            metric_type: None,
            metric_value: None,
            threshold: None,
        };

        self.alerts.write().push(alert);

        // Update system health
        self.degrade_system_health(system);
    }

    /// Generate comprehensive health report
    pub fn generate_health_report(&self) -> SystemHealthReport {
        let now = Instant::now();

        // Update resource information
        self.resource_monitor.update_resource_info();

        // Calculate overall status
        let system_statuses = self.calculate_system_statuses();
        let overall_status = self.calculate_overall_status(&system_statuses);

        // Get active alerts
        let active_alerts = self.get_active_alerts();

        // Generate performance summary
        let performance_summary = self.generate_performance_summary();

        // Get resource usage
        let resource_usage = self.resource_monitor.get_resource_usage();

        // Analyze trends
        let trend_analysis = self.analyze_trends();

        // Generate recommendations
        let recommendations = self.generate_recommendations(&system_statuses, &performance_summary);

        *self.last_health_check.write() = now;

        SystemHealthReport {
            overall_status,
            system_statuses,
            active_alerts,
            performance_summary,
            resource_usage,
            trend_analysis,
            recommendations,
            report_timestamp: now,
        }
    }

    /// Check metric against thresholds
    fn check_thresholds(&self, system: MonitoredSystem, metric_type: MetricType, value: f64) {
        let (warning_threshold, critical_threshold) = match metric_type {
            MetricType::FrameTime => (
                self.thresholds.frame_time_warning_ms,
                self.thresholds.frame_time_critical_ms,
            ),
            MetricType::MemoryUsage => (
                self.thresholds.memory_warning_percent,
                self.thresholds.memory_critical_percent,
            ),
            MetricType::CPUUsage => (
                self.thresholds.cpu_warning_percent,
                self.thresholds.cpu_critical_percent,
            ),
            _ => return, // No thresholds defined for this metric type
        };

        let alert_level = if value >= critical_threshold {
            AlertLevel::Critical
        } else if value >= warning_threshold {
            AlertLevel::Warning
        } else {
            return; // No alert needed
        };

        let alert_id = self
            .next_alert_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let alert = SystemAlert {
            id: alert_id,
            level: alert_level,
            system,
            message: format!(
                "{:?} {} exceeded threshold",
                metric_type,
                match metric_type {
                    MetricType::FrameTime => "frame time",
                    MetricType::MemoryUsage => "memory usage",
                    MetricType::CPUUsage => "CPU usage",
                    _ => "value",
                }
            ),
            timestamp: Instant::now(),
            metric_type: Some(metric_type),
            metric_value: Some(value),
            threshold: Some(if alert_level == AlertLevel::Critical {
                critical_threshold
            } else {
                warning_threshold
            }),
        };

        self.alerts.write().push(alert);
    }

    /// Update system health based on metric
    fn update_system_health(&self, system: MonitoredSystem, metric_type: MetricType, value: f64) {
        let mut health = self.system_health.write();
        let current_health = health.entry(system).or_insert(HealthStatus::Good);

        // Simple health assessment based on metrics
        let health_impact = match metric_type {
            MetricType::FrameTime if value > self.thresholds.frame_time_critical_ms => {
                HealthStatus::Critical
            }
            MetricType::FrameTime if value > self.thresholds.frame_time_warning_ms => {
                HealthStatus::Warning
            }
            MetricType::ErrorRate if value > self.thresholds.error_rate_critical => {
                HealthStatus::Critical
            }
            MetricType::ErrorRate if value > self.thresholds.error_rate_warning => {
                HealthStatus::Warning
            }
            _ => HealthStatus::Good,
        };

        // Take the worst health status
        if health_impact < *current_health {
            *current_health = health_impact;
        }
    }

    /// Degrade system health due to error
    fn degrade_system_health(&self, system: MonitoredSystem) {
        let mut health = self.system_health.write();
        let current_health = health.entry(system).or_insert(HealthStatus::Good);

        *current_health = match *current_health {
            HealthStatus::Excellent => HealthStatus::Good,
            HealthStatus::Good => HealthStatus::Warning,
            HealthStatus::Warning => HealthStatus::Critical,
            HealthStatus::Critical => HealthStatus::Critical,
        };
    }

    /// Calculate system statuses
    fn calculate_system_statuses(&self) -> HashMap<MonitoredSystem, HealthStatus> {
        self.system_health.read().clone()
    }

    /// Calculate overall system status
    fn calculate_overall_status(
        &self,
        system_statuses: &HashMap<MonitoredSystem, HealthStatus>,
    ) -> HealthStatus {
        if system_statuses
            .values()
            .any(|&status| status == HealthStatus::Critical)
        {
            HealthStatus::Critical
        } else if system_statuses
            .values()
            .any(|&status| status == HealthStatus::Warning)
        {
            HealthStatus::Warning
        } else if system_statuses
            .values()
            .all(|&status| status == HealthStatus::Excellent)
        {
            HealthStatus::Excellent
        } else {
            HealthStatus::Good
        }
    }

    /// Get active alerts
    fn get_active_alerts(&self) -> Vec<SystemAlert> {
        let alerts = self.alerts.read();
        let cutoff = Instant::now() - Duration::from_secs(300); // 5 minutes

        alerts
            .iter()
            .filter(|alert| alert.timestamp > cutoff)
            .cloned()
            .collect()
    }

    /// Generate performance summary
    fn generate_performance_summary(&self) -> PerformanceSummary {
        let metrics = self.metrics.read();

        let frame_time = metrics
            .get(&(MonitoredSystem::Renderer, MetricType::FrameTime))
            .map(|h| h.average)
            .unwrap_or(16.67); // Default to 60 FPS

        let memory_usage = metrics
            .get(&(MonitoredSystem::Memory, MetricType::MemoryUsage))
            .map(|h| h.average)
            .unwrap_or(0.0);

        let cpu_usage = metrics
            .get(&(MonitoredSystem::ThreadPool, MetricType::CPUUsage))
            .map(|h| h.average)
            .unwrap_or(0.0);

        PerformanceSummary {
            average_frame_time_ms: frame_time,
            frame_rate: 1000.0 / frame_time,
            memory_usage_mb: memory_usage,
            cpu_usage_percent: cpu_usage,
            gpu_usage_percent: 0.0,       // TODO: Get from GPU monitor
            thread_pool_utilization: 0.0, // TODO: Get from thread pool
            error_rate_per_minute: self.calculate_error_rate(),
        }
    }

    /// Calculate current error rate
    fn calculate_error_rate(&self) -> f64 {
        let error_history = self.error_history.read();
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);

        let total_errors: usize = error_history
            .values()
            .map(|errors| {
                errors
                    .iter()
                    .filter(|&&timestamp| timestamp > one_minute_ago)
                    .count()
            })
            .sum();

        total_errors as f64
    }

    /// Analyze performance trends
    fn analyze_trends(&self) -> TrendAnalysis {
        // Simplified trend analysis - in a real implementation this would be more sophisticated
        TrendAnalysis {
            performance_trend: TrendDirection::Stable,
            memory_trend: TrendDirection::Stable,
            error_trend: TrendDirection::Stable,
            stability_score: 0.8,
            prediction_confidence: 0.7,
        }
    }

    /// Generate system recommendations
    fn generate_recommendations(
        &self,
        _system_statuses: &HashMap<MonitoredSystem, HealthStatus>,
        performance_summary: &PerformanceSummary,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if performance_summary.average_frame_time_ms > 20.0 {
            recommendations
                .push("Consider reducing render distance or graphics quality".to_string());
        }

        if performance_summary.memory_usage_mb > 2048.0 {
            recommendations.push(
                "High memory usage detected - consider increasing garbage collection frequency"
                    .to_string(),
            );
        }

        if performance_summary.error_rate_per_minute > 1.0 {
            recommendations
                .push("Elevated error rate - check system logs for recurring issues".to_string());
        }

        recommendations
    }

    /// Enable or disable monitoring
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled
            .store(enabled, std::sync::atomic::Ordering::Relaxed);
    }

    /// Check if monitoring is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl MetricHistory {
    fn new(max_samples: usize) -> Self {
        Self {
            values: VecDeque::new(),
            timestamps: VecDeque::new(),
            max_samples,
            average: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            trend: TrendDirection::Stable,
        }
    }

    fn add_value(&mut self, value: f64, timestamp: Instant) {
        self.values.push_back(value);
        self.timestamps.push_back(timestamp);

        if self.values.len() > self.max_samples {
            self.values.pop_front();
            self.timestamps.pop_front();
        }

        self.update_statistics();
    }

    fn update_statistics(&mut self) {
        if self.values.is_empty() {
            return;
        }

        self.average = self.values.iter().sum::<f64>() / self.values.len() as f64;
        self.min = self.values.iter().copied().fold(f64::MAX, f64::min);
        self.max = self.values.iter().copied().fold(f64::MIN, f64::max);

        // Simple trend analysis
        if self.values.len() >= 10 {
            let recent_avg = self.values.iter().rev().take(5).sum::<f64>() / 5.0;
            let older_avg = self.values.iter().rev().skip(5).take(5).sum::<f64>() / 5.0;

            let change_percent = (recent_avg - older_avg) / older_avg * 100.0;

            self.trend = if change_percent > 10.0 {
                TrendDirection::Degrading
            } else if change_percent < -10.0 {
                TrendDirection::Improving
            } else if change_percent.abs() > 5.0 {
                TrendDirection::Volatile
            } else {
                TrendDirection::Stable
            };
        }
    }
}

impl PerformanceProfiler {
    fn new(max_history: usize) -> Self {
        Self {
            active_sessions: RwLock::new(HashMap::new()),
            session_history: RwLock::new(VecDeque::new()),
            max_history,
        }
    }
}

impl ResourceMonitor {
    fn new() -> Self {
        Self {
            memory_info: RwLock::new(MemoryInfo::default()),
            thread_stats: RwLock::new(ThreadPoolStats::default()),
            gpu_info: RwLock::new(None),
            filesystem_info: RwLock::new(FileSystemInfo::default()),
        }
    }

    fn update_resource_info(&self) {
        // Update memory info
        {
            let mut memory = self.memory_info.write();
            // In a real implementation, this would query actual system memory
            memory.total_system_mb = 16384.0; // Placeholder
            memory.available_system_mb = 8192.0; // Placeholder
            memory.process_usage_mb = 1024.0; // Placeholder
        }

        // Update thread stats
        {
            let mut thread_stats = self.thread_stats.write();
            let pool_stats = ThreadPoolManager::global().get_stats();
            thread_stats.completed_tasks = pool_stats.tasks_completed.values().sum();
        }
    }

    fn get_resource_usage(&self) -> ResourceUsage {
        let memory = self.memory_info.read();
        let thread_stats = self.thread_stats.read();
        let filesystem = self.filesystem_info.read();

        ResourceUsage {
            total_memory_mb: memory.total_system_mb,
            available_memory_mb: memory.available_system_mb,
            gpu_memory_mb: 0.0, // TODO: Get from GPU monitor
            thread_count: thread_stats.total_threads,
            file_handles: filesystem.open_file_handles,
            network_connections: 0, // TODO: Get from network monitor
        }
    }
}

/// Convenience macro for profiling code blocks
#[macro_export]
macro_rules! profile {
    ($monitor:expr, $name:expr, $system:expr, $code:block) => {
        $monitor.start_profiling($name.to_string(), $system);
        let result = $code;
        $monitor.end_profiling($name);
        result
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_monitor_creation() {
        let monitor = SystemMonitor::new();
        assert!(monitor.is_enabled());
    }

    #[test]
    fn test_metric_recording() {
        let monitor = SystemMonitor::new();
        monitor.record_metric(MonitoredSystem::Renderer, MetricType::FrameTime, 16.67);

        let metrics = monitor.metrics.read();
        let key = (MonitoredSystem::Renderer, MetricType::FrameTime);
        assert!(metrics.contains_key(&key));
    }

    #[test]
    fn test_profiling() {
        let monitor = SystemMonitor::new();
        monitor.start_profiling("test_session".to_string(), MonitoredSystem::Renderer);

        std::thread::sleep(Duration::from_millis(10));

        let duration = monitor.end_profiling("test_session");
        assert!(duration.is_some());
        assert!(
            duration.expect("[Test] Profiling duration should be present")
                >= Duration::from_millis(10)
        );
    }

    #[test]
    fn test_health_report_generation() {
        let monitor = SystemMonitor::new();
        monitor.record_metric(MonitoredSystem::Renderer, MetricType::FrameTime, 16.67);

        let report = monitor.generate_health_report();
        assert!(!report.system_statuses.is_empty());
    }

    #[test]
    fn test_metric_history() {
        let mut history = MetricHistory::new(5);

        for i in 1..=7 {
            history.add_value(i as f64, Instant::now());
        }

        assert_eq!(history.values.len(), 5); // Should only keep last 5 values
        assert_eq!(history.average, 5.0); // Average of [3, 4, 5, 6, 7]
    }
}
