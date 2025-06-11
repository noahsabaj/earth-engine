use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::error::{EngineError, EngineResult};

/// Memory access pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessPattern {
    /// Sequential access (cache-friendly)
    Sequential,
    /// Strided access (potentially cache-friendly)
    Strided(usize),
    /// Random access (cache-unfriendly)
    Random,
}

/// Memory profiler for tracking access patterns and hot paths
#[derive(Clone)]
pub struct MemoryProfiler {
    data: Arc<Mutex<ProfileData>>,
}

struct ProfileData {
    /// Function call counts
    function_calls: HashMap<&'static str, u64>,
    /// Function execution times
    function_times: HashMap<&'static str, Duration>,
    /// Memory access patterns per function
    access_patterns: HashMap<&'static str, Vec<AccessPattern>>,
    /// Hot path detection
    hot_paths: Vec<HotPath>,
}

#[derive(Debug, Clone)]
pub struct HotPath {
    pub function: &'static str,
    pub call_count: u64,
    pub total_time: Duration,
    pub avg_time: Duration,
    pub access_pattern: AccessPattern,
}

impl MemoryProfiler {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(ProfileData {
                function_calls: HashMap::new(),
                function_times: HashMap::new(),
                access_patterns: HashMap::new(),
                hot_paths: Vec::new(),
            })),
        }
    }

    /// Record a function call
    pub fn record_function_call(&self, function: &'static str, duration: Duration) -> EngineResult<()> {
        let mut data = self.data.lock()
            .map_err(|_| EngineError::LockPoisoned { resource: "profiler_data".to_string() })?;
        
        *data.function_calls.entry(function).or_insert(0) += 1;
        *data.function_times.entry(function).or_insert(Duration::ZERO) += duration;
        Ok(())
    }

    /// Record memory access pattern
    pub fn record_access_pattern(&self, function: &'static str, pattern: AccessPattern) -> EngineResult<()> {
        let mut data = self.data.lock()
            .map_err(|_| EngineError::LockPoisoned { resource: "profiler_data".to_string() })?;
        data.access_patterns.entry(function).or_insert_with(Vec::new).push(pattern);
        Ok(())
    }

    /// Analyze memory access pattern from addresses
    pub fn analyze_access_pattern(&self, addresses: &[usize]) -> AccessPattern {
        if addresses.len() < 2 {
            return AccessPattern::Sequential;
        }

        let mut strides = Vec::new();
        for i in 1..addresses.len() {
            let stride = (addresses[i] as i64 - addresses[i-1] as i64).abs() as usize;
            strides.push(stride);
        }

        // Check if all strides are the same
        let first_stride = strides[0];
        if strides.iter().all(|&s| s == first_stride) {
            if first_stride <= 64 { // Cache line size
                AccessPattern::Sequential
            } else {
                AccessPattern::Strided(first_stride)
            }
        } else {
            AccessPattern::Random
        }
    }

    /// Identify hot paths (functions called frequently or taking significant time)
    pub fn identify_hot_paths(&self, min_calls: u64, min_time_ms: u64) -> EngineResult<()> {
        let mut data = self.data.lock()
            .map_err(|_| EngineError::LockPoisoned { resource: "profiler_data".to_string() })?;
        data.hot_paths.clear();

        // Collect hot paths to avoid borrow checker issues
        let mut hot_paths = Vec::new();
        
        for (&function, &call_count) in &data.function_calls {
            let total_time = data.function_times.get(&function).cloned().unwrap_or(Duration::ZERO);
            let avg_time = total_time / call_count.max(1) as u32;
            
            if call_count >= min_calls || total_time.as_millis() >= min_time_ms as u128 {
                // Determine predominant access pattern
                let patterns = data.access_patterns.get(&function);
                let access_pattern = if let Some(patterns) = patterns {
                    // Find most common pattern
                    let mut pattern_counts = HashMap::new();
                    for pattern in patterns {
                        *pattern_counts.entry(pattern).or_insert(0) += 1;
                    }
                    pattern_counts.into_iter()
                        .max_by_key(|&(_, count)| count)
                        .map(|(pattern, _)| *pattern)
                        .unwrap_or(AccessPattern::Random)
                } else {
                    AccessPattern::Random
                };

                hot_paths.push(HotPath {
                    function,
                    call_count,
                    total_time,
                    avg_time,
                    access_pattern,
                });
            }
        }

        // Sort by total time descending
        hot_paths.sort_by(|a, b| b.total_time.cmp(&a.total_time));
        
        // Now update the stored hot paths
        data.hot_paths = hot_paths;
        Ok(())
    }

    /// Get identified hot paths
    pub fn hot_paths(&self) -> EngineResult<Vec<HotPath>> {
        Ok(self.data.lock()
            .map_err(|_| EngineError::LockPoisoned { resource: "profiler_data".to_string() })?
            .hot_paths.clone())
    }

    /// Print profiling report
    pub fn report(&self) -> EngineResult<()> {
        let data = self.data.lock()
            .map_err(|_| EngineError::LockPoisoned { resource: "profiler_data".to_string() })?;
        
        println!("\n=== Memory Profiling Report ===");
        println!("\nFunction Call Statistics:");
        
        let mut functions: Vec<_> = data.function_calls.iter().collect();
        functions.sort_by(|a, b| b.1.cmp(a.1));
        
        for (&function, &count) in functions.iter().take(10) {
            let total_time = data.function_times.get(&function).cloned().unwrap_or(Duration::ZERO);
            println!("  {}: {} calls, {:.2}ms total", function, count, total_time.as_secs_f64() * 1000.0);
        }
        
        println!("\nHot Paths:");
        for hot_path in &data.hot_paths {
            println!("  {} - {} calls, {:.2}ms avg, pattern: {:?}", 
                hot_path.function, 
                hot_path.call_count,
                hot_path.avg_time.as_secs_f64() * 1000.0,
                hot_path.access_pattern
            );
        }
        
        println!("================================\n");
        Ok(())
    }
}

/// Profiling scope guard
pub struct ProfileScope<'a> {
    profiler: &'a MemoryProfiler,
    function: &'static str,
    start: Instant,
}

impl<'a> ProfileScope<'a> {
    pub fn new(profiler: &'a MemoryProfiler, function: &'static str) -> Self {
        Self {
            profiler,
            function,
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for ProfileScope<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        let _ = self.profiler.record_function_call(self.function, duration);
    }
}