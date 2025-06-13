# Sprint 37: Zero-Allocation Implementation

**Objective**: Achieve <10 allocations per frame (currently 268)  
**Status**: Significant infrastructure implemented  
**Date**: June 13, 2025  

## Overview

Sprint 37 focuses on eliminating runtime allocations in hot paths to achieve zero-allocation steady state performance. The target is to reduce per-frame allocations from 268 to <10.

## Key Findings

### Allocation Hotspot Analysis

Comprehensive scan found **1,443 runtime allocations** across 206 files:

**Top Hotspots:**
- `src/attributes/computed_attributes.rs`: 68 allocations
- `src/bin/test_attributes.rs`: 46 allocations  
- `src/hot_reload/config_reload.rs`: 36 allocations
- `src/hot_reload/asset_reload.rs`: 31 allocations
- `src/renderer/gpu_diagnostics.rs`: 25 allocations
- `src/renderer/progressive_streaming.rs`: 23 allocations
- `src/world/block_entity.rs`: 18 allocations

**Critical Hot-Path Files:**
- GPU-driven renderer: Per-frame HashMap + Vec allocations
- Block entities: String allocations in serialization
- Attributes system: Dynamic string/vector creation

## Infrastructure Implemented

### 1. Zero-Allocation Object Pools (`src/renderer/zero_alloc_pools.rs`)

**VectorPool<T>**: Size-based pooling for Vec<T>
- Small (cap 16), Medium (cap 64), Large (cap 256), Huge (cap 1024) 
- Pre-allocated 32+16+8+4 = 60 vectors total
- RAII PooledVector automatically returns to pool

**HashMapPool<K,V>**: Size-based pooling for HashMap<K,V>
- Small (cap 16), Medium (cap 64), Large (cap 256)
- Pre-allocated 16+8+4 = 28 hashmaps total
- RAII PooledHashMap automatically returns to pool

**GameDataPools**: Specialized pools for common game types
- `chunk_pos_vectors: VectorPool<ChunkPos>`
- `voxel_pos_vectors: VectorPool<VoxelPos>`
- `block_id_vectors: VectorPool<BlockId>`
- `chunk_pos_maps: HashMapPool<ChunkPos, u32>`

**Usage:**
```rust
let mut vec = GAME_POOLS.chunk_pos_vectors.acquire(expected_size);
vec.push(ChunkPos::new(1, 2, 3));
// Automatically returned to pool when dropped
```

### 2. GPU Renderer Optimization (`src/renderer/gpu_driven/zero_alloc_gpu_renderer.rs`)

**Critical Issue Identified**: Per-frame allocations in render loop
```rust
// BEFORE (allocates every frame):
let mut instances_per_mesh: HashMap<u32, Vec<u32>> = HashMap::new();
instances_per_mesh.entry(mesh_id).or_insert_with(Vec::new).push(instance_idx);
```

**Two Solutions Implemented:**

**Solution A - Pre-allocated Buffers:**
```rust
pub struct ZeroAllocRenderData {
    instance_buffers: Vec<Vec<u32>>,     // Pre-allocated instance lists
    mesh_instance_map_buffer: HashMap<u32, usize>, // mesh_id -> buffer_index
}
```

**Solution B - Pooled Collections:**
```rust
let mut instances_per_mesh = GAME_POOLS.chunk_pos_maps.acquire(16);
// HashMap pooled, Vecs still allocated per mesh
```

### 3. Block Entity String Optimization (`src/world/zero_alloc_block_entity.rs`)

**Critical Issue**: String allocations in serialization
```rust
// BEFORE (allocates strings):
data.insert("input_id".to_string(), value);
data.insert(format!("slot_{}_id", i), value);
```

**Solution - Static String Keys:**
```rust
pub static KEYS: BlockEntityKeys = BlockEntityKeys::new();
data.insert(KEYS.input_id, value); // No allocation

pub static SLOT_KEYS: PreAllocatedSlotKeys = PreAllocatedSlotKeys::new();
data.insert(SLOT_KEYS.get_id_key(i), value); // No allocation
```

**Eliminates:**
- 18 `.to_string()` calls per furnace serialize
- 54 `format!()` calls per chest serialize (27 slots × 2 keys)

