/// Benchmark to verify zero-allocation rendering and update loops
/// Measures allocations in hot paths using custom allocator tracking

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use earth_engine::{
    renderer::{OptimizedGreedyMesher, ObjectPool, with_meshing_buffers},
    physics::PhysicsWorldData,
    lighting::OptimizedLightPropagator,
    world::{World, VoxelPos, BlockId, Chunk, ChunkPos},
    Camera,
};
use cgmath::Point3;

/// Tracking allocator to measure allocations
struct TrackingAllocator {
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
    bytes_allocated: AtomicUsize,
    bytes_deallocated: AtomicUsize,
}

impl TrackingAllocator {
    const fn new() -> Self {
        Self {
            allocations: AtomicUsize::new(0),
            deallocations: AtomicUsize::new(0),
            bytes_allocated: AtomicUsize::new(0),
            bytes_deallocated: AtomicUsize::new(0),
        }
    }
    
    fn reset(&self) {
        self.allocations.store(0, Ordering::SeqCst);
        self.deallocations.store(0, Ordering::SeqCst);
        self.bytes_allocated.store(0, Ordering::SeqCst);
        self.bytes_deallocated.store(0, Ordering::SeqCst);
    }
    
    fn report(&self) -> AllocationReport {
        AllocationReport {
            allocations: self.allocations.load(Ordering::SeqCst),
            deallocations: self.deallocations.load(Ordering::SeqCst),
            bytes_allocated: self.bytes_allocated.load(Ordering::SeqCst),
            bytes_deallocated: self.bytes_deallocated.load(Ordering::SeqCst),
        }
    }
}

#[derive(Debug)]
struct AllocationReport {
    allocations: usize,
    deallocations: usize,
    bytes_allocated: usize,
    bytes_deallocated: usize,
}

impl AllocationReport {
    fn net_allocations(&self) -> isize {
        self.allocations as isize - self.deallocations as isize
    }
    
