# Dirty Chunk System Implementation Plan

## Executive Summary

This document outlines a comprehensive plan to implement a fully-featured, performant dirty chunk tracking system for Hearth Engine. The system will minimize GPU work by only updating voxel chunks that have changed, supporting the engine's goal of 10,000+ concurrent players at 144+ FPS.

## Problem Statement

### Current State
- **Partial Implementation**: Basic dirty flags exist but lack granularity
- **Full Chunk Remeshing**: Single block change triggers 125,000 voxel remesh (50³)
- **No Neighbor Awareness**: Edge block changes don't notify adjacent chunks
- **CPU Bottleneck**: All mesh generation happens on CPU before GPU upload
- **No Prioritization**: All dirty chunks treated equally regardless of camera distance

### Performance Impact
- Unnecessary remeshing of 99.99% unchanged data for single block edits
- Visual artifacts at chunk boundaries when neighbors aren't updated
- Frame drops during heavy building/destruction
- Poor scalability with player count

## Proposed Solution

A multi-tiered dirty tracking system with sub-chunk granularity, GPU-direct updates, and intelligent prioritization.

### Core Architecture

```rust
/// Dirty state tracking with sub-chunk granularity
pub struct ChunkDirtyState {
    /// 8x8x8 sub-regions = 512 bits (64 bytes)
    pub regions: BitVec,
    
    /// Optimization flag when entire chunk is dirty
    pub full_dirty: bool,
    
    /// Bit flags for which faces need neighbor notification
    /// Bits: +X, -X, +Y, -Y, +Z, -Z
    pub neighbor_flags: u8,
    
    /// Frame when chunk was marked dirty
    pub dirty_frame: u64,
    
    /// Priority score for update ordering
    pub priority: f32,
}

/// GPU-friendly block update structure
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BlockUpdate {
    pub position: [u32; 3],  // World position
    pub block_id: u32,       // New block type
}
```

## Implementation Phases

### Phase 1: Neighbor Notification System (1 week)

**Goal**: Eliminate chunk boundary artifacts

#### 1.1 Edge Detection
```rust
pub fn is_on_chunk_edge(local_pos: LocalVoxelPos) -> u8 {
    let mut edges = 0u8;
    if local_pos.x == 0 { edges |= EDGE_NEG_X; }
    if local_pos.x == CHUNK_SIZE - 1 { edges |= EDGE_POS_X; }
    if local_pos.y == 0 { edges |= EDGE_NEG_Y; }
    if local_pos.y == CHUNK_SIZE - 1 { edges |= EDGE_POS_Y; }
    if local_pos.z == 0 { edges |= EDGE_NEG_Z; }
    if local_pos.z == CHUNK_SIZE - 1 { edges |= EDGE_POS_Z; }
    edges
}
```

#### 1.2 Neighbor Marking
```rust
pub fn mark_block_dirty(
    dirty_chunks: &mut HashMap<ChunkPos, ChunkDirtyState>,
    world_pos: VoxelPos,
    current_frame: u64,
) {
    let chunk_pos = world_pos.to_chunk_pos();
    let local_pos = world_pos.to_local();
    
    // Mark primary chunk
    let state = dirty_chunks.entry(chunk_pos)
        .or_insert_with(|| ChunkDirtyState::new(current_frame));
    state.mark_region(local_pos);
    
    // Check edges and mark neighbors
    let edges = is_on_chunk_edge(local_pos);
    if edges & EDGE_NEG_X != 0 {
        mark_chunk_face_dirty(dirty_chunks, chunk_pos.offset(-1, 0, 0), EDGE_POS_X, current_frame);
    }
    // ... other edges
}
```

### Phase 2: Sub-Chunk Dirty Regions (2 weeks)

**Goal**: Reduce mesh generation by 90%+ for typical edits

