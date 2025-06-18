pub mod cache_profiler;
pub mod memory_profiler;
pub mod performance_metrics;
pub mod final_profiler;
pub mod dop_benchmarks;
pub mod allocation_profiler;
pub mod reality_check_profiler;
pub mod gpu_workload_profiler;

pub use cache_profiler::CacheProfiler;
pub use memory_profiler::{MemoryProfiler, AccessPattern};
pub use performance_metrics::PerformanceMetrics;
pub use final_profiler::{
    FinalProfiler, PerformanceMetrics as FinalMetrics, PROFILER,
    begin_frame, time_operation, record_allocation, calculate_metrics, generate_report
};
pub use dop_benchmarks::{
    DOPBenchmarks, BenchmarkResult,
    dop_benchmarks_run_all_benchmarks,
    dop_benchmarks_benchmark_particle_processing,
    dop_benchmarks_benchmark_particle_memory_access,
    dop_benchmarks_benchmark_particle_cache_patterns,
    dop_benchmarks_benchmark_vector_operations,
    dop_benchmarks_benchmark_simd_operations,
    dop_benchmarks_benchmark_aos_vs_soa,
    dop_benchmarks_benchmark_memory_bandwidth,
    dop_benchmarks_benchmark_cache_line_utilization,
    dop_benchmarks_benchmark_prefetch_patterns,
};
pub use allocation_profiler::{AllocationProfiler, AllocationReport, AllocationBenchmark};
pub use reality_check_profiler::{
    RealityCheckProfiler, FrameMetrics, BlockingOperation, BlockingType, SystemMetrics,
    TrackingAllocator, GpuTimestamps,
    begin_frame as reality_begin_frame,
    end_frame as reality_end_frame,
    time_cpu_operation, record_draw_call, record_compute_dispatch,
    write_gpu_timestamp, generate_reality_report,
};
pub use gpu_workload_profiler::{
    GpuWorkloadProfiler, WorkloadAnalysis, SystemWorkload, FrameBreakdown,
    GpuOperationScope,
};

// Analysis module removed during cleanup

/// Macro for timing code blocks
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        let _profiler = $crate::profiling::ScopeProfiler::new($name);
    };
}

/// Automatic scope profiler
pub struct ScopeProfiler {
    name: &'static str,
    start: std::time::Instant,
}

impl ScopeProfiler {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: std::time::Instant::now(),
        }
    }
}

impl Drop for ScopeProfiler {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        eprintln!("[PROFILE] {}: {:?}", self.name, duration);
    }
}