    fn net_bytes(&self) -> isize {
        self.bytes_allocated as isize - self.bytes_deallocated as isize
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocations.fetch_add(1, Ordering::SeqCst);
        self.bytes_allocated.fetch_add(layout.size(), Ordering::SeqCst);
        System.alloc(layout)
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocations.fetch_add(1, Ordering::SeqCst);
        self.bytes_deallocated.fetch_add(layout.size(), Ordering::SeqCst);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();

fn create_test_chunk() -> Chunk {
    let mut chunk = Chunk::new(ChunkPos::new(0, 0, 0), 32);
    
    // Add some blocks
    for y in 0..16 {
        for x in 0..32 {
            for z in 0..32 {
                if y < 8 {
                    chunk.set_block(x, y, z, BlockId(1)); // Stone
                } else if y < 12 {
                    chunk.set_block(x, y, z, BlockId(2)); // Dirt
                } else if y == 12 {
                    chunk.set_block(x, y, z, BlockId(3)); // Grass
                }
            }
        }
    }
    
    chunk
}

fn benchmark_meshing() {
    println!("\n=== Meshing Benchmark ===");
    
    let mut mesher = OptimizedGreedyMesher::new(32);
    let chunk = create_test_chunk();
    let chunk_arc = Arc::new(parking_lot::RwLock::new(chunk));
    let registry = Arc::new(earth_engine::BlockRegistry::new());
    
    // Warmup
    for _ in 0..10 {
        with_meshing_buffers(32, |buffers| {
            // Simulate mesh building
            buffers.clear();
        });
    }
    
    // Reset allocator
    ALLOCATOR.reset();
    let start = Instant::now();
    
    // Run meshing 1000 times
    const ITERATIONS: usize = 1000;
    for _ in 0..ITERATIONS {
        let chunk = chunk_arc.read();
        let mesh = mesher.build_chunk_mesh(
            &chunk,
            ChunkPos::new(0, 0, 0),
            32,
            &registry,
            &[None, None, None, None, None, None],
        );
        
        // Simulate using the mesh
        let _ = mesh.vertices.len();
        let _ = mesh.indices.len();
    }
    
    let duration = start.elapsed();
    let report = ALLOCATOR.report();
    
    println!("Iterations: {}", ITERATIONS);
    println!("Duration: {:?}", duration);
    println!("Per iteration: {:?}", duration / ITERATIONS as u32);
    println!("Allocations: {} (per frame: {:.2})", 
        report.allocations, 
        report.allocations as f64 / ITERATIONS as f64
    );
    println!("Net allocations: {}", report.net_allocations());
    println!("Bytes allocated: {} (per frame: {:.2})", 
        report.bytes_allocated,
        report.bytes_allocated as f64 / ITERATIONS as f64
    );
}

fn benchmark_physics() {
    println!("\n=== Physics Benchmark ===");
    
    let mut physics_world = PhysicsWorldData::new();
    let mut world = World::new(32);
    
    // Add some blocks for collision
    for x in 0..10 {
        for z in 0..10 {
            world.set_block(VoxelPos::new(x, 0, z), BlockId(1));
        }
    }
    
    // Add a physics body
    physics_world.add_entity(
        Point3::new(5.0, 10.0, 5.0),
        cgmath::Vector3::zero(),
        cgmath::Vector3::new(0.8, 1.8, 0.8),
        80.0,
        0.8,
        0.0,
    );
    
    // Warmup
    for _ in 0..100 {
        physics_world.update(&world, 0.016);
    }
    
    // Reset allocator
    ALLOCATOR.reset();
    let start = Instant::now();
    
    // Run physics updates
    const ITERATIONS: usize = 10000;
    for _ in 0..ITERATIONS {
        physics_world.update(&world, 0.016);
    }
    
    let duration = start.elapsed();
    let report = ALLOCATOR.report();
    
    println!("Iterations: {}", ITERATIONS);
    println!("Duration: {:?}", duration);
    println!("Per iteration: {:?}", duration / ITERATIONS as u32);
    println!("Allocations: {} (per frame: {:.2})", 
        report.allocations, 
        report.allocations as f64 / ITERATIONS as f64
    );
    println!("Net allocations: {}", report.net_allocations());
    println!("Bytes allocated: {} (per frame: {:.2})", 
        report.bytes_allocated,
        report.bytes_allocated as f64 / ITERATIONS as f64
    );
}

fn benchmark_lighting() {
    println!("\n=== Lighting Benchmark ===");
    
    let mut propagator = OptimizedLightPropagator::new();
    let mut world = World::new(32);
    
    // Add some blocks
    for x in 0..32 {
        for z in 0..32 {
            for y in 0..16 {
                if y < 10 {
                    world.set_block(VoxelPos::new(x, y, z), BlockId(1));
                }
            }
        }
    }
    
    // Warmup
    for _ in 0..10 {
        propagator.add_light(VoxelPos::new(16, 12, 16), earth_engine::lighting::LightType::Block, 15);
        propagator.propagate(&mut world);
        propagator.clear();
    }
    
    // Reset allocator
    ALLOCATOR.reset();
    let start = Instant::now();
    
    // Run lighting updates
    const ITERATIONS: usize = 1000;
    for i in 0..ITERATIONS {
        // Add a light source
        let x = (i % 30) as i32 + 1;
        let z = ((i / 30) % 30) as i32 + 1;
        propagator.add_light(VoxelPos::new(x, 12, z), earth_engine::lighting::LightType::Block, 15);
        
        // Remove another light
        if i > 30 {
            let rx = ((i - 30) % 30) as i32 + 1;
            let rz = (((i - 30) / 30) % 30) as i32 + 1;
            propagator.remove_light(VoxelPos::new(rx, 12, rz), earth_engine::lighting::LightType::Block, 15);
        }
        
        propagator.propagate(&mut world);
    }
    
    let duration = start.elapsed();
    let report = ALLOCATOR.report();
    
    println!("Iterations: {}", ITERATIONS);
    println!("Duration: {:?}", duration);
    println!("Per iteration: {:?}", duration / ITERATIONS as u32);
    println!("Allocations: {} (per frame: {:.2})", 
        report.allocations, 
        report.allocations as f64 / ITERATIONS as f64
    );
    println!("Net allocations: {}", report.net_allocations());
    println!("Bytes allocated: {} (per frame: {:.2})", 
        report.bytes_allocated,
        report.bytes_allocated as f64 / ITERATIONS as f64
    );
}

fn benchmark_object_pools() {
    println!("\n=== Object Pool Benchmark ===");
    
    #[derive(Clone)]
    struct TestObject {
        data: Vec<u8>,
    }
    
    let pool = ObjectPool::new(16, || TestObject {
        data: vec![0u8; 1024],
    });
    
    // Warmup
    for _ in 0..100 {
        let mut objects = Vec::new();
        for _ in 0..10 {
            objects.push(pool.acquire());
        }
        // Objects are returned to pool when dropped
    }
    
    // Reset allocator
    ALLOCATOR.reset();
    let start = Instant::now();
    
    // Run pool operations
    const ITERATIONS: usize = 10000;
    for _ in 0..ITERATIONS {
        // Acquire and release objects
        let mut obj1 = pool.acquire();
        let mut obj2 = pool.acquire();
        let mut obj3 = pool.acquire();
        
        // Simulate some work
        obj1.data[0] = 1;
        obj2.data[0] = 2;
        obj3.data[0] = 3;
        
        // Objects automatically returned when dropped
    }
    
    let duration = start.elapsed();
    let report = ALLOCATOR.report();
    
    println!("Iterations: {}", ITERATIONS);
    println!("Duration: {:?}", duration);
    println!("Per iteration: {:?}", duration / ITERATIONS as u32);
    println!("Allocations: {} (per frame: {:.2})", 
        report.allocations, 
        report.allocations as f64 / ITERATIONS as f64
    );
    println!("Net allocations: {}", report.net_allocations());
}

fn main() {
    println!("Earth Engine Zero-Allocation Benchmark");
    println!("=====================================");
    
    // Run benchmarks
    benchmark_meshing();
    benchmark_physics();
    benchmark_lighting();
    benchmark_object_pools();
    
    println!("\n=== Summary ===");
    println!("Zero-allocation goal: All hot paths should show 0 allocations per frame");
    println!("Any non-zero per-frame allocations indicate areas that need optimization");
}