#### 2.1 Region Mapping
```rust
/// Convert local voxel position to sub-region index
pub fn voxel_to_region(local_pos: LocalVoxelPos) -> usize {
    const REGION_SIZE: u32 = CHUNK_SIZE / 8; // 6.25 voxels per region
    let rx = (local_pos.x / REGION_SIZE).min(7);
    let ry = (local_pos.y / REGION_SIZE).min(7);
    let rz = (local_pos.z / REGION_SIZE).min(7);
    (rx + ry * 8 + rz * 64) as usize
}

/// Get voxel bounds for a region
pub fn region_bounds(region_idx: usize) -> (LocalVoxelPos, LocalVoxelPos) {
    const REGION_SIZE: u32 = CHUNK_SIZE / 8;
    let rz = (region_idx / 64) as u32;
    let ry = ((region_idx % 64) / 8) as u32;
    let rx = (region_idx % 8) as u32;
    
    let min = LocalVoxelPos::new(
        rx * REGION_SIZE,
        ry * REGION_SIZE,
        rz * REGION_SIZE,
    );
    let max = LocalVoxelPos::new(
        ((rx + 1) * REGION_SIZE).min(CHUNK_SIZE),
        ((ry + 1) * REGION_SIZE).min(CHUNK_SIZE),
        ((rz + 1) * REGION_SIZE).min(CHUNK_SIZE),
    );
    (min, max)
}
```

#### 2.2 Incremental Mesh Generation
```rust
pub fn generate_mesh_incremental(
    chunk_data: &ChunkData,
    dirty_state: &ChunkDirtyState,
    previous_mesh: Option<&ChunkMesh>,
) -> ChunkMesh {
    if dirty_state.full_dirty || previous_mesh.is_none() {
        return generate_mesh_full(chunk_data);
    }
    
    let mut mesh = previous_mesh.unwrap().clone();
    
    // Process only dirty regions
    for region_idx in dirty_state.regions.iter_ones() {
        let (min, max) = region_bounds(region_idx);
        regenerate_mesh_region(&mut mesh, chunk_data, min, max);
    }
    
    mesh
}
```

### Phase 3: Priority-Based Update Queue (1 week)

**Goal**: Maintain high FPS by prioritizing visible chunks

#### 3.1 Priority Calculation
```rust
pub fn calculate_chunk_priority(
    chunk_pos: ChunkPos,
    camera_pos: Vec3,
    camera_forward: Vec3,
    dirty_frame: u64,
    current_frame: u64,
) -> f32 {
    let chunk_center = chunk_pos.to_world_center();
    let to_chunk = (chunk_center - camera_pos).normalize();
    
    // Distance factor (inverse square)
    let distance = chunk_center.distance(camera_pos);
    let distance_factor = 1.0 / (1.0 + distance * distance * 0.0001);
    
    // View angle factor (chunks in view direction have higher priority)
    let angle_factor = (camera_forward.dot(to_chunk) + 1.0) * 0.5;
    
    // Age factor (older dirty chunks get priority boost)
    let age = (current_frame - dirty_frame) as f32;
    let age_factor = 1.0 + age * 0.01;
    
    distance_factor * angle_factor * age_factor
}
```

#### 3.2 Update Budget System
```rust
pub struct DirtyChunkProcessor {
    high_priority: BinaryHeap<(OrderedFloat<f32>, ChunkPos)>,
    medium_priority: BinaryHeap<(OrderedFloat<f32>, ChunkPos)>,
    low_priority: VecDeque<ChunkPos>,
    
    time_budget_ms: f32,
    chunks_per_frame_limit: usize,
}

impl DirtyChunkProcessor {
    pub fn process_frame(
        &mut self,
        delta_time: f32,
        camera: &CameraData,
    ) -> Vec<ChunkPos> {
        let start_time = Instant::now();
        let mut processed = Vec::new();
        
        // Dynamic budget based on frame time
        let target_ms = 1000.0 / TARGET_FPS as f32;
        self.time_budget_ms = (target_ms * 0.2).min(4.0); // 20% of frame, max 4ms
        
        // Process high priority first
        while processed.len() < self.chunks_per_frame_limit {
            if start_time.elapsed().as_secs_f32() * 1000.0 > self.time_budget_ms {
                break;
            }
            
            if let Some((_, chunk_pos)) = self.high_priority.pop() {
                processed.push(chunk_pos);
            } else if let Some((_, chunk_pos)) = self.medium_priority.pop() {
                processed.push(chunk_pos);
            } else if let Some(chunk_pos) = self.low_priority.pop_front() {
                processed.push(chunk_pos);
            } else {
                break;
            }
        }
        
        processed
    }
}
```

### Phase 4: GPU-Direct Block Updates (3 weeks)

**Goal**: Eliminate CPU bottleneck for block modifications

