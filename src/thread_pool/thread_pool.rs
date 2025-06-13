/// Global Thread Pool Manager
/// 
/// Centralizes thread pool management to prevent thread exhaustion
/// and improve resource utilization across the engine.

use std::sync::{Arc, OnceLock};
use rayon::{ThreadPool, ThreadPoolBuilder};
use tokio::runtime::Runtime;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Thread pool categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolCategory {
    /// World generation and chunk processing
    WorldGeneration,
    /// Mesh building and optimization
    MeshBuilding,
    /// Light propagation
    Lighting,
    /// Physics calculations
    Physics,
    /// Network I/O
    Network,
    /// File I/O and streaming
    FileIO,
    /// General compute tasks
    Compute,
}

/// Configuration for thread pools
#[derive(Debug, Clone)]
pub struct ThreadPoolConfig {
    /// Total number of threads across all pools
    pub total_threads: usize,
    /// Per-category thread limits (optional)
    pub category_limits: HashMap<PoolCategory, usize>,
    /// Enable thread naming
    pub enable_thread_names: bool,
    /// Stack size for worker threads (in bytes)
    pub stack_size: Option<usize>,
    /// Maximum number of thread pools allowed
    pub max_pool_count: usize,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        let total_threads = cpu_count.saturating_sub(2).max(4); // Leave 2 cores for OS/main thread
        
        let mut category_limits = HashMap::new();
        
        // Default distribution based on workload priorities
        category_limits.insert(PoolCategory::WorldGeneration, total_threads / 3);
        category_limits.insert(PoolCategory::MeshBuilding, total_threads / 3);
        category_limits.insert(PoolCategory::Lighting, total_threads / 6);
        category_limits.insert(PoolCategory::Physics, total_threads / 6);
        category_limits.insert(PoolCategory::Network, 2.min(total_threads / 4));
        category_limits.insert(PoolCategory::FileIO, 2.min(total_threads / 4));
        category_limits.insert(PoolCategory::Compute, total_threads / 4);
        
        Self {
            total_threads,
            category_limits,
            enable_thread_names: true,
            stack_size: Some(2 * 1024 * 1024), // 2MB stack per thread
            max_pool_count: 10, // Reasonable limit to prevent resource exhaustion
        }
    }
}

/// Global thread pool manager
pub struct ThreadPoolManager {
    /// Thread pools by category
    pools: RwLock<HashMap<PoolCategory, Arc<ThreadPool>>>,
    /// Shared general-purpose pool
    shared_pool: Arc<ThreadPool>,
    /// Tokio runtime for async tasks
    async_runtime: Arc<Runtime>,
    /// Configuration
    config: ThreadPoolConfig,
    /// Usage statistics
    stats: Arc<RwLock<ThreadPoolStats>>,
}

/// Thread pool usage statistics
#[derive(Debug, Default, Clone)]
pub struct ThreadPoolStats {
    pub tasks_submitted: HashMap<PoolCategory, u64>,
    pub tasks_completed: HashMap<PoolCategory, u64>,
    pub average_task_time_ms: HashMap<PoolCategory, f64>,
}

/// Global thread pool manager instance
static THREAD_POOL_MANAGER: OnceLock<Arc<ThreadPoolManager>> = OnceLock::new();

impl ThreadPoolManager {
    /// Initialize the global thread pool manager
    pub fn initialize(config: ThreadPoolConfig) -> Result<(), String> {
        if THREAD_POOL_MANAGER.get().is_some() {
            return Err("Thread pool manager already initialized".to_string());
        }
        
        let manager = Arc::new(Self::new(config)?);
        THREAD_POOL_MANAGER.set(manager)
            .map_err(|_| "Failed to set thread pool manager".to_string())?;
        
        Ok(())
    }
    
    /// Get the global thread pool manager
    pub fn global() -> Arc<ThreadPoolManager> {
        THREAD_POOL_MANAGER.get_or_init(|| {
            Arc::new(Self::new(ThreadPoolConfig::default())
                .expect("Failed to create default thread pool manager"))
        }).clone()
    }
    
