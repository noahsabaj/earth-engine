/// Memory Bandwidth Profiler
/// 
/// Tracks and analyzes GPU memory transfer performance
/// to identify bottlenecks and optimize data movement.

use std::sync::Mutex;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Single transfer record
#[derive(Debug, Clone)]
struct TransferRecord {
    /// Bytes transferred
    bytes: u64,
    
    /// Duration in microseconds
    duration_us: u64,
    
    /// Timestamp of transfer
    timestamp: Instant,
    
    /// Transfer type
    transfer_type: TransferType,
}

/// Type of memory transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferType {
    /// CPU to GPU upload
    Upload,
    /// GPU to CPU download
    Download,
    /// GPU to GPU copy
    Copy,
    /// Buffer mapping
    Map,
}

/// Transfer metrics over a time window
#[derive(Debug, Clone)]
pub struct TransferMetrics {
    /// Total bytes transferred
    pub total_bytes: u64,
    
    /// Total duration in microseconds
    pub total_duration_us: u64,
    
    /// Number of transfers
    pub transfer_count: usize,
    
    /// Average bandwidth in MB/s
    pub avg_bandwidth_mbps: f64,
    
    /// Peak bandwidth in MB/s
    pub peak_bandwidth_mbps: f64,
    
    /// Minimum bandwidth in MB/s
    pub min_bandwidth_mbps: f64,
    
    /// Breakdown by transfer type
    pub by_type: std::collections::HashMap<TransferType, TypeMetrics>,
}

/// Metrics for a specific transfer type
#[derive(Debug, Clone)]
pub struct TypeMetrics {
    pub bytes: u64,
    pub count: usize,
    pub avg_bandwidth_mbps: f64,
}

/// Bandwidth profiler for memory transfers
pub struct BandwidthProfiler {
    /// Recent transfer records (circular buffer)
    records: Mutex<VecDeque<TransferRecord>>,
    
    /// Maximum records to keep
    max_records: usize,
    
    /// Time window for metrics (seconds)
    window_duration: Duration,
    
    /// Start time for profiling
    start_time: Instant,
}

