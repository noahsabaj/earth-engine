# Thread Pool Consolidation

## Overview

This document describes the thread pool consolidation performed to prevent thread exhaustion and improve resource utilization across the Earth Engine.

## Problem

Multiple systems were creating their own thread pools:
- `ParallelWorld` - world generation thread pool
- `AsyncMeshBuilder` - mesh building thread pool  
- `ParallelLightPropagator` - lighting thread pool
- `ParallelProcessor` - process execution thread pool
- `ParallelQueryExecutor` - spatial query thread pool
- Various tokio runtimes for async operations

This could lead to:
- Thread exhaustion (too many threads competing for CPU cores)
- Poor cache locality
- Increased context switching overhead
- Difficulty in managing thread resources

## Solution

Created a centralized `ThreadPoolManager` that:

1. **Manages thread pools by category** - Different workload types get appropriate thread pools:
   - `WorldGeneration` - Chunk generation and world processing
   - `MeshBuilding` - Mesh generation and optimization
   - `Lighting` - Light propagation calculations
   - `Physics` - Physics simulations
   - `Network` - Network I/O operations
   - `FileIO` - File I/O and streaming
   - `Compute` - General compute tasks

2. **Configurable thread distribution** - Default configuration:
   - Total threads: CPU cores - 2 (leave cores for OS/main thread)
   - World generation: 1/3 of threads
   - Mesh building: 1/3 of threads
   - Lighting: 1/6 of threads
   - Other categories: Proportionally distributed

3. **Single async runtime** - One tokio runtime for all async operations

4. **Performance metrics** - Tracks:
   - Tasks submitted per category
   - Tasks completed per category
   - Average task execution time

## Implementation

### Core Components

1. **ThreadPoolManager** (`src/thread_pool/thread_pool.rs`)
   - Global singleton instance
   - Lazy pool creation
   - Thread naming for debugging
   - Configurable stack size

2. **ThreadPoolConfig**
   - Total thread count
   - Per-category limits
   - Thread naming enable/disable
   - Stack size configuration

3. **PoolCategory** enum
   - Categorizes different workload types
   - Enables targeted thread allocation

### Updated Systems

1. **ParallelWorld** (`src/world/parallel_world.rs`)
   - Removed `generation_pool` field
   - Uses `ThreadPoolManager::global().spawn(PoolCategory::WorldGeneration, ...)`

2. **AsyncMeshBuilder** (`src/renderer/async_mesh_builder.rs`)
   - Removed `mesh_pool` field
   - Uses `ThreadPoolManager::global().spawn(PoolCategory::MeshBuilding, ...)`

3. **ParallelLightPropagator** (`src/lighting/parallel_propagator.rs`)
   - Removed `light_pool` field
   - Uses `ThreadPoolManager::global().execute(PoolCategory::Lighting, ...)`

4. **ParallelProcessor** (`src/process/parallel_processor.rs`)
   - Removed `thread_pool` field
   - Uses `ThreadPoolManager::global().execute(PoolCategory::Compute, ...)`

5. **ParallelQueryExecutor** (`src/spatial_index/parallel_query.rs`)
   - Removed `thread_pool` field
   - Uses `ThreadPoolManager::global().execute(PoolCategory::Compute, ...)`

### Usage Examples

```rust
// Spawn a world generation task
ThreadPoolManager::global().spawn(PoolCategory::WorldGeneration, move || {
    // Generate chunks...
});

// Execute a compute task and get result
let result = ThreadPoolManager::global().execute(PoolCategory::Compute, || {
    // Perform computation...
    42
});

// Convenience functions
ThreadPoolManager::execute_world_gen(|| { /* ... */ });
ThreadPoolManager::execute_mesh_build(|| { /* ... */ });
ThreadPoolManager::spawn_async(async { /* ... */ });
```

## Benefits

1. **Resource Control** - Central management prevents over-allocation
2. **Better Performance** - Optimized thread distribution based on workload
3. **Easier Debugging** - Named threads make debugging easier
4. **Flexibility** - Can adjust thread counts at runtime
5. **Monitoring** - Built-in performance metrics

## Configuration

The system uses sensible defaults but can be customized:

```rust
let mut config = ThreadPoolConfig::default();
config.total_threads = 16; // Override total thread count
config.category_limits.insert(PoolCategory::WorldGeneration, 8); // More threads for world gen

ThreadPoolManager::initialize(config)?;
```

## Future Improvements

1. Dynamic thread pool resizing based on workload
2. Priority-based task scheduling within categories
3. Thread pool warmup for reduced latency
4. Integration with profiling system for auto-tuning
5. Per-category queue depth limits