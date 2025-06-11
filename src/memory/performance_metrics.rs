/// Performance Metrics Module
/// 
/// Provides comprehensive performance comparison between
/// legacy CPU-based and new GPU-based systems.

use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Performance metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricType {
    /// Chunk migration from CPU to GPU
    ChunkMigration,
    /// Light propagation calculation
    LightPropagation,
    /// World modification (block placement/breaking)
    WorldModification,
    /// Memory allocation
    MemoryAllocation,
    /// Terrain generation
    TerrainGeneration,
    /// Total frame time
    FrameTime,
}

/// Single performance measurement
#[derive(Debug, Clone)]
pub struct Measurement {
    /// Type of metric
    metric_type: MetricType,
    /// Implementation used (CPU or GPU)
    implementation: Implementation,
    /// Duration of the operation
    duration: Duration,
    /// Amount of work done (e.g., chunks processed)
    work_units: u64,
    /// Additional context
    context: String,
    /// Timestamp
    timestamp: Instant,
}

/// Implementation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Implementation {
    /// Legacy CPU-based implementation
    Cpu,
    /// New GPU-based implementation
    Gpu,
}

/// Performance comparison result
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub metric_type: MetricType,
    pub cpu_avg_duration: Duration,
    pub gpu_avg_duration: Duration,
    pub speedup_factor: f64,
    pub cpu_throughput: f64, // work units per second
    pub gpu_throughput: f64,
    pub sample_count: usize,
}

/// Performance metrics collector
pub struct PerformanceMetrics {
    measurements: Arc<Mutex<Vec<Measurement>>>,
    start_time: Instant,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            measurements: Arc::new(Mutex::new(Vec::new())),
            start_time: Instant::now(),
        }
    }
    
    /// Record a performance measurement
    pub fn record(
        &self,
        metric_type: MetricType,
        implementation: Implementation,
        duration: Duration,
        work_units: u64,
        context: &str,
    ) {
        let measurement = Measurement {
            metric_type,
            implementation,
            duration,
            work_units,
            context: context.to_string(),
            timestamp: Instant::now(),
        };
        
        match self.measurements.lock() {
            Ok(mut measurements) => measurements.push(measurement),
            Err(e) => eprintln!("Failed to lock measurements: {}", e),
        }
    }
    
    /// Start a scoped measurement
    pub fn start_measurement(
        &self,
        metric_type: MetricType,
        implementation: Implementation,
        work_units: u64,
        context: &str,
    ) -> ScopedMeasurement {
        ScopedMeasurement::new(
            self.clone(),
            metric_type,
            implementation,
            work_units,
            context.to_string(),
        )
    }
    
    /// Get comparison results for all metrics
    pub fn get_comparisons(&self) -> Vec<ComparisonResult> {
        let measurements = match self.measurements.lock() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to lock measurements: {}", e);
                return Vec::new();
            }
        };
        let mut results = Vec::new();
        
        // Group measurements by metric type
        let mut by_metric: HashMap<MetricType, Vec<&Measurement>> = HashMap::new();
        for measurement in measurements.iter() {
            by_metric.entry(measurement.metric_type)
                .or_insert_with(Vec::new)
                .push(measurement);
        }
        
        // Calculate comparisons for each metric
        for (metric_type, measurements) in by_metric {
            let cpu_measurements: Vec<_> = measurements.iter()
                .filter(|m| m.implementation == Implementation::Cpu)
                .collect();
            
            let gpu_measurements: Vec<_> = measurements.iter()
                .filter(|m| m.implementation == Implementation::Gpu)
                .collect();
            
            if cpu_measurements.is_empty() || gpu_measurements.is_empty() {
                continue;
            }
            
            // Calculate averages
            let cpu_total_duration: Duration = cpu_measurements.iter()
                .map(|m| m.duration)
                .sum();
            let cpu_total_work: u64 = cpu_measurements.iter()
                .map(|m| m.work_units)
                .sum();
            let cpu_avg_duration = cpu_total_duration / cpu_measurements.len() as u32;
            let cpu_throughput = cpu_total_work as f64 / cpu_total_duration.as_secs_f64();
            
            let gpu_total_duration: Duration = gpu_measurements.iter()
                .map(|m| m.duration)
                .sum();
            let gpu_total_work: u64 = gpu_measurements.iter()
                .map(|m| m.work_units)
                .sum();
            let gpu_avg_duration = gpu_total_duration / gpu_measurements.len() as u32;
            let gpu_throughput = gpu_total_work as f64 / gpu_total_duration.as_secs_f64();
            
            // Calculate speedup
            let speedup_factor = cpu_avg_duration.as_secs_f64() / gpu_avg_duration.as_secs_f64();
            
            results.push(ComparisonResult {
                metric_type,
                cpu_avg_duration,
                gpu_avg_duration,
                speedup_factor,
                cpu_throughput,
                gpu_throughput,
                sample_count: cpu_measurements.len().min(gpu_measurements.len()),
            });
        }
        
        results.sort_by(|a, b| {
            b.speedup_factor.partial_cmp(&a.speedup_factor)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }
    
    /// Print performance comparison report
    pub fn print_report(&self) {
        println!("\n=== Performance Comparison Report ===");
        println!("Runtime: {:.2}s", self.start_time.elapsed().as_secs_f64());
        println!();
        
        let comparisons = self.get_comparisons();
        
        if comparisons.is_empty() {
            println!("No comparison data available yet.");
            return;
        }
        
        println!("{:<20} {:>15} {:>15} {:>10} {:>15}",
                 "Metric", "CPU Avg (ms)", "GPU Avg (ms)", "Speedup", "Samples");
        println!("{:-<75}", "");
        
        for comparison in &comparisons {
            println!("{:<20} {:>15.2} {:>15.2} {:>9.1}x {:>15}",
                     format!("{:?}", comparison.metric_type),
                     comparison.cpu_avg_duration.as_secs_f64() * 1000.0,
                     comparison.gpu_avg_duration.as_secs_f64() * 1000.0,
                     comparison.speedup_factor,
                     comparison.sample_count);
        }
        
        println!();
        println!("Throughput Comparison (work units/second):");
        println!("{:<20} {:>15} {:>15} {:>10}",
                 "Metric", "CPU", "GPU", "Improvement");
        println!("{:-<60}", "");
        
        for comparison in &comparisons {
            let throughput_improvement = comparison.gpu_throughput / comparison.cpu_throughput;
            println!("{:<20} {:>15.0} {:>15.0} {:>9.1}x",
                     format!("{:?}", comparison.metric_type),
                     comparison.cpu_throughput,
                     comparison.gpu_throughput,
                     throughput_improvement);
        }
        
        // Overall summary
        let avg_speedup = comparisons.iter()
            .map(|c| c.speedup_factor)
            .sum::<f64>() / comparisons.len() as f64;
        
        println!();
        println!("=== Summary ===");
        println!("Average speedup: {:.1}x", avg_speedup);
        println!("Best improvement: {:?} ({:.1}x)",
                 comparisons[0].metric_type,
                 comparisons[0].speedup_factor);
    }
    
    /// Clear all measurements
    pub fn clear(&self) {
        match self.measurements.lock() {
            Ok(mut measurements) => measurements.clear(),
            Err(e) => eprintln!("Failed to lock measurements: {}", e),
        }
    }
}