    /// Create a new thread pool manager
    fn new(config: ThreadPoolConfig) -> Result<Self, String> {
        // Create shared general-purpose pool
        let mut shared_builder = ThreadPoolBuilder::new()
            .num_threads(config.total_threads / 2);
        
        if config.enable_thread_names {
            shared_builder = shared_builder.thread_name(|idx| format!("shared-worker-{}", idx));
        }
        
        if let Some(stack_size) = config.stack_size {
            shared_builder = shared_builder.stack_size(stack_size);
        }
        
        let shared_pool = Arc::new(
            shared_builder.build()
                .map_err(|e| format!("Failed to create shared pool: {}", e))?
        );
        
        // Create async runtime with limited threads
        let async_runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2.min(config.total_threads / 4))
                .thread_name("async-worker")
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to create async runtime: {}", e))?
        );
        
        Ok(Self {
            pools: RwLock::new(HashMap::new()),
            shared_pool,
            async_runtime,
            config,
            stats: Arc::new(RwLock::new(ThreadPoolStats::default())),
        })
    }
    
    /// Get or create a thread pool for a category
    pub fn get_pool(&self, category: PoolCategory) -> Arc<ThreadPool> {
        // Check if pool exists
        {
            let pools = self.pools.read();
            if let Some(pool) = pools.get(&category) {
                return pool.clone();
            }
        }
        
        // Create pool if it doesn't exist
        let mut pools = self.pools.write();
        
        // Double-check after acquiring write lock
        if let Some(pool) = pools.get(&category) {
            return pool.clone();
        }
        
        // Check pool count limit
        if pools.len() >= self.config.max_pool_count {
            log::warn!("Maximum pool count ({}) reached, using shared pool for {:?}", 
                self.config.max_pool_count, category);
            return self.shared_pool.clone();
        }
        
        // Create new pool
        let thread_count = self.config.category_limits
            .get(&category)
            .copied()
            .unwrap_or(2);
        
        let mut builder = ThreadPoolBuilder::new()
            .num_threads(thread_count);
        
        if self.config.enable_thread_names {
            let category_name = format!("{:?}", category).to_lowercase();
            builder = builder.thread_name(move |idx| format!("{}-{}", category_name, idx));
        }
        
        if let Some(stack_size) = self.config.stack_size {
            builder = builder.stack_size(stack_size);
        }
        
        let pool = match builder.build() {
            Ok(pool) => Arc::new(pool),
            Err(_) => {
                log::warn!("Failed to create pool for {:?}, using shared pool", category);
                return self.shared_pool.clone();
            }
        };
        
        pools.insert(category, pool.clone());
        pool
    }
    
    /// Get the shared general-purpose pool
    pub fn shared_pool(&self) -> Arc<ThreadPool> {
        self.shared_pool.clone()
    }
    
    /// Get the async runtime
    pub fn async_runtime(&self) -> Arc<Runtime> {
        self.async_runtime.clone()
    }
    
    /// Execute a task on a category-specific pool
    pub fn execute<F, R>(&self, category: PoolCategory, task: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        let pool = self.get_pool(category);
        let start = std::time::Instant::now();
        
        let result = pool.install(task);
        
        // Update statistics
        let elapsed = start.elapsed().as_millis() as f64;
        let mut stats = self.stats.write();
        
        *stats.tasks_submitted.entry(category).or_insert(0) += 1;
        *stats.tasks_completed.entry(category).or_insert(0) += 1;
        
        let count = *stats.tasks_completed.get(&category).unwrap_or(&0) as f64;
        let avg = stats.average_task_time_ms.entry(category).or_insert(0.0);
        if count > 0.0 {
            *avg = (*avg * (count - 1.0) + elapsed) / count;
        }
        
        result
    }
    
    /// Spawn a task on a category-specific pool
    pub fn spawn<F>(&self, category: PoolCategory, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let pool = self.get_pool(category);
        pool.spawn(task);
        
        // Update statistics
        let mut stats = self.stats.write();
        *stats.tasks_submitted.entry(category).or_insert(0) += 1;
    }
    
    /// Get usage statistics
    pub fn get_stats(&self) -> ThreadPoolStats {
        let stats = self.stats.read();
        ThreadPoolStats {
            tasks_submitted: stats.tasks_submitted.clone(),
            tasks_completed: stats.tasks_completed.clone(),
            average_task_time_ms: stats.average_task_time_ms.clone(),
        }
    }
    
    /// Resize a pool (creates a new pool with different thread count)
    pub fn resize_pool(&self, category: PoolCategory, new_thread_count: usize) -> Result<(), String> {
        if new_thread_count == 0 {
            return Err("Thread count must be greater than 0".to_string());
        }
        
        let mut builder = ThreadPoolBuilder::new()
            .num_threads(new_thread_count);
        
        if self.config.enable_thread_names {
            let category_name = format!("{:?}", category).to_lowercase();
            builder = builder.thread_name(move |idx| format!("{}-{}", category_name, idx));
        }
        
        if let Some(stack_size) = self.config.stack_size {
            builder = builder.stack_size(stack_size);
        }
        
        let new_pool = Arc::new(
            builder.build()
                .map_err(|e| format!("Failed to create resized pool: {}", e))?
        );
        
        let mut pools = self.pools.write();
        pools.insert(category, new_pool);
        
        Ok(())
    }
}

/// Convenience functions for common operations
impl ThreadPoolManager {
    /// Execute a world generation task
    pub fn execute_world_gen<F, R>(task: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        Self::global().execute(PoolCategory::WorldGeneration, task)
    }
    
    /// Execute a mesh building task
    pub fn execute_mesh_build<F, R>(task: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        Self::global().execute(PoolCategory::MeshBuilding, task)
    }
    
    /// Execute a lighting task
    pub fn execute_lighting<F, R>(task: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        Self::global().execute(PoolCategory::Lighting, task)
    }
    
    /// Spawn an async task
    pub fn spawn_async<F>(future: F)
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        Self::global().async_runtime().spawn(future);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_thread_pool_manager_initialization() {
        let config = ThreadPoolConfig::default();
        let manager = ThreadPoolManager::new(config)
            .expect("Failed to create ThreadPoolManager for test");
        
        // Test pool creation
        let world_pool = manager.get_pool(PoolCategory::WorldGeneration);
        let mesh_pool = manager.get_pool(PoolCategory::MeshBuilding);
        
        // Pools should be different
        assert!(!Arc::ptr_eq(&world_pool, &mesh_pool));
    }
    
    #[test]
    fn test_task_execution() {
        let config = ThreadPoolConfig::default();
        let manager = Arc::new(ThreadPoolManager::new(config).expect("Failed to create ThreadPoolManager for task execution test"));
        
        let result = manager.execute(PoolCategory::Compute, || {
            1 + 1
        });
        
        assert_eq!(result, 2);
        
        // Check stats
        let stats = manager.get_stats();
        assert_eq!(stats.tasks_submitted[&PoolCategory::Compute], 1);
        assert_eq!(stats.tasks_completed[&PoolCategory::Compute], 1);
    }
}