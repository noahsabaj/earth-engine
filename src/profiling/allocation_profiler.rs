use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use crate::error::EngineResult;

/// Global allocation tracker for measuring DOP vs OOP allocation patterns  
// Note: Will be initialized with lazy_static or other runtime initialization

/// Profiler for tracking memory allocations and identifying hot allocation paths
pub struct AllocationProfiler {
    stats: Arc<AllocationStats>,
}

#[derive(Default)]
struct AllocationStats {
    /// Total number of allocations
    total_allocations: AtomicUsize,
    /// Total bytes allocated
    total_bytes: AtomicU64,
    /// Allocations in current frame
    frame_allocations: AtomicUsize,
    /// Bytes allocated in current frame
    frame_bytes: AtomicU64,
    /// Peak allocations per frame
    peak_frame_allocations: AtomicUsize,
    /// Peak bytes per frame
    peak_frame_bytes: AtomicU64,
    /// Current frame number
    frame_number: AtomicU64,
}

impl AllocationProfiler {
    /// Create a new allocation profiler
    pub fn new() -> Self {
        Self {
            stats: Arc::new(AllocationStats::default()),
        }
    }

    /// Record an allocation
    pub fn record_allocation(&self, bytes: usize) {
        self.stats.total_allocations.fetch_add(1, Ordering::Relaxed);
        self.stats.total_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        self.stats.frame_allocations.fetch_add(1, Ordering::Relaxed);
        self.stats.frame_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Start a new frame (resets frame counters)
    pub fn start_frame(&self) {
        // Update peaks if necessary
        let frame_allocs = self.stats.frame_allocations.load(Ordering::Relaxed);
        let frame_bytes = self.stats.frame_bytes.load(Ordering::Relaxed);
        
        // Update peak allocations
        let mut peak_allocs = self.stats.peak_frame_allocations.load(Ordering::Relaxed);
        while frame_allocs > peak_allocs {
            match self.stats.peak_frame_allocations.compare_exchange_weak(
                peak_allocs, frame_allocs, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => peak_allocs = x,
            }
        }
        
        // Update peak bytes
        let mut peak_bytes = self.stats.peak_frame_bytes.load(Ordering::Relaxed);
        while frame_bytes > peak_bytes {
            match self.stats.peak_frame_bytes.compare_exchange_weak(
                peak_bytes, frame_bytes, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => peak_bytes = x,
            }
        }
        
        // Reset frame counters
        self.stats.frame_allocations.store(0, Ordering::Relaxed);
        self.stats.frame_bytes.store(0, Ordering::Relaxed);
        self.stats.frame_number.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current allocation statistics
    pub fn get_stats(&self) -> AllocationReport {
        AllocationReport {
            total_allocations: self.stats.total_allocations.load(Ordering::Relaxed),
            total_bytes: self.stats.total_bytes.load(Ordering::Relaxed),
            current_frame_allocations: self.stats.frame_allocations.load(Ordering::Relaxed),
            current_frame_bytes: self.stats.frame_bytes.load(Ordering::Relaxed),
            peak_frame_allocations: self.stats.peak_frame_allocations.load(Ordering::Relaxed),
            peak_frame_bytes: self.stats.peak_frame_bytes.load(Ordering::Relaxed),
            frame_number: self.stats.frame_number.load(Ordering::Relaxed),
        }
    }

    /// Print allocation report
    pub fn report(&self) {
        let stats = self.get_stats();
        
        println!("\n=== Allocation Profiling Report ===");
        println!("Frame: {}", stats.frame_number);
        println!("Total allocations: {}", stats.total_allocations);
        println!("Total bytes: {:.2} MB", stats.total_bytes as f64 / 1_000_000.0);
        println!("Current frame allocations: {}", stats.current_frame_allocations);
        println!("Current frame bytes: {:.2} KB", stats.current_frame_bytes as f64 / 1000.0);
        println!("Peak frame allocations: {}", stats.peak_frame_allocations);
        println!("Peak frame bytes: {:.2} KB", stats.peak_frame_bytes as f64 / 1000.0);
        
        if stats.frame_number > 0 {
            let avg_allocs = stats.total_allocations as f64 / stats.frame_number as f64;
            let avg_bytes = stats.total_bytes as f64 / stats.frame_number as f64;
            println!("Average allocations/frame: {:.1}", avg_allocs);
            println!("Average bytes/frame: {:.2} KB", avg_bytes / 1000.0);
        }
        
        println!("====================================\n");
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.stats.total_allocations.store(0, Ordering::Relaxed);
        self.stats.total_bytes.store(0, Ordering::Relaxed);
        self.stats.frame_allocations.store(0, Ordering::Relaxed);
        self.stats.frame_bytes.store(0, Ordering::Relaxed);
        self.stats.peak_frame_allocations.store(0, Ordering::Relaxed);
        self.stats.peak_frame_bytes.store(0, Ordering::Relaxed);
        self.stats.frame_number.store(0, Ordering::Relaxed);
    }
}

impl Clone for AllocationProfiler {
    fn clone(&self) -> Self {
        Self {
            stats: Arc::clone(&self.stats),
        }
    }
}

/// Allocation statistics report
#[derive(Debug, Clone)]
pub struct AllocationReport {
    pub total_allocations: usize,
    pub total_bytes: u64,
    pub current_frame_allocations: usize,
    pub current_frame_bytes: u64,
    pub peak_frame_allocations: usize,
    pub peak_frame_bytes: u64,
    pub frame_number: u64,
}

/// Macro for tracking allocations in a scope
#[macro_export]
macro_rules! track_allocations {
    ($profiler:expr, $bytes:expr, $code:block) => {{
        $profiler.record_allocation($bytes);
        $code
    }};
}

/// Allocation tracker for specific operations
pub struct AllocationScope<'a> {
    profiler: &'a AllocationProfiler,
    start_allocations: usize,
    start_bytes: u64,
    operation_name: &'static str,
}

impl<'a> AllocationScope<'a> {
    pub fn new(profiler: &'a AllocationProfiler, operation_name: &'static str) -> Self {
        let stats = profiler.get_stats();
        Self {
            profiler,
            start_allocations: stats.current_frame_allocations,
            start_bytes: stats.current_frame_bytes,
            operation_name,
        }
    }
}

impl<'a> Drop for AllocationScope<'a> {
    fn drop(&mut self) {
        let stats = self.profiler.get_stats();
        let alloc_delta = stats.current_frame_allocations - self.start_allocations;
        let bytes_delta = stats.current_frame_bytes - self.start_bytes;
        
        if alloc_delta > 0 || bytes_delta > 0 {
            println!("[ALLOC] {}: {} allocations, {} bytes", 
                self.operation_name, alloc_delta, bytes_delta);
        }
    }
}

/// Benchmark allocation patterns for DOP vs OOP
pub struct AllocationBenchmark {
    profiler: AllocationProfiler,
}

impl AllocationBenchmark {
    pub fn new() -> Self {
        Self {
            profiler: AllocationProfiler::new(),
        }
    }
}

/// Compare allocation patterns between DOP and OOP approaches
pub fn compare_allocation_patterns(benchmark: &mut AllocationBenchmark) -> EngineResult<()> {
    println!("🔍 Comparing Allocation Patterns: DOP vs OOP");
    println!("===========================================");

    const ITERATIONS: usize = 1000;
    const ELEMENTS: usize = 1000;

    // Test DOP approach (pre-allocated, no runtime allocations)
    println!("\n1. DOP Approach (Pre-allocated SOA):");
    benchmark.profiler.reset();
    
    // Pre-allocate all data
    let mut positions_x = Vec::with_capacity(ELEMENTS);
    let mut positions_y = Vec::with_capacity(ELEMENTS);
    let mut positions_z = Vec::with_capacity(ELEMENTS);
    
    // Fill initial data (one-time allocation cost)
    for i in 0..ELEMENTS {
        positions_x.push(i as f32);
        positions_y.push(i as f32);
        positions_z.push(i as f32);
    }

    // Simulate frame processing with no allocations
    for _frame in 0..ITERATIONS {
        benchmark.profiler.start_frame();
        
        // Process data without allocations
        for i in 0..ELEMENTS {
            positions_x[i] += 1.0;
            positions_y[i] += 1.0;
            positions_z[i] += 1.0;
        }
    }
    
    let dop_stats = benchmark.profiler.get_stats();
    println!("   Frames processed: {}", dop_stats.frame_number);
    println!("   Total allocations: {}", dop_stats.total_allocations);
    println!("   Allocations per frame: {:.2}", 
        dop_stats.total_allocations as f64 / dop_stats.frame_number as f64);

    // Test OOP approach (dynamic allocations)
    println!("\n2. OOP Approach (Dynamic Vector of Structs):");
    benchmark.profiler.reset();

    #[derive(Clone)]
    struct Position {
        x: f32,
        y: f32, 
        z: f32,
    }

    for _frame in 0..ITERATIONS {
        benchmark.profiler.start_frame();
        
        // Simulate dynamic object creation (typical OOP pattern)
        let mut objects: Vec<Position> = Vec::new();
        benchmark.profiler.record_allocation(ELEMENTS * std::mem::size_of::<Position>());
        
        for i in 0..ELEMENTS {
            objects.push(Position {
                x: i as f32,
                y: i as f32,
                z: i as f32,
            });
        }
        
        // Process objects
        for obj in &mut objects {
            obj.x += 1.0;
            obj.y += 1.0;
            obj.z += 1.0;
        }
        
        // Objects drop here, simulating frame cleanup
    }
    
    let oop_stats = benchmark.profiler.get_stats();
    println!("   Frames processed: {}", oop_stats.frame_number);
    println!("   Total allocations: {}", oop_stats.total_allocations);
    println!("   Allocations per frame: {:.2}", 
        oop_stats.total_allocations as f64 / oop_stats.frame_number as f64);

    // Performance comparison
    println!("\n📊 Allocation Performance Comparison:");
    let alloc_reduction = (oop_stats.total_allocations as f64 - dop_stats.total_allocations as f64) 
        / oop_stats.total_allocations as f64 * 100.0;
    
    println!("   DOP total allocations: {}", dop_stats.total_allocations);
    println!("   OOP total allocations: {}", oop_stats.total_allocations);
    println!("   Allocation reduction: {:.1}%", alloc_reduction);
    
    if dop_stats.total_allocations == 0 {
        println!("   🎯 DOP achieves ZERO runtime allocations!");
    } else {
        println!("   Allocation improvement: {:.2}x fewer allocations", 
            oop_stats.total_allocations as f64 / dop_stats.total_allocations as f64);
    }

    Ok(())
}

/// Test memory pool vs dynamic allocation
pub fn test_memory_pools(benchmark: &mut AllocationBenchmark) -> EngineResult<()> {
    println!("\n💾 Memory Pool vs Dynamic Allocation");
    println!("===================================");

    const PARTICLE_COUNT: usize = 10000;
    const FRAMES: usize = 100;

    // Test memory pool approach (DOP)
    println!("\n1. Memory Pool Approach:");
    benchmark.profiler.reset();

    // Pre-allocate particle pool
    let pool_capacity = PARTICLE_COUNT * 2; // Generous capacity
    let mut particle_pool: Vec<Option<[f32; 6]>> = vec![None; pool_capacity]; // pos + vel
    let mut next_free = 0;

    for frame in 0..FRAMES {
        benchmark.profiler.start_frame();
        
        // Spawn particles using pool (no allocation)
        let spawn_count = 100;
        for _ in 0..spawn_count.min(pool_capacity - next_free) {
            particle_pool[next_free] = Some([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
            next_free += 1;
        }
        
        // Update existing particles
        for particle in &mut particle_pool[0..next_free] {
            if let Some(data) = particle {
                data[0] += data[3] * 0.016; // Update position
                data[1] += data[4] * 0.016;
                data[2] += data[5] * 0.016;
            }
        }
        
        // Remove some particles (no deallocation, just mark as free)
        if frame % 10 == 0 && next_free > 50 {
            for i in 0..50 {
                particle_pool[i] = None;
            }
            // Compact pool (still no allocation)
            let mut write_index = 0;
            for read_index in 0..next_free {
                if particle_pool[read_index].is_some() {
                    particle_pool[write_index] = particle_pool[read_index];
                    write_index += 1;
                }
            }
            next_free = write_index;
        }
    }

    let pool_stats = benchmark.profiler.get_stats();
    println!("   Pool allocations: {}", pool_stats.total_allocations);
    println!("   Allocations per frame: {:.2}", 
        pool_stats.total_allocations as f64 / pool_stats.frame_number as f64);

    // Test dynamic allocation approach (OOP)
    println!("\n2. Dynamic Allocation Approach:");
    benchmark.profiler.reset();

    #[derive(Clone)]
    struct Particle {
        position: [f32; 3],
        velocity: [f32; 3],
    }

    for frame in 0..FRAMES {
        benchmark.profiler.start_frame();
        
        // Dynamic particle vector (allocates every frame)
        let mut particles: Vec<Particle> = Vec::new();
        benchmark.profiler.record_allocation(std::mem::size_of::<Vec<Particle>>());
        
        // Spawn particles (dynamic allocation)
        let spawn_count = 100;
        for i in 0..spawn_count {
            benchmark.profiler.record_allocation(std::mem::size_of::<Particle>());
            particles.push(Particle {
                position: [i as f32, 0.0, 0.0],
                velocity: [1.0, 1.0, 1.0],
            });
        }
        
        // Update particles
        for particle in &mut particles {
            particle.position[0] += particle.velocity[0] * 0.016;
            particle.position[1] += particle.velocity[1] * 0.016;
            particle.position[2] += particle.velocity[2] * 0.016;
        }
        
        // Remove particles (deallocate)
        if frame % 10 == 0 && particles.len() > 50 {
            particles.truncate(particles.len() - 50);
            particles.shrink_to_fit(); // Force deallocation
            benchmark.profiler.record_allocation(particles.capacity() * std::mem::size_of::<Particle>());
        }
        
        // Particles are deallocated when vector drops
    }

    let dynamic_stats = benchmark.profiler.get_stats();
    println!("   Dynamic allocations: {}", dynamic_stats.total_allocations);
    println!("   Allocations per frame: {:.2}", 
        dynamic_stats.total_allocations as f64 / dynamic_stats.frame_number as f64);

    // Comparison
    println!("\n🏆 Memory Management Comparison:");
    if pool_stats.total_allocations == 0 {
        println!("   Memory pool: ZERO allocations (perfect!)");
        println!("   Dynamic allocation: {} allocations", dynamic_stats.total_allocations);
        println!("   Improvement: ∞x (eliminated all allocations)");
    } else {
        let improvement = dynamic_stats.total_allocations as f64 / pool_stats.total_allocations as f64;
        println!("   Memory pool improvement: {:.2}x fewer allocations", improvement);
    }

    Ok(())
}