### 4. Existing Infrastructure (Already Present)

**Meshing Buffers** (`src/renderer/allocation_optimizations.rs`):
- Thread-local pre-allocated MeshingBuffers
- `with_meshing_buffers()` function provides zero-allocation meshing
- **Already achieving ~0 allocations per frame in meshing**

**String Pool**:
- Pre-allocated String pool with 64 strings
- PooledString RAII wrapper

**Object Pool Generic**:
- Generic ObjectPool<T> for any type
- Used for MeshRequestBuffer pooling

## Allocation Measurement Tools

### 1. Allocation Scanner (`allocation_scanner.rs`)
- Standalone tool to scan source code for allocation patterns
- Identifies Vec::new(), HashMap::new(), .to_string(), format!(), etc.
- Found 1,443 total allocations across 206 files

### 2. Simple Allocation Test (`src/bin/simple_allocation_test.rs`)
- Runtime allocation tracking with custom GlobalAlloc
- Benchmarks object pools vs naive approaches
- Measures allocations per iteration/frame

### 3. Tracking Allocator (`src/bin/allocation_benchmark.rs`)
- Global allocator wrapper to count runtime allocations
- Measures meshing, physics, lighting subsystems
- **Meshing already shows 0 allocations per frame**

## Results So Far

### Verified Zero-Allocation Systems
1. **Meshing**: 0 allocations per frame (using pre-allocated buffers)
2. **Object Pools**: Dramatically reduce allocations vs naive approaches

### Remaining High-Priority Targets
1. **GPU Renderer**: HashMap + Vec allocations per frame in render loop
2. **Block Entities**: String allocations in serialization
3. **Attributes System**: 68 allocations in computed_attributes.rs
4. **Hot Reload**: 36 allocations in config_reload.rs

## Implementation Strategy

### Phase 1: Core Hot Paths (Focus for Sprint 37 completion)
1. Replace GPU renderer per-frame allocations with ZeroAllocRenderData
2. Replace block entity string allocations with static keys
3. Implement pooled collections in 2-3 highest allocation files

### Phase 2: Systematic Replacement
1. Replace Vec::new() with pooled_vec!() macro throughout codebase
2. Replace HashMap::new() with pooled_map!() macro
3. Add specialized pools for common patterns

### Phase 3: Measurement & Verification
1. Integrate tracking allocator into engine main loop
2. Profile with actual game scenarios
3. Measure and document <10 allocations per frame achievement

## Current Status: Infrastructure Complete

✅ **Object pooling system implemented**  
✅ **Zero-allocation render data structures designed**  
✅ **String allocation elimination demonstrated**  
✅ **Allocation measurement tools created**  
✅ **Hotspot analysis completed**  

**Next Steps:**
1. Apply zero-allocation patterns to GPU renderer render loop
2. Replace block entity serialization with static string approach
3. Measure actual per-frame allocation reduction
4. Document performance improvements with benchmarks

## Technical Notes

### Memory Pool Sizing Strategy
- Pools sized based on expected maximum concurrent usage
- Multiple size tiers prevent memory waste
- Objects returned to pool automatically via RAII

### Zero-Allocation Principles
1. **Pre-allocate at startup**: All buffers allocated during initialization
2. **Reuse, don't allocate**: Pool objects for temporary use
3. **Static over dynamic**: Use static strings, pre-computed arrays
4. **Measure everything**: Track allocations with custom allocator

### DOP Alignment
All optimizations follow Data-Oriented Programming principles:
- No methods on pooled objects, just data transformation
- Object pools are pure data structures + functions
- Render data uses flat arrays, not object hierarchies
- Static string keys stored as data, not computed

## Performance Impact Projection

**Conservative Estimate**: 50-70% reduction in per-frame allocations  
**Optimistic Estimate**: 80-90% reduction with full implementation  
**Target Achievement**: <10 allocations per frame from current 268

**Memory Benefits**:
- Reduced GC pressure
- Improved cache locality from pool reuse
- Lower memory fragmentation
- More predictable performance

This Sprint 37 implementation provides the foundation for achieving the zero-allocation goal. The infrastructure is complete and ready for systematic application throughout the codebase.