impl BandwidthProfiler {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(VecDeque::new()),
            max_records: 10000,
            window_duration: Duration::from_secs(60), // 1 minute window
            start_time: Instant::now(),
        }
    }
    
    /// Record a transfer
    pub fn record_transfer(&self, bytes: u64, duration_us: u64) {
        self.record_typed_transfer(bytes, duration_us, TransferType::Copy);
    }
    
    /// Record a typed transfer
    pub fn record_typed_transfer(&self, bytes: u64, duration_us: u64, transfer_type: TransferType) {
        let record = TransferRecord {
            bytes,
            duration_us,
            timestamp: Instant::now(),
            transfer_type,
        };
        
        let mut records = self.records.lock().unwrap();
        records.push_back(record);
        
        // Maintain max size
        while records.len() > self.max_records {
            records.pop_front();
        }
    }
    
    /// Get metrics for the current window
    pub fn get_metrics(&self) -> TransferMetrics {
        let now = Instant::now();
        let window_start = now - self.window_duration;
        
        let records = self.records.lock().unwrap();
        let recent_records: Vec<_> = records.iter()
            .filter(|r| r.timestamp >= window_start)
            .cloned()
            .collect();
        
        self.calculate_metrics(&recent_records)
    }
    
    /// Get metrics for all time
    pub fn get_all_time_metrics(&self) -> TransferMetrics {
        let records = self.records.lock().unwrap();
        let all_records: Vec<_> = records.iter().cloned().collect();
        self.calculate_metrics(&all_records)
    }
    
    /// Calculate metrics from records
    fn calculate_metrics(&self, records: &[TransferRecord]) -> TransferMetrics {
        if records.is_empty() {
            return TransferMetrics {
                total_bytes: 0,
                total_duration_us: 0,
                transfer_count: 0,
                avg_bandwidth_mbps: 0.0,
                peak_bandwidth_mbps: 0.0,
                min_bandwidth_mbps: 0.0,
                by_type: std::collections::HashMap::new(),
            };
        }
        
        let mut total_bytes = 0u64;
        let mut total_duration_us = 0u64;
        let mut peak_bandwidth_mbps = 0.0f64;
        let mut min_bandwidth_mbps = f64::MAX;
        let mut by_type = std::collections::HashMap::new();
        
        for record in records {
            total_bytes += record.bytes;
            total_duration_us += record.duration_us;
            
            // Calculate bandwidth for this transfer (MB/s)
            if record.duration_us > 0 {
                let bandwidth_mbps = (record.bytes as f64 / 1_000_000.0) / 
                                   (record.duration_us as f64 / 1_000_000.0);
                peak_bandwidth_mbps = peak_bandwidth_mbps.max(bandwidth_mbps);
                min_bandwidth_mbps = min_bandwidth_mbps.min(bandwidth_mbps);
            }
            
            // Track by type
            let type_entry = by_type.entry(record.transfer_type).or_insert(TypeMetrics {
                bytes: 0,
                count: 0,
                avg_bandwidth_mbps: 0.0,
            });
            type_entry.bytes += record.bytes;
            type_entry.count += 1;
        }
        
        // Calculate average bandwidth
        let avg_bandwidth_mbps = if total_duration_us > 0 {
            (total_bytes as f64 / 1_000_000.0) / (total_duration_us as f64 / 1_000_000.0)
        } else {
            0.0
        };
        
        // Calculate per-type averages
        for (transfer_type, metrics) in by_type.iter_mut() {
            let type_records: Vec<_> = records.iter()
                .filter(|r| r.transfer_type == *transfer_type)
                .collect();
            
            let type_duration_us: u64 = type_records.iter()
                .map(|r| r.duration_us)
                .sum();
            
            metrics.avg_bandwidth_mbps = if type_duration_us > 0 {
                (metrics.bytes as f64 / 1_000_000.0) / (type_duration_us as f64 / 1_000_000.0)
            } else {
                0.0
            };
        }
        
        TransferMetrics {
            total_bytes,
            total_duration_us,
            transfer_count: records.len(),
            avg_bandwidth_mbps,
            peak_bandwidth_mbps,
            min_bandwidth_mbps: if min_bandwidth_mbps == f64::MAX { 0.0 } else { min_bandwidth_mbps },
            by_type,
        }
    }
    
    /// Clear all recorded data
    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
    }
    
    /// Get bandwidth histogram (for visualization)
    pub fn get_bandwidth_histogram(&self, bucket_size_mbps: f64) -> Vec<(f64, usize)> {
        let records = self.records.lock().unwrap();
        let mut histogram = std::collections::HashMap::new();
        
        for record in records.iter() {
            if record.duration_us > 0 {
                let bandwidth_mbps = (record.bytes as f64 / 1_000_000.0) / 
                                   (record.duration_us as f64 / 1_000_000.0);
                let bucket = (bandwidth_mbps / bucket_size_mbps).floor() * bucket_size_mbps;
                *histogram.entry(bucket as i64).or_insert(0) += 1;
            }
        }
        
        let mut result: Vec<_> = histogram.into_iter()
            .map(|(bucket, count)| (bucket as f64, count))
            .collect();
        result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        result
    }
}

/// Scoped bandwidth measurement
pub struct BandwidthMeasurement<'a> {
    profiler: &'a BandwidthProfiler,
    bytes: u64,
    transfer_type: TransferType,
    start: Instant,
}

impl<'a> BandwidthMeasurement<'a> {
    pub fn new(profiler: &'a BandwidthProfiler, bytes: u64, transfer_type: TransferType) -> Self {
        Self {
            profiler,
            bytes,
            transfer_type,
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for BandwidthMeasurement<'a> {
    fn drop(&mut self) {
        let duration_us = self.start.elapsed().as_micros() as u64;
        self.profiler.record_typed_transfer(self.bytes, duration_us, self.transfer_type);
    }
}

/// Helper macros for bandwidth profiling
#[macro_export]
macro_rules! profile_transfer {
    ($profiler:expr, $bytes:expr, $transfer_type:expr, $code:block) => {
        {
            let _measurement = $crate::memory::bandwidth_profiler::BandwidthMeasurement::new(
                $profiler, $bytes, $transfer_type
            );
            $code
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bandwidth_calculation() {
        let profiler = BandwidthProfiler::new();
        
        // Record 100MB transfer in 100ms = 1000 MB/s
        profiler.record_transfer(100_000_000, 100_000);
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.transfer_count, 1);
        assert_eq!(metrics.total_bytes, 100_000_000);
        assert!((metrics.avg_bandwidth_mbps - 1000.0).abs() < 0.1);
    }
    
    #[test]
    fn test_transfer_types() {
        let profiler = BandwidthProfiler::new();
        
        profiler.record_typed_transfer(50_000_000, 50_000, TransferType::Upload);
        profiler.record_typed_transfer(30_000_000, 30_000, TransferType::Download);
        
        let metrics = profiler.get_metrics();
        assert_eq!(metrics.by_type.len(), 2);
        assert_eq!(metrics.by_type[&TransferType::Upload].count, 1);
        assert_eq!(metrics.by_type[&TransferType::Download].count, 1);
    }
}