# Reality Check Profiler

A brutally honest performance profiler for Earth Engine that exposes the ACTUAL performance, not marketing claims.

## Current Reality

- **Claimed**: "80-85% GPU compute"
- **Actual**: 0.8 FPS (terrible performance)
- **Truth**: Most time spent on CPU, not GPU

## Features

- **Frame-level profiling**: Total frame time breakdown
- **CPU/GPU split**: Actual measurement of where time is spent
- **Memory tracking**: Exposes "zero-allocation" lies
- **Blocking operations**: Identifies what blocks the main thread
- **GPU timestamps**: Real GPU execution time (when available)
- **System metrics**: Per-system performance breakdown
- **Brutal honesty mode**: Logs warnings for terrible performance

## Usage

### Basic Integration

```rust
use earth_engine::profiling::{
    RealityCheckProfiler, BlockingType,
    reality_begin_frame, reality_end_frame,
    time_cpu_operation, generate_reality_report,
};

// Create profiler with GPU support
let profiler = RealityCheckProfiler::new(Some(&device), Some(&queue));

// In your render loop:
reality_begin_frame(&profiler);

// Time expensive operations
time_cpu_operation(&profiler, "chunk_generation", BlockingType::ChunkGeneration, || {
    // Your chunk generation code
});

// End frame and get metrics
reality_end_frame(&profiler, Some(&device), Some(&queue)).await;

// Generate brutal honesty report
println!("{}", generate_reality_report(&profiler));
```

### GPU Timing

```rust
let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    label: Some("Frame"),
});

// Start GPU timing
let gpu_start = write_gpu_timestamp(&profiler, &mut encoder);

// ... GPU work ...

// End GPU timing
write_gpu_timestamp(&profiler, &mut encoder);
```

### Memory Tracking

To track memory allocations, you need to use the `TrackingAllocator` as a global allocator:

```rust
#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();

// Then in main:
let mut profiler = RealityCheckProfiler::new(Some(&device), Some(&queue));
profiler.set_memory_tracker(Arc::new(ALLOCATOR));
```

## Blocking Operation Types

- `GpuSync`: Waiting for GPU to finish
- `MemoryAllocation`: Memory allocation
- `FileIO`: File I/O operations  
- `ChunkGeneration`: Voxel chunk generation
- `MeshBuilding`: Mesh construction
- `PhysicsUpdate`: Physics simulation
- `CpuWork`: General CPU work

## Example Output

```
=== EARTH ENGINE REALITY CHECK REPORT ===

ACTUAL PERFORMANCE: 0.8 FPS
STATUS: SLIDESHOW MODE - This is not a real-time engine

FRAME TIME BREAKDOWN:
  Total: 1250.0ms
  CPU Main Thread: 875.0ms (70.0%)
  GPU Execution: 250.0ms (20.0%)
  GPU Wait/Sync: 125.0ms (10.0%)

GPU UTILIZATION: 20.0%
REALITY: This is NOT a GPU-first engine. CPU is the bottleneck.

RENDERING STATS:
  Draw Calls: 1523
  Compute Dispatches: 45

MEMORY BEHAVIOR:
  Allocations per frame: 4096 KB
  Deallocations per frame: 3072 KB
  WARNING: MEMORY LEAK - Growing by 1024 KB/frame

SYSTEM BREAKDOWN:
  chunk_generation: 450.0ms CPU [BLOCKING MAIN THREAD]
  physics: 200.0ms CPU [BLOCKING MAIN THREAD]
  mesh_building: 150.0ms CPU [BLOCKING MAIN THREAD]
  rendering: 75.0ms CPU, 250.0ms GPU

BIGGEST PERFORMANCE LIES EXPOSED:
  ❌ "80-85% GPU compute" - ACTUAL: 20.0% GPU utilization
  ❌ "Real-time performance" - ACTUAL: 0.8 FPS
  ❌ "Zero-allocation" - ACTUAL: 4 MB allocated per frame

=== END REALITY CHECK ===
```

## Running the Benchmark

```bash
# Run the reality check benchmark
cargo run --release --bin reality_check_benchmark

# Run the demo with visual output
cargo run --release --example reality_check_demo
```

## Integration Tips

1. **Start Early**: Add profiling from the beginning to catch problems early
2. **Profile Release Builds**: Debug builds have different performance characteristics
3. **Measure Everything**: Don't trust assumptions - measure actual performance
4. **Be Honest**: Accept the brutal truth about performance and fix it
5. **Track Trends**: Watch for performance regressions over time

## Future Improvements

- [ ] Chrome tracing format export
- [ ] Network profiling for multiplayer
- [ ] Asset loading profiling
- [ ] Per-thread CPU tracking
- [ ] GPU memory tracking
- [ ] Automated performance regression detection