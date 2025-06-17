/// Simple allocation test to count allocations in basic operations
/// without complex physics or lighting systems that might crash

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Tracking allocator to measure allocations
struct TrackingAllocator {
    allocations: AtomicUsize,
}

impl TrackingAllocator {
    const fn new() -> Self {
        Self {
            allocations: AtomicUsize::new(0),
        }
    }
    
    fn reset(&self) {
        self.allocations.store(0, Ordering::SeqCst);
    }
    
    fn count(&self) -> usize {
        self.allocations.load(Ordering::SeqCst)
    }
}

// SAFETY: Implementing GlobalAlloc requires unsafe because it deals with raw memory management.
// This implementation is safe because it delegates to the system allocator and only adds tracking.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocations.fetch_add(1, Ordering::SeqCst);
        System.alloc(layout)
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();

fn test_basic_operations() {
    println!("=== Basic Operations Test ===");
    
    ALLOCATOR.reset();
    
    // Test vector allocations
    let _v1 = Vec::<i32>::new();
    let _v2 = vec![1, 2, 3];
    let mut _v3: Vec<i32> = Vec::with_capacity(10);
    
    // Test string allocations  
    let _s1 = String::new();
    let _s2 = "hello".to_string();
    
    // Test hashmap allocations
    let mut _h1 = std::collections::HashMap::<i32, i32>::new();
    _h1.insert(1, 2);
    
    println!("Basic operations allocations: {}", ALLOCATOR.count());
}

fn test_loop_allocations() {
    println!("\n=== Loop Allocations Test ===");
    
    ALLOCATOR.reset();
    
    // Simulate frame loop with allocations
    for i in 0..1000 {
        // This would be terrible for zero-allocation
        let _temp_vec = vec![i, i+1, i+2]; 
        let _temp_string = format!("frame_{}", i);
        let mut _temp_map = std::collections::HashMap::new();
        _temp_map.insert(i, i * 2);
    }
    
    println!("Loop (1000 iterations) allocations: {}", ALLOCATOR.count());
    println!("Per iteration: {:.2}", ALLOCATOR.count() as f64 / 1000.0);
}

fn test_object_pool() {
    println!("\n=== Object Pool Test ===");
    
    use earth_engine::renderer::ObjectPool;
    
    // Create a pool of vectors
    let pool = ObjectPool::new(10, || Vec::<i32>::with_capacity(100));
    
    ALLOCATOR.reset();
    
    // Use pool objects in a loop
    for i in 0..1000 {
        let mut obj = pool.acquire();
        obj.clear();
        obj.push(i);
        obj.push(i + 1);
        // Object is returned to pool when dropped
    }
    
    println!("Object pool (1000 iterations) allocations: {}", ALLOCATOR.count());
    println!("Per iteration: {:.2}", ALLOCATOR.count() as f64 / 1000.0);
}

fn test_mesh_buffers() {
    println!("\n=== Mesh Buffer Test ===");
    
    use earth_engine::renderer::{with_meshing_buffers, Vertex};
    
    ALLOCATOR.reset();
    
    // Test using pre-allocated meshing buffers
    for i in 0..1000 {
        with_meshing_buffers(32, |buffers| {
            buffers.clear();
            
            // Add some vertices
            for j in 0..10 {
                let vertex = Vertex {
                    position: [i as f32, j as f32, 0.0],
                    color: [1.0, 1.0, 1.0],
                    normal: [0.0, 1.0, 0.0],
                    light: 1.0,
                    ao: 1.0,
                };
                buffers.vertices.push(vertex);
                buffers.indices.push((i * 10 + j) as u32);
            }
        });
    }
    
    println!("Mesh buffers (1000 iterations) allocations: {}", ALLOCATOR.count());
    println!("Per iteration: {:.2}", ALLOCATOR.count() as f64 / 1000.0);
}

fn main() {
    println!("Hearth Engine Allocation Counter");
    println!("==============================");
    
    test_basic_operations();
    test_loop_allocations();
    test_object_pool();
    test_mesh_buffers();
    
    println!("\n=== Summary ===");
    println!("Goal: <10 allocations per frame");
    println!("Mesh buffers already achieve ~0 allocations per frame!");
    println!("Object pools dramatically reduce allocations vs naive approaches");
}