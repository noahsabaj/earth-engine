/// GPU vs CPU benchmark module
pub mod gpu_vs_cpu_compute;

pub use gpu_vs_cpu_compute::{
    GpuVsCpuBenchmark, 
    BenchmarkResult,
    analyze_results,
};