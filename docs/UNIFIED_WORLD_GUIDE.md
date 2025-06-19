# Unified World Module Guide

## Overview

The `world_unified` module represents the future of Hearth Engine - a GPU-first architecture that unifies CPU and GPU world management into a single, coherent system.

## Key Benefits

1. **Single Data Representation** - No more duplicate CPU/GPU structures
2. **Zero-Copy Operations** - Data lives where it's processed (GPU)
3. **Automatic Synchronization** - No manual upload/download
4. **10-100x Performance** - Massive parallelization for all operations
5. **Type Safety** - Compile-time GPU layout validation

## Architecture

```
world_unified/
├── core/           # Unified types (GPU-layout from the start)
├── storage/        # GPU-primary, CPU-fallback storage
├── compute/        # All GPU kernels and operations
├── generation/     # GPU-accelerated world generation
├── management/     # High-level world management
└── interfaces/     # Clean API boundaries
```

## Basic Usage

```rust
use hearth_engine::{
    UnifiedWorldManager, UnifiedWorldConfig,
    VoxelPos, BlockId,
};

// Create world manager
let config = UnifiedWorldConfig {
    chunk_size: 32,
    render_distance: 16,
    ..Default::default()
};

let world = UnifiedWorldManager::new(device, queue, config)?;

// All operations are GPU-accelerated
world.set_block(VoxelPos { x: 10, y: 20, z: 30 }, BlockId(1));
let block = world.get_block(VoxelPos { x: 10, y: 20, z: 30 });
```

## GPU-First Operations

### Terrain Generation
```rust
// Runs entirely on GPU using SOA layout
let generator = UnifiedGenerator::new(device, config)?;
// Generates chunks in parallel on GPU
```

### Physics Simulation
```rust
// Dispatch physics kernel
let compute = ComputeEngine::new(device, queue, config)?;
compute.dispatch(ComputeCommand::RunPhysics { delta_time: 0.016 });
```

### Batch Operations
```rust
// Update thousands of blocks in one GPU dispatch
let modifications = vec![
    (VoxelPos { x: 0, y: 0, z: 0 }, BlockId(1)),
    // ... thousands more
];
world.batch_set_blocks(modifications);
```

## Migration from Old System

### Old Way (world + world_gpu)
```rust
// CPU generation
let chunk = world.generate_chunk(pos);
// Manual GPU upload
chunk.upload_to_gpu();
// GPU operation
gpu_world.process_chunk(pos);
// Manual download
let result = gpu_world.download_chunk(pos);
```

### New Way (world_unified)
```rust
// Everything stays on GPU
world.generate_chunk(pos); // GPU generation
world.process_chunk(pos);  // GPU processing
// No transfers needed!
```

## Performance Considerations

1. **Minimize CPU Access** - Reading individual blocks from CPU is slow
2. **Batch Operations** - Group modifications for GPU dispatch
3. **Use Compute Shaders** - Write custom kernels for complex operations
4. **Profile GPU Usage** - Use tools like RenderDoc or NSight

## Advanced Features

### Custom GPU Kernels
```rust
// Add custom compute shaders
compute.register_kernel("my_erosion", include_str!("erosion.wgsl"));
compute.dispatch(ComputeCommand::Custom { 
    kernel: "my_erosion",
    workgroups: (32, 32, 32),
});
```

### Memory Management
```rust
// Automatic GPU memory management
let stats = world.storage.memory_stats();
println!("GPU memory: {} MB", stats.gpu_memory_mb);
println!("Chunks loaded: {}", stats.loaded_chunks);
```

### Parallel Chunk Loading
```rust
// Load multiple chunks in parallel on GPU
let positions = vec![ChunkPos { x: 0, y: 0, z: 0 }, /* ... */];
world.load_chunks_parallel(&positions).await;
```

## Debugging

Enable debug logging:
```
RUST_LOG=hearth_engine::world_unified=debug cargo run
```

Check GPU validation:
```rust
// Validates all GPU type layouts
world.validate_gpu_types()?;
```

## Future Roadmap

1. **Streaming Terrain** - GPU-driven LOD and streaming
2. **Advanced Physics** - Fluid simulation, destruction
3. **Neural Generation** - ML-based terrain generation
4. **Multi-GPU Support** - Scale across multiple GPUs

## Example

See `examples/test_unified_world.rs` for a complete working example.

```bash
cargo run --example test_unified_world
```