impl Clone for PerformanceMetrics {
    fn clone(&self) -> Self {
        Self {
            measurements: self.measurements.clone(),
            start_time: self.start_time,
        }
    }
}

/// Scoped measurement that records on drop
pub struct ScopedMeasurement {
    metrics: PerformanceMetrics,
    metric_type: MetricType,
    implementation: Implementation,
    work_units: u64,
    context: String,
    start: Instant,
}

impl ScopedMeasurement {
    fn new(
        metrics: PerformanceMetrics,
        metric_type: MetricType,
        implementation: Implementation,
        work_units: u64,
        context: String,
    ) -> Self {
        Self {
            metrics,
            metric_type,
            implementation,
            work_units,
            context,
            start: Instant::now(),
        }
    }
}

impl Drop for ScopedMeasurement {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.metrics.record(
            self.metric_type,
            self.implementation,
            duration,
            self.work_units,
            &self.context,
        );
    }
}

/// Macro for easy performance measurement
#[macro_export]
macro_rules! measure_performance {
    ($metrics:expr, $metric_type:expr, $impl:expr, $work:expr, $context:expr, $code:block) => {
        {
            let _measurement = $metrics.start_measurement(
                $metric_type,
                $impl,
                $work,
                $context,
            );
            $code
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_performance_comparison() {
        let metrics = PerformanceMetrics::new();
        
        // Simulate CPU measurements
        for i in 0..5 {
            metrics.record(
                MetricType::ChunkMigration,
                Implementation::Cpu,
                Duration::from_millis(100 + i * 10),
                10,
                "test",
            );
        }
        
        // Simulate GPU measurements (faster)
        for i in 0..5 {
            metrics.record(
                MetricType::ChunkMigration,
                Implementation::Gpu,
                Duration::from_millis(20 + i * 2),
                10,
                "test",
            );
        }
        
        let comparisons = metrics.get_comparisons();
        assert_eq!(comparisons.len(), 1);
        
        let comparison = &comparisons[0];
        assert!(comparison.speedup_factor > 4.0);
        assert!(comparison.speedup_factor < 6.0);
    }
}