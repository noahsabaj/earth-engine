# Structure-of-Arrays (SOA) Guidelines for Earth Engine

## Overview

This document establishes the guidelines for implementing Structure-of-Arrays patterns in Earth Engine to achieve optimal cache efficiency and performance as part of Sprint 37: DOP Reality Check.

## Core Principles

### 1. No Methods on Data Structures
```rust
// ❌ WRONG - AOS with methods
struct Particle {
    position: Vec3,
    velocity: Vec3,
    lifetime: f32,
}
impl Particle {
    fn update(&mut self, dt: f32) { /* ... */ }
}

// ✅ CORRECT - SOA with pure functions
struct ParticleData {
    positions_x: Vec<f32>,
    positions_y: Vec<f32>,
    positions_z: Vec<f32>,
    velocities_x: Vec<f32>,
    velocities_y: Vec<f32>,
    velocities_z: Vec<f32>,
    lifetimes: Vec<f32>,
}

fn update_particles(particles: &mut ParticleData, dt: f32) {
    // Cache-friendly iteration over separate arrays
}
```

### 2. Separate Arrays for Each Attribute
- Position components: `positions_x`, `positions_y`, `positions_z`
- Velocity components: `velocities_x`, `velocities_y`, `velocities_z`
- Scalar values: `masses`, `lifetimes`, `sizes`

### 3. Cache-Aligned Memory Layout
```rust
use std::alloc::{alloc_zeroed, Layout};

const CACHE_LINE_SIZE: usize = 64;

struct AlignedArray<T> {
    ptr: *mut T,
    len: usize,
    layout: Layout,
}

impl<T: Copy + Default> AlignedArray<T> {
    fn new(len: usize) -> Self {
        let size = len * std::mem::size_of::<T>();
        let align = CACHE_LINE_SIZE.max(std::mem::align_of::<T>());
        let layout = Layout::from_size_align(size, align).expect("Invalid layout");
        
        unsafe {
            let ptr = alloc_zeroed(layout) as *mut T;
            Self { ptr, len, layout }
        }
    }
}
```

### 4. GPU-Ready Data Layout
```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuParticleData {
    position: [f32; 3],
    size: f32,
    color: [f32; 4],
    // Aligned to 16-byte boundaries
}

// Convert SOA to GPU format efficiently
fn prepare_gpu_data(particles: &ParticleData) -> Vec<GpuParticleData> {
    (0..particles.count)
        .map(|i| GpuParticleData {
            position: [
                particles.positions_x[i],
                particles.positions_y[i],
                particles.positions_z[i],
            ],
            size: particles.sizes[i],
            color: [
                particles.colors_r[i],
                particles.colors_g[i],
                particles.colors_b[i],
                particles.colors_a[i],
            ],
        })
        .collect()
}
```

## Implementation Patterns

### Pattern 1: Component-wise Operations
```rust
// Process all X components together
for i in 0..count {
    positions_x[i] += velocities_x[i] * dt;
}
// Process all Y components together  
for i in 0..count {
    positions_y[i] += velocities_y[i] * dt;
}
// Process all Z components together
for i in 0..count {
    positions_z[i] += velocities_z[i] * dt;
}
```

### Pattern 2: SIMD-Friendly Batches
```rust
// Process in chunks of 4 for SIMD optimization
const SIMD_WIDTH: usize = 4;

fn update_positions_simd(
    positions_x: &mut [f32],
    velocities_x: &[f32],
    dt: f32
) {
    for chunk in positions_x.chunks_exact_mut(SIMD_WIDTH) {
        // SIMD operations here
        for (pos, vel) in chunk.iter_mut().zip(velocities_x.iter()) {
            *pos += vel * dt;
        }
    }
}
```

### Pattern 3: Parallel Processing
```rust
use rayon::prelude::*;

fn update_physics_parallel(physics: &mut PhysicsSoA, dt: f32) {
    let count = physics.len();
    
    // Update velocities in parallel
    physics.velocities_x
        .par_iter_mut()
        .zip(physics.accelerations_x.par_iter())
        .take(count)
        .for_each(|(vel, acc)| *vel += acc * dt);
        
    physics.velocities_y
        .par_iter_mut()
        .zip(physics.accelerations_y.par_iter())
        .take(count)
        .for_each(|(vel, acc)| *vel += acc * dt);
        
    physics.velocities_z
        .par_iter_mut()
        .zip(physics.accelerations_z.par_iter())
        .take(count)
        .for_each(|(vel, acc)| *vel += acc * dt);
}
```

### Pattern 4: Batch Component Management
```rust
/// Add multiple components efficiently
fn add_transform_components_batch(
    transforms: &mut TransformSoA,
    entities: &[EntityId],
    positions: &[[f32; 3]],
    rotations: &[[f32; 3]],
    scales: &[[f32; 3]],
) {
    let count = entities.len();
    let start_idx = transforms.len();
    
    // Reserve space
    transforms.positions_x.reserve(count);
    transforms.positions_y.reserve(count);
    transforms.positions_z.reserve(count);
    // ... etc for all arrays
    
    // Batch append
    for i in 0..count {
        transforms.positions_x.push(positions[i][0]);
        transforms.positions_y.push(positions[i][1]);
        transforms.positions_z.push(positions[i][2]);
        // ... etc
    }
    
    // Update mappings
    for (i, &entity) in entities.iter().enumerate() {
        transforms.entity_to_component[entity.idx()] = Some((start_idx + i) as u32);
        transforms.entities.push(entity);
    }
    
    transforms.count.fetch_add(count as u32, Ordering::AcqRel);
}
```

## Migration Strategy

### Phase 1: Identify AOS Patterns
1. Search for structs with position/velocity fields
2. Find impl blocks with update methods
3. Locate entity storage patterns
4. Identify hot-path performance code

### Phase 2: Create SOA Equivalent
1. Split struct fields into separate arrays
2. Add entity mapping arrays
3. Implement cache alignment
4. Add GPU buffer support

### Phase 3: Convert Methods to Functions
1. Remove impl blocks
2. Create pure functions operating on SOA data
3. Pass references to arrays, not self
4. Batch operations where possible

### Phase 4: Update Usage Sites
1. Replace struct creation with SOA operations
2. Update system calls to use pure functions
3. Batch component additions/removals
4. Optimize iteration patterns

## Performance Targets

### Cache Efficiency Improvements
- **Single-attribute access**: 70% → 95% cache hit rate
- **Position-only operations**: 100% cache efficiency (all positions contiguous)
- **Component iteration**: 80-90% cache efficiency

### Memory Bandwidth Reduction
- **GPU transfers**: 30-50% less bandwidth due to better packing
- **Cross-component operations**: Eliminate cache line pollution
- **Hot path operations**: 2-5x performance improvement

### Allocation Reduction
- **Frame allocations**: 268 → <10 per frame
- **Component pools**: Pre-allocated, zero runtime allocation
- **Temporary objects**: Eliminated through direct array operations

## Testing and Verification

### Cache Profiling
```bash
# Use valgrind to profile cache behavior
valgrind --tool=cachegrind --cachegrind-out-file=profile.out ./target/debug/earth-engine

# Analyze cache misses
cg_annotate profile.out

# Look for improvements in:
# - L1 data cache hit rate
# - LLC (Last Level Cache) hit rate
# - Memory bandwidth usage
```

### Performance Benchmarks
```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{criterion_group, criterion_main, Criterion};
    
    fn bench_aos_vs_soa(c: &mut Criterion) {
        let mut group = c.benchmark_group("particle_update");
        
        // Benchmark AOS version
        group.bench_function("aos", |b| {
            b.iter(|| update_particles_aos(&mut particles_aos, 0.016))
        });
        
        // Benchmark SOA version
        group.bench_function("soa", |b| {
            b.iter(|| update_particles_soa(&mut particles_soa, 0.016))
        });
        
        group.finish();
    }
    
    criterion_group!(benches, bench_aos_vs_soa);
    criterion_main!(benches);
}
```

### Memory Usage Analysis
```rust
fn analyze_memory_layout() {
    let soa = ParticleDataSoA::new(10000);
    
    // Check alignment
    assert_eq!(soa.positions_x.as_ptr() as usize % 64, 0);
    assert_eq!(soa.positions_y.as_ptr() as usize % 64, 0);
    
    // Check memory locality
    let pos_x_ptr = soa.positions_x.as_ptr();
    let pos_y_ptr = soa.positions_y.as_ptr();
    
    // Arrays should be contiguous in memory
    println!("Memory layout analysis:");
    println!("positions_x: {:p}", pos_x_ptr);
    println!("positions_y: {:p}", pos_y_ptr);
}
```

## Common Pitfalls and Solutions

### Pitfall 1: Index Out of Bounds
```rust
// ❌ WRONG - Can panic
fn get_position(transforms: &TransformSoA, index: usize) -> [f32; 3] {
    [
        transforms.positions_x[index],
        transforms.positions_y[index], 
        transforms.positions_z[index],
    ]
}

// ✅ CORRECT - Bounds checking
fn get_position_safe(transforms: &TransformSoA, index: usize) -> Option<[f32; 3]> {
    if index < transforms.len() {
        Some([
            transforms.positions_x[index],
            transforms.positions_y[index],
            transforms.positions_z[index],
        ])
    } else {
        None
    }
}
```

### Pitfall 2: Inconsistent Array Lengths
```rust
// ✅ SOLUTION - Always maintain consistent lengths
impl TransformSoA {
    fn add_component(&mut self, pos: [f32; 3], rot: [f32; 3], scale: [f32; 3]) {
        // Add to ALL arrays or NONE
        self.positions_x.push(pos[0]);
        self.positions_y.push(pos[1]);
        self.positions_z.push(pos[2]);
        self.rotations_x.push(rot[0]);
        self.rotations_y.push(rot[1]);
        self.rotations_z.push(rot[2]);
        self.scales_x.push(scale[0]);
        self.scales_y.push(scale[1]);
        self.scales_z.push(scale[2]);
        
        self.count.fetch_add(1, Ordering::AcqRel);
    }
    
    fn invariant_check(&self) {
        let count = self.len();
        assert_eq!(self.positions_x.len(), count);
        assert_eq!(self.positions_y.len(), count);
        assert_eq!(self.positions_z.len(), count);
        // ... etc for all arrays
    }
}
```

### Pitfall 3: Poor Cache Usage Patterns
```rust
// ❌ WRONG - Poor cache usage
fn update_entities_bad(transforms: &mut TransformSoA, physics: &mut PhysicsSoA) {
    for i in 0..transforms.len() {
        // Accessing different cache lines each iteration
        transforms.positions_x[i] += physics.velocities_x[i];
        transforms.positions_y[i] += physics.velocities_y[i];
        transforms.positions_z[i] += physics.velocities_z[i];
    }
}

// ✅ CORRECT - Cache-friendly pattern
fn update_entities_good(transforms: &mut TransformSoA, physics: &PhysicsSoA) {
    let count = transforms.len();
    
    // Process each component separately for better cache usage
    for i in 0..count {
        transforms.positions_x[i] += physics.velocities_x[i];
    }
    for i in 0..count {
        transforms.positions_y[i] += physics.velocities_y[i];
    }
    for i in 0..count {
        transforms.positions_z[i] += physics.velocities_z[i];
    }
}
```

## Integration with Existing Systems

### GPU Integration
```rust
impl TransformSoA {
    fn create_gpu_buffers(&self, device: &wgpu::Device) -> TransformGpuBuffers {
        // Create separate buffers for each component
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transform Positions"),
            contents: bytemuck::cast_slice(&[
                &self.positions_x[..self.len()],
                &self.positions_y[..self.len()],
                &self.positions_z[..self.len()],
            ].concat()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        TransformGpuBuffers { position_buffer }
    }
}
```

### Networking Integration
```rust
fn serialize_transforms_delta(
    old: &TransformSoA,
    new: &TransformSoA,
    changed_mask: &[bool]
) -> Vec<u8> {
    let mut buffer = Vec::new();
    
    for (i, &changed) in changed_mask.iter().enumerate() {
        if changed && i < new.len() {
            // Serialize only changed positions
            buffer.extend_from_slice(&new.positions_x[i].to_le_bytes());
            buffer.extend_from_slice(&new.positions_y[i].to_le_bytes());
            buffer.extend_from_slice(&new.positions_z[i].to_le_bytes());
        }
    }
    
    buffer
}
```

## Success Criteria

By the end of Sprint 37, we should achieve:

1. **✅ 80%+ SOA Coverage**: Core systems converted to SOA patterns
2. **✅ >90% Cache Efficiency**: For single-attribute operations  
3. **✅ 30%+ Memory Bandwidth Reduction**: Due to better data layout
4. **✅ <10 Frame Allocations**: Down from 268 per frame
5. **✅ Zero Compilation Errors**: All SOA systems compile cleanly
6. **✅ Performance Benchmarks**: Documented improvements with evidence

This document serves as the definitive guide for implementing data-oriented, cache-efficient SOA patterns throughout Earth Engine.