#### 4.1 GPU Update Buffer
```rust
/// GPU buffer for batched block updates
pub struct GpuBlockUpdateBuffer {
    /// Persistent mapped buffer for zero-copy updates
    pub update_buffer: wgpu::Buffer,
    
    /// Staging area for CPU writes
    pub staging: Vec<BlockUpdate>,
    
    /// Current write position
    pub write_pos: usize,
    
    /// Maximum updates per frame
    pub capacity: usize,
}

impl GpuBlockUpdateBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        const MAX_UPDATES: usize = 4096;
        
        let update_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Block Update Buffer"),
            size: (MAX_UPDATES * std::mem::size_of::<BlockUpdate>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            update_buffer,
            staging: Vec::with_capacity(MAX_UPDATES),
            write_pos: 0,
            capacity: MAX_UPDATES,
        }
    }
    
    pub fn add_update(&mut self, update: BlockUpdate) {
        if self.staging.len() < self.capacity {
            self.staging.push(update);
        }
    }
    
    pub fn flush(&mut self, queue: &wgpu::Queue) {
        if !self.staging.is_empty() {
            queue.write_buffer(
                &self.update_buffer,
                0,
                bytemuck::cast_slice(&self.staging),
            );
            self.write_pos = self.staging.len();
            self.staging.clear();
        }
    }
}
```

#### 4.2 GPU Update Compute Shader
```wgsl
struct BlockUpdate {
    position: vec3<u32>,
    block_id: u32,
}

@group(0) @binding(0) var<storage, read> updates: array<BlockUpdate>;
@group(0) @binding(1) var<storage, read_write> world_data: array<u32>;
@group(0) @binding(2) var<storage, read_write> dirty_chunks: array<u32>;

@compute @workgroup_size(64)
fn apply_block_updates(
    @builtin(global_invocation_id) id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let update_idx = id.x;
    if update_idx >= arrayLength(&updates) {
        return;
    }
    
    let update = updates[update_idx];
    let world_idx = morton_encode(update.position);
    
    // Update block
    world_data[world_idx] = update.block_id;
    
    // Mark chunk dirty
    let chunk_pos = update.position / CHUNK_SIZE;
    let chunk_idx = chunk_pos.x + chunk_pos.y * WORLD_SIZE + chunk_pos.z * WORLD_SIZE * WORLD_SIZE;
    atomicOr(&dirty_chunks[chunk_idx / 32u], 1u << (chunk_idx % 32u));
}
```

### Phase 5: Mesh Caching & Versioning (2 weeks)

**Goal**: Reuse unchanged mesh portions

#### 5.1 Versioned Mesh Cache
```rust
pub struct MeshCache {
    /// LRU cache of recent meshes
    cache: LruCache<(ChunkPos, u64), Arc<ChunkMesh>>,
    
    /// Version counter for each chunk
    versions: HashMap<ChunkPos, u64>,
    
    /// Memory budget in bytes
    memory_budget: usize,
    
    /// Current memory usage
    current_usage: AtomicUsize,
}

pub struct VersionedChunkMesh {
    /// Base mesh data
    pub mesh: ChunkMesh,
    
    /// Version number
    pub version: u64,
    
    /// Region versions for incremental updates
    pub region_versions: [u16; 512],
    
    /// Vertex runs grouped by region
    pub region_vertex_runs: Vec<VertexRun>,
}
```

## Critical Coding Philosophies

### 1. **Data-Oriented Programming (DOP) - MANDATORY**
```rust
// ❌ WRONG - OOP style
impl DirtyChunk {
    fn update(&mut self) { } // NO METHODS!
}

// ✅ CORRECT - DOP style
fn update_dirty_chunk(
    state: &mut ChunkDirtyState,
    chunk_data: &ChunkData,
    update_params: &UpdateParams,
) {
    // Transform data, no self
}
```

### 2. **GPU-First Thinking**
- Always ask: "Can GPU do this in parallel?"
- Batch operations for GPU efficiency
- Minimize CPU-GPU synchronization
- Use compute shaders for data transformation

### 3. **Zero Bandaids Policy**
- No temporary hacks in dirty tracking
- Build the system right from the start
- Performance problems require architectural solutions
- Document why, not just what

### 4. **Performance Over "Clean Code"**
- Bit manipulation over boolean arrays
- SOA over AOS for cache efficiency
- Intrinsics and SIMD where beneficial
- Profile before and after changes

### 5. **Single Source of Truth**
- Dirty state lives in ONE place
- No duplicate tracking between systems
- Clear ownership of dirty flags
- Explicit state transitions

## Success Metrics

The implementation will be considered successful when:

1. **Single Block Edit Performance**
   - < 0.1ms to mark chunk dirty
   - < 2ms to regenerate affected mesh region
   - Zero impact on non-adjacent chunks

