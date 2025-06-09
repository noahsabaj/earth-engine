use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Cache line size on most modern CPUs
pub const CACHE_LINE_SIZE: usize = 64;

/// Profiler for tracking cache-related metrics
#[derive(Clone)]
pub struct CacheProfiler {
    stats: Arc<CacheStats>,
}

#[derive(Default)]
struct CacheStats {
    /// Estimated cache misses based on memory access patterns
    cache_misses: AtomicU64,
    /// Sequential memory accesses (cache-friendly)
    sequential_accesses: AtomicU64,
    /// Random memory accesses (cache-unfriendly)
    random_accesses: AtomicU64,
    /// Total bytes accessed
    bytes_accessed: AtomicU64,
}

impl CacheProfiler {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(CacheStats::default()),
        }
    }

    /// Record a memory access pattern
    pub fn record_access(&self, address: usize, size: usize, previous_address: Option<usize>) {
        self.stats.bytes_accessed.fetch_add(size as u64, Ordering::Relaxed);

        if let Some(prev) = previous_address {
            let distance = (address as i64 - prev as i64).abs() as usize;
            
            if distance <= CACHE_LINE_SIZE {
                // Sequential access within cache line
                self.stats.sequential_accesses.fetch_add(1, Ordering::Relaxed);
            } else {
                // Random access - likely cache miss
                self.stats.random_accesses.fetch_add(1, Ordering::Relaxed);
                self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Analyze array access pattern
    pub fn analyze_array_access<T>(&self, array: &[T], indices: &[usize]) {
        let element_size = std::mem::size_of::<T>();
        let base_addr = array.as_ptr() as usize;
        
        let mut prev_addr = None;
        for &index in indices {
            let addr = base_addr + (index * element_size);
            self.record_access(addr, element_size, prev_addr);
            prev_addr = Some(addr);
        }
    }

    /// Get cache efficiency ratio (0.0 to 1.0)
    pub fn cache_efficiency(&self) -> f64 {
        let sequential = self.stats.sequential_accesses.load(Ordering::Relaxed) as f64;
        let random = self.stats.random_accesses.load(Ordering::Relaxed) as f64;
        let total = sequential + random;
        
        if total > 0.0 {
            sequential / total
        } else {
            1.0
        }
    }

    /// Print profiling report
    pub fn report(&self) {
        let sequential = self.stats.sequential_accesses.load(Ordering::Relaxed);
        let random = self.stats.random_accesses.load(Ordering::Relaxed);
        let misses = self.stats.cache_misses.load(Ordering::Relaxed);
        let bytes = self.stats.bytes_accessed.load(Ordering::Relaxed);
        
        println!("\n=== Cache Profiling Report ===");
        println!("Sequential accesses: {}", sequential);
        println!("Random accesses: {}", random);
        println!("Estimated cache misses: {}", misses);
        println!("Total bytes accessed: {}", bytes);
        println!("Cache efficiency: {:.2}%", self.cache_efficiency() * 100.0);
        println!("==============================\n");
    }
}