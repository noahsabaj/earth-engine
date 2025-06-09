pub mod cache_profiler;
pub mod memory_profiler;
pub mod performance_metrics;

pub use cache_profiler::CacheProfiler;
pub use memory_profiler::MemoryProfiler;
pub use performance_metrics::PerformanceMetrics;

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