2. **Bulk Edit Performance**
   - 1000 block edits/frame with < 5ms overhead
   - Linear scaling with edit count
   - Automatic batching of nearby edits

3. **Memory Efficiency**
   - < 100 bytes per dirty chunk overhead
   - Reuse 80%+ of mesh data on updates
   - Total dirty tracking < 1MB for 10,000 chunks

4. **Visual Quality**
   - Zero chunk boundary artifacts
   - No visible update delays < 100m
   - Smooth LOD transitions

5. **Scalability**
   - 10,000+ concurrent players
   - 144+ FPS maintained
   - Linear performance scaling

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_region_dirty_tracking() {
    let mut state = ChunkDirtyState::new(0);
    
    // Test single region
    state.mark_region(LocalVoxelPos::new(5, 5, 5));
    assert_eq!(state.regions.count_ones(), 1);
    assert_eq!(state.get_dirty_region_index(), Some(0));
    
    // Test boundary regions
    state.mark_region(LocalVoxelPos::new(49, 49, 49));
    assert_eq!(state.regions.count_ones(), 2);
}

#[test]
fn test_neighbor_notification() {
    let mut dirty_chunks = HashMap::new();
    
    // Test edge block
    mark_block_dirty(&mut dirty_chunks, VoxelPos::new(0, 25, 25), 0);
    
    assert_eq!(dirty_chunks.len(), 2); // Current + neighbor
    assert!(dirty_chunks.contains_key(&ChunkPos::new(0, 0, 0)));
    assert!(dirty_chunks.contains_key(&ChunkPos::new(-1, 0, 0)));
}
```

### Integration Tests
- Stress test with 10,000 random block edits
- Verify mesh consistency after updates
- Test priority queue under load
- Validate GPU update correctness

### Performance Benchmarks
```rust
#[bench]
fn bench_mark_single_block_dirty(b: &mut Bencher) {
    let mut dirty_chunks = HashMap::new();
    b.iter(|| {
        mark_block_dirty(&mut dirty_chunks, VoxelPos::new(100, 100, 100), 0);
    });
}

#[bench]
fn bench_incremental_mesh_generation(b: &mut Bencher) {
    let chunk_data = create_test_chunk();
    let mut dirty_state = ChunkDirtyState::new(0);
    dirty_state.mark_region(LocalVoxelPos::new(25, 25, 25));
    
    b.iter(|| {
        generate_mesh_incremental(&chunk_data, &dirty_state, None);
    });
}
```

## Migration Plan

### Phase 0: Preparation (Current)
1. Document existing system
2. Add metrics collection
3. Create feature flag `dirty-chunk-v2`

### Phase 1: Parallel Implementation
1. Implement new system alongside old
2. Add comparison tests
3. Gradual rollout with feature flag

### Phase 2: Migration
1. Switch default to new system
2. Deprecate old dirty flags
3. Remove legacy code after validation

### Phase 3: Optimization
1. Profile real-world usage
2. Tune parameters (region size, priorities)
3. Add advanced features (predictive marking)

## Risk Mitigation

### Technical Risks
1. **GPU Memory Pressure**
   - Mitigation: Dynamic buffer sizing
   - Fallback: CPU path for low-end GPUs

2. **Complexity Increase**
   - Mitigation: Comprehensive testing
   - Phased rollout with monitoring

3. **Platform Compatibility**
   - Mitigation: Feature detection
   - Graceful degradation

### Performance Risks
1. **Overhead > Savings**
   - Mitigation: Configurable granularity
   - Kill switch to disable regions

2. **Priority Queue Bottleneck**
   - Mitigation: Lock-free implementation
   - Parallel queue processing

## Future Enhancements

### Predictive Dirty Marking
- Analyze player behavior patterns
- Pre-mark likely edit locations
- Speculative mesh generation

### Hierarchical Dirty Tracking
- Multiple LOD levels of dirty regions
- Cascading updates for distant chunks
- Adaptive region sizes

### Machine Learning Integration
- Learn common edit patterns
- Optimize region boundaries
- Predict mesh complexity

## Conclusion

This dirty chunk system will provide a 10-100x performance improvement for typical gameplay scenarios while maintaining visual quality and supporting massive scale. The phased implementation ensures we can validate each component before moving to the next, minimizing risk while maximizing performance gains.

Remember: We're building for 10,000 players, not 10. Every millisecond counts.