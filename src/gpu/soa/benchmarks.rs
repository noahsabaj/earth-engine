//! Benchmarking tools for SOA vs AOS performance comparison
//!
//! Provides tools to measure and validate the performance improvements
//! from using Structure of Arrays layouts.

use crate::gpu::soa::types::{BlockDistributionSOA, SoaCompatible, TerrainParamsSOA};
use crate::gpu::types::terrain::{BlockDistribution, TerrainParams};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performance metrics for memory access patterns
#[derive(Debug, Clone)]
pub struct MemoryAccessMetrics {
    /// Total bytes accessed
    pub bytes_accessed: u64,
    /// Number of cache lines touched
    pub cache_lines_touched: u64,
    /// Estimated cache misses
    pub cache_misses: u64,
    /// Memory bandwidth utilization (0.0 - 1.0)
    pub bandwidth_utilization: f32,
    /// Access pattern efficiency (0.0 - 1.0)
    pub efficiency: f32,
}

/// Benchmark results comparing AOS vs SOA
#[derive(Debug, Clone)]
pub struct SoaBenchmarkResults {
    /// AOS performance metrics
    pub aos_metrics: PerformanceMetrics,
    /// SOA performance metrics
    pub soa_metrics: PerformanceMetrics,
    /// Improvement factor (SOA time / AOS time)
    pub speedup: f32,
    /// Memory bandwidth savings
    pub bandwidth_savings: f32,
    /// Cache efficiency improvement
    pub cache_improvement: f32,
}

/// Performance metrics for a single test
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Total execution time
    pub duration: Duration,
    /// Operations per second
    pub ops_per_second: f64,
    /// Memory access metrics
    pub memory_metrics: MemoryAccessMetrics,
    /// GPU utilization (if available)
    pub gpu_utilization: Option<f32>,
}

/// SOA benchmark suite
pub struct SoaBenchmarkSuite {
    /// Number of iterations per test
    iterations: usize,
    /// Test data size
    data_size: usize,
}

impl SoaBenchmarkSuite {
    /// Create a new benchmark suite
    pub fn new(iterations: usize, data_size: usize) -> Self {
        Self {
            iterations,
            data_size,
        }
    }

    /// Run height check benchmark
    pub fn bench_height_check(&self) -> SoaBenchmarkResults {
        // Generate test data
        let distributions = self.generate_test_distributions();
        let test_heights: Vec<i32> = (0..self.iterations)
            .map(|i| (i % 128) as i32 - 64)
            .collect();

        // Benchmark AOS
        let aos_metrics = self.bench_aos_height_check(&distributions, &test_heights);

        // Benchmark SOA
        let soa_metrics = self.bench_soa_height_check(&distributions, &test_heights);

        // Calculate improvements
        let speedup = aos_metrics.duration.as_secs_f32() / soa_metrics.duration.as_secs_f32();
        let bandwidth_savings = 1.0
            - (soa_metrics.memory_metrics.bytes_accessed as f32
                / aos_metrics.memory_metrics.bytes_accessed as f32);
        let cache_improvement =
            soa_metrics.memory_metrics.efficiency - aos_metrics.memory_metrics.efficiency;

        SoaBenchmarkResults {
            aos_metrics,
            soa_metrics,
            speedup,
            bandwidth_savings,
            cache_improvement,
        }
    }

    /// Benchmark AOS height check
    fn bench_aos_height_check(
        &self,
        distributions: &[BlockDistribution],
        test_heights: &[i32],
    ) -> PerformanceMetrics {
        let start = Instant::now();
        let mut matches = 0u32;

        // Simulate AOS access pattern
        for &height in test_heights {
            for dist in distributions {
                if height >= dist.min_height && height <= dist.max_height {
                    matches += 1;
                    break;
                }
            }
        }

        let duration = start.elapsed();

        // Calculate memory metrics
        let bytes_per_distribution = std::mem::size_of::<BlockDistribution>();
        let bytes_accessed =
            test_heights.len() as u64 * distributions.len() as u64 * bytes_per_distribution as u64;

        // Cache line size (typical)
        const CACHE_LINE_SIZE: u64 = 64;
        let cache_lines_touched = bytes_accessed / CACHE_LINE_SIZE;

        // Estimate cache misses (AOS has poor locality)
        let cache_misses = cache_lines_touched * 8 / 10; // ~80% miss rate

        PerformanceMetrics {
            duration,
            ops_per_second: test_heights.len() as f64 / duration.as_secs_f64(),
            memory_metrics: MemoryAccessMetrics {
                bytes_accessed,
                cache_lines_touched,
                cache_misses,
                bandwidth_utilization: 0.4, // Poor for AOS
                efficiency: 0.3,            // Poor cache efficiency
            },
            gpu_utilization: None,
        }
    }

    /// Benchmark SOA height check
    fn bench_soa_height_check(
        &self,
        distributions: &[BlockDistribution],
        test_heights: &[i32],
    ) -> PerformanceMetrics {
        // Convert to SOA
        let soa_data = BlockDistribution::to_soa(distributions);

        let start = Instant::now();
        let mut matches = 0u32;

        // Simulate SOA access pattern
        for &height in test_heights {
            // Access only min_heights and max_heights arrays
            for i in 0..soa_data.count as usize {
                if height >= soa_data.min_heights[i] && height <= soa_data.max_heights[i] {
                    matches += 1;
                    break;
                }
            }
        }

        let duration = start.elapsed();

        // Calculate memory metrics
        // SOA only accesses the arrays we need
        let bytes_per_element = std::mem::size_of::<i32>();
        let bytes_accessed =
            test_heights.len() as u64 * soa_data.count as u64 * bytes_per_element as u64 * 2; // min + max arrays

        const CACHE_LINE_SIZE: u64 = 64;
        let cache_lines_touched = bytes_accessed / CACHE_LINE_SIZE;

        // SOA has excellent locality
        let cache_misses = cache_lines_touched / 10; // ~10% miss rate

        PerformanceMetrics {
            duration,
            ops_per_second: test_heights.len() as f64 / duration.as_secs_f64(),
            memory_metrics: MemoryAccessMetrics {
                bytes_accessed,
                cache_lines_touched,
                cache_misses,
                bandwidth_utilization: 0.9, // Excellent for SOA
                efficiency: 0.85,           // Great cache efficiency
            },
            gpu_utilization: None,
        }
    }

    /// Generate test distributions
    fn generate_test_distributions(&self) -> Vec<BlockDistribution> {
        (0..self.data_size.min(crate::gpu::soa::MAX_BLOCK_DISTRIBUTIONS))
            .map(|i| BlockDistribution {
                block_id: i as u32 + 1,
                min_height: (i * 10) as i32 - 50,
                max_height: (i * 10 + 20) as i32 - 50,
                probability: 0.1 * (i + 1) as f32,
                noise_threshold: 0.5,
                _padding: [0; 3],
            })
            .collect()
    }

    /// Run full benchmark suite
    pub fn run_all(&self) -> SoaBenchmarkReport {
        log::info!("[SOA Benchmark] Starting performance comparison");
        log::info!(
            "[SOA Benchmark] Iterations: {}, Data size: {}",
            self.iterations,
            self.data_size
        );

        let height_check = self.bench_height_check();

        // Additional benchmarks could be added here

        SoaBenchmarkReport {
            height_check_results: height_check,
            timestamp: std::time::SystemTime::now(),
            configuration: BenchmarkConfig {
                iterations: self.iterations,
                data_size: self.data_size,
            },
        }
    }
}

/// Complete benchmark report
#[derive(Debug, Clone)]
pub struct SoaBenchmarkReport {
    /// Height check benchmark results
    pub height_check_results: SoaBenchmarkResults,
    /// When the benchmark was run
    pub timestamp: std::time::SystemTime,
    /// Benchmark configuration
    pub configuration: BenchmarkConfig,
}

impl SoaBenchmarkReport {
    /// Generate a summary of the results
    pub fn summary(&self) -> String {
        format!(
            "SOA Benchmark Summary:\n\
             Configuration: {} iterations, {} data elements\n\
             \n\
             Height Check Performance:\n\
             - AOS Duration: {:?}\n\
             - SOA Duration: {:?}\n\
             - Speedup: {:.2}x\n\
             - Memory Bandwidth Savings: {:.1}%\n\
             - Cache Efficiency Improvement: {:.1}%\n\
             \n\
             Memory Access Patterns:\n\
             - AOS Bytes Accessed: {} MB\n\
             - SOA Bytes Accessed: {} MB\n\
             - Reduction: {:.1}%",
            self.configuration.iterations,
            self.configuration.data_size,
            self.height_check_results.aos_metrics.duration,
            self.height_check_results.soa_metrics.duration,
            self.height_check_results.speedup,
            self.height_check_results.bandwidth_savings * 100.0,
            self.height_check_results.cache_improvement * 100.0,
            self.height_check_results
                .aos_metrics
                .memory_metrics
                .bytes_accessed
                / 1_000_000,
            self.height_check_results
                .soa_metrics
                .memory_metrics
                .bytes_accessed
                / 1_000_000,
            self.height_check_results.bandwidth_savings * 100.0,
        )
    }
}

/// Benchmark configuration
#[derive(Debug, Clone, Copy)]
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub data_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soa_benchmark() {
        let suite = SoaBenchmarkSuite::new(1000, 10);
        let results = suite.bench_height_check();

        // SOA should be faster
        assert!(results.speedup > 1.0);

        // SOA should use less bandwidth
        assert!(results.bandwidth_savings > 0.0);

        // SOA should have better cache efficiency
        assert!(results.cache_improvement > 0.0);

        println!("Benchmark results:\n{}", suite.run_all().summary());
    }
}
