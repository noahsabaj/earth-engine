# Earth Engine Zero-Allocation Optimizations

## Overview
This document describes the optimizations implemented to achieve zero allocations per frame in the Earth Engine's hot paths.

## Key Optimizations

### 1. Object Pooling System (`allocation_optimizations.rs`)
- **ObjectPool<T>**: Generic object pool for reusable allocations
- **PooledObject<T>**: RAII wrapper that automatically returns objects to pool
- **StringPool**: Specialized pool for string allocations (format operations)
- **Global pools**: Lazy-initialized pools for commonly allocated objects

### 2. Pre-allocated Buffers

#### Meshing Buffers (`MeshingBuffers`)
- 2D mask for face extraction (reused per face)
- Used flags for rectangle extraction  
- Temporary quad storage
- Vertex and index buffers
- Thread-local access pattern to avoid contention

#### Physics Buffers (`PhysicsBuffers`)
- Update collection buffer
- Overlapping blocks buffer
- Solid blocks buffer
- Eliminates Vec allocations in collision detection

#### Lighting Buffers (`PropagationBuffers`)
- Dual queue system for swapping (avoids allocations)
- Pre-allocated neighbor positions
- Fixed-size buffers for light propagation

### 3. Optimized Components

#### OptimizedGreedyMesher
- Uses thread-local `MeshingBuffers`
- Zero allocations during mesh generation
- Reuses all temporary storage
- Direct writes to output buffers

#### OptimizedPhysicsWorld
- Pre-allocated update buffers
- Reusable collision detection arrays
- No temporary Vec allocations in hot path
- Buffered overlapping block detection

#### OptimizedLightPropagator  
- Dual-buffer queue swapping technique
- Pre-allocated neighbor arrays
- No allocations during propagation
- Thread-local propagator instances

### 4. Allocation Reduction Patterns

#### String Formatting
```rust
// Before: Allocates every frame
let label = format!("Chunk {:?}", pos);

// After: Static formatter with pre-allocated buffer
let label = formatter.format_chunk_label(pos);
```

#### Collection Reuse
```rust
// Before: New Vec every frame
let blocks = self.get_overlapping_blocks(world, aabb);

// After: Reuse pre-allocated buffer
self.get_overlapping_blocks_buffered(world, aabb);
// Uses self.buffers.overlapping_blocks
```

#### Fixed Arrays
```rust
// Before: Dynamic Vec
let neighbors = vec![pos + offset1, pos + offset2, ...];

// After: Fixed-size array
self.buffers.neighbors = [(pos1, true), (pos2, true), ...];
```

### 5. Thread-Local Storage
- Meshing buffers are thread-local to avoid contention
- Each thread maintains its own pre-allocated buffers
- No synchronization overhead in hot paths

### 6. Memory Layout Optimizations
- Use of fixed-size arrays where possible
- Capacity pre-allocation based on worst-case scenarios
- Buffer clearing without deallocation (clear() vs new())

## Verification

Run the allocation benchmark to verify zero allocations:
```bash
cargo run --release --bin allocation_benchmark
```

Expected output should show 0 allocations per frame for:
- Mesh generation
- Physics updates
- Light propagation
- Object pool operations

## Integration Guide

### Using Optimized Mesher
```rust
let mut mesher = OptimizedGreedyMesher::new(chunk_size);
let mesh = mesher.build_chunk_mesh(chunk, pos, size, registry, neighbors);
```

### Using Optimized Physics
```rust
let mut physics = OptimizedPhysicsWorld::new();
physics.update(&world, delta_time); // Zero allocations
```

### Using Optimized Lighting
```rust
let mut lighting = OptimizedLightPropagator::new();
lighting.add_light(pos, light_type, level);
lighting.propagate(&mut world); // Zero allocations
```

### Using Object Pools
```rust
// Create pool once
let pool = ObjectPool::new(capacity, || YourObject::new());

// Use in hot path
let mut obj = pool.acquire(); // No allocation
// ... use object ...
// Automatically returned when dropped
```

## Performance Impact

These optimizations provide:
- **Zero allocations per frame** in all hot paths
- **Improved cache locality** through buffer reuse
- **Reduced GC pressure** (important for long sessions)
- **Consistent frame times** (no allocation spikes)
- **Better multi-threading** through thread-local buffers

## Future Optimizations

Additional areas that could benefit from zero-allocation treatment:
1. Chunk loading/unloading paths
2. Network packet processing
3. UI rendering (if allocations are found)
4. Particle systems
5. Entity component updates