# Data-Oriented Programming Enforcement Guide

## Version 1.0 - Sprint 37: DOP Reality Check
**Last Updated**: June 13, 2025  
**Status**: Active Enforcement

## Table of Contents
1. [Philosophy: NO OBJECTS, EVER](#philosophy-no-objects-ever)
2. [DOP vs OOP: Core Differences](#dop-vs-oop-core-differences)
3. [Approved DOP Patterns](#approved-dop-patterns)
4. [Forbidden OOP Patterns](#forbidden-oop-patterns)
5. [Automated Detection Tools](#automated-detection-tools)
6. [Code Review Checklist](#code-review-checklist)
7. [Migration Strategies](#migration-strategies)
8. [Performance Validation](#performance-validation)
9. [Examples and Anti-Examples](#examples-and-anti-examples)
10. [Enforcement Tooling](#enforcement-tooling)

## Philosophy: NO OBJECTS, EVER

Earth Engine follows strict Data-Oriented Programming (DOP) principles as mandated by `CLAUDE.md`:

> **❌ NO classes, objects, or OOP patterns**  
> **❌ NO methods - only functions that transform data**  
> **✅ Data lives in shared buffers (WorldBuffer, RenderBuffer, etc.)**  
> **✅ Systems are stateless kernels that read/write buffers**  
> **✅ GPU-first architecture - data lives where it's processed**  
> **✅ If you're writing `self.method()`, you're doing it wrong**

### Core Principle: Data + Kernels

Every system in Earth Engine follows this pattern:
- **Data**: Stored in buffers (preferably GPU-accessible)
- **Kernels**: Pure functions that transform data
- **No State**: Functions don't own data, they operate on it

## DOP vs OOP: Core Differences

| Aspect | ❌ OOP (Forbidden) | ✅ DOP (Required) |
|--------|------------------|-----------------|
| **Data Storage** | `struct Player { fn move() }` | `PlayerData { positions: Vec<Vec3> }` |
| **Behavior** | `player.move()` | `move_players(&mut data, input)` |
| **Memory Layout** | Array of Structs (AoS) | Structure of Arrays (SoA) |
| **Processing** | One at a time | Batch/SIMD/GPU |
| **Cache** | Scattered access | Sequential access |
| **GPU** | Incompatible | Native |

### Why DOP Matters for Earth Engine

1. **Performance**: 10-100x faster due to cache efficiency
2. **GPU Compatibility**: Data lives where it's processed
3. **Scalability**: SIMD and GPU parallelization
4. **Memory Efficiency**: Better compression and streaming
5. **Determinism**: Pure functions enable predictable behavior

## Approved DOP Patterns

### ✅ Pattern 1: Structure of Arrays (SoA)

```rust
// ✅ CORRECT - Structure of Arrays
pub struct PlayerData {
    pub count: usize,
    pub positions_x: Vec<f32>,
    pub positions_y: Vec<f32>,
    pub positions_z: Vec<f32>,
    pub health: Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub velocities_z: Vec<f32>,
}

// ✅ CORRECT - Pure kernel functions
pub fn update_player_physics(
    data: &mut PlayerData,
    input: &InputData,
    dt: f32,
) {
    for i in 0..data.count {
        data.velocities_x[i] += input.movement_x[i] * dt;
        data.positions_x[i] += data.velocities_x[i] * dt;
    }
}
```

### ✅ Pattern 2: Buffer-Based Systems

```rust
// ✅ CORRECT - GPU-accessible buffers
pub struct RenderBuffer {
    pub vertex_positions: wgpu::Buffer,
    pub vertex_normals: wgpu::Buffer,
    pub instance_transforms: wgpu::Buffer,
    pub material_indices: wgpu::Buffer,
}

// ✅ CORRECT - Kernel operates on buffers
pub fn submit_render_data(
    render_buffer: &RenderBuffer,
    world_buffer: &WorldBuffer,
    render_pass: &mut wgpu::RenderPass,
) {
    // Direct GPU operations, no intermediate objects
    render_pass.set_vertex_buffer(0, render_buffer.vertex_positions.slice(..));
    render_pass.set_vertex_buffer(1, render_buffer.vertex_normals.slice(..));
    render_pass.draw_indexed(0..world_buffer.index_count, 0, 0..world_buffer.instance_count);
}
```

### ✅ Pattern 3: Stateless Kernel Functions

```rust
// ✅ CORRECT - Pure transformation functions
pub fn apply_chunk_generation(
    world_buffer: &mut WorldBuffer,
    noise_params: &NoiseParams,
    chunk_positions: &[ChunkPos],
) {
    for &pos in chunk_positions {
        let chunk_index = calculate_chunk_index(pos);
        generate_chunk_voxels(
            &mut world_buffer.voxels[chunk_index..],
            pos,
            noise_params,
        );
    }
}

// ✅ CORRECT - No state, just data transformation
pub fn calculate_lighting_fast(
    voxel_data: &[VoxelId],
    light_data: &mut [LightLevel],
    light_sources: &[LightSource],
) {
    // Pure function - same inputs always produce same outputs
}
```

### ✅ Pattern 4: Pre-Allocated Pools

```rust
// ✅ CORRECT - Fixed-size pools prevent allocation
pub struct ParticlePool {
    pub capacity: usize,
    pub active_count: usize,
    pub positions: Vec<Vec3>,    // Pre-allocated
    pub velocities: Vec<Vec3>,   // Pre-allocated
    pub lifetimes: Vec<f32>,     // Pre-allocated
}

impl ParticlePool {
    // ✅ ACCEPTABLE - Constructor only, no behavior methods
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            active_count: 0,
            positions: vec![Vec3::ZERO; capacity],
            velocities: vec![Vec3::ZERO; capacity],
            lifetimes: vec![0.0; capacity],
        }
    }
}

// ✅ CORRECT - External kernel function
pub fn spawn_particles(
    pool: &mut ParticlePool,
    spawn_data: &SpawnData,
) {
    let available = pool.capacity - pool.active_count;
    let spawn_count = spawn_data.count.min(available);
    
    for i in 0..spawn_count {
        let index = pool.active_count + i;
        pool.positions[index] = spawn_data.positions[i];
        pool.velocities[index] = spawn_data.velocities[i];
        pool.lifetimes[index] = spawn_data.lifetimes[i];
    }
    
    pool.active_count += spawn_count;
}
```

## Forbidden OOP Patterns

### ❌ Anti-Pattern 1: Array of Structs (AoS)

```rust
// ❌ WRONG - Array of Structs
pub struct Player {
    pub position: Vec3,
    pub health: f32,
    pub velocity: Vec3,
}

// ❌ WRONG - Methods on structs
impl Player {
    pub fn update(&mut self, input: &Input, dt: f32) {
        self.velocity += input.movement * dt;
        self.position += self.velocity * dt;
    }
}

// ❌ WRONG - Processing one-by-one
fn update_players(players: &mut Vec<Player>, inputs: &[Input], dt: f32) {
    for (player, input) in players.iter_mut().zip(inputs) {
        player.update(input, dt);  // ❌ Method call!
    }
}
```

**Why This Is Wrong:**
- Cache inefficient (position and health not used together)
- No SIMD vectorization possible
- GPU incompatible
- Memory fragmentation

### ❌ Anti-Pattern 2: Stateful Objects

```rust
// ❌ WRONG - Object with internal state
pub struct ChunkGenerator {
    noise: Perlin,
    cached_chunks: HashMap<ChunkPos, Chunk>,
    generation_queue: VecDeque<ChunkPos>,
}

impl ChunkGenerator {
    // ❌ WRONG - Stateful behavior
    pub fn generate_chunk(&mut self, pos: ChunkPos) -> Chunk {
        if let Some(cached) = self.cached_chunks.get(&pos) {
            return cached.clone();
        }
        
        let chunk = self.noise.generate_chunk(pos);
        self.cached_chunks.insert(pos, chunk.clone());
        chunk
    }
}
```

**Why This Is Wrong:**
- Hidden mutable state
- Not thread-safe
- Not deterministic
- GPU incompatible

### ❌ Anti-Pattern 3: Trait-Based Polymorphism

```rust
// ❌ WRONG - Trait-based behavior
pub trait Drawable {
    fn draw(&self, renderer: &mut Renderer);
    fn update(&mut self, dt: f32);
}

pub struct Mesh {
    vertices: Vec<Vertex>,
}

impl Drawable for Mesh {
    fn draw(&self, renderer: &mut Renderer) {
        renderer.draw_mesh(self);
    }
    
    fn update(&mut self, dt: f32) {
        // Update logic here
    }
}

// ❌ WRONG - Dynamic dispatch
fn render_objects(objects: &mut [Box<dyn Drawable>], renderer: &mut Renderer) {
    for obj in objects {
        obj.update(0.016);  // ❌ Virtual call!
        obj.draw(renderer); // ❌ Virtual call!
    }
}
```

**Why This Is Wrong:**
- Virtual function calls prevent inlining
- Dynamic dispatch prevents vectorization
- Runtime polymorphism instead of compile-time
- GPU incompatible

### ❌ Anti-Pattern 4: Builder Patterns

```rust
// ❌ WRONG - Builder pattern with fluent API
pub struct ParticleSystemBuilder {
    max_particles: Option<usize>,
    gravity: Option<f32>,
    wind_strength: Option<f32>,
}

impl ParticleSystemBuilder {
    pub fn new() -> Self { /* ... */ }
    
    pub fn max_particles(mut self, count: usize) -> Self {
        self.max_particles = Some(count);
        self
    }
    
    pub fn gravity(mut self, gravity: f32) -> Self {
        self.gravity = Some(gravity);
        self
    }
    
    pub fn build(self) -> ParticleSystem {
        // ❌ Creates object with methods
    }
}
```

**Why This Is Wrong:**
- Creates objects instead of data
- Unnecessary abstraction
- Runtime configuration instead of compile-time
- Encourages method-based design

## Automated Detection Tools

### Clippy Lints for DOP Enforcement

Create `.clippy.toml`:

```toml
# Enforce DOP patterns
warn-on-object-patterns = true
warn-on-method-calls = true
warn-on-trait-objects = true
```

### Custom Clippy Lints

```rust
// custom_lints/dop_enforcement.rs
declare_clippy_lint! {
    pub METHODS_ON_DATA_STRUCTS,
    restriction,
    "methods on data structures violate DOP principles"
}

impl<'tcx> LateLintPass<'tcx> for DopEnforcement {
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'tcx>) {
        if let ImplItemKind::Fn(..) = impl_item.kind {
            // Check if this is a method on a data struct
            if self.is_data_struct(cx, impl_item) {
                span_lint(
                    cx,
                    METHODS_ON_DATA_STRUCTS,
                    impl_item.span,
                    "methods on data structures are forbidden - use external kernel functions instead"
                );
            }
        }
    }
}
```

### Grep Patterns for CI/CD

```bash
# Check for forbidden patterns in CI
#!/bin/bash

echo "🔍 Checking for OOP anti-patterns..."

# Check for self method calls
if rg "\..*\(.*self.*\)" src --type rust; then
    echo "❌ Found method calls with self - convert to kernel functions"
    exit 1
fi

# Check for trait objects
if rg "Box<dyn.*>" src --type rust; then
    echo "❌ Found trait objects - use data-driven dispatch instead"
    exit 1
fi

# Check for impl blocks with behavior (exclude constructors)
if rg "impl.*\{[\s\S]*fn.*\(.*&.*self" src --type rust; then
    echo "❌ Found methods with self parameter - use external functions"
    exit 1
fi

echo "✅ No OOP anti-patterns detected"
```

### ripgrep Aliases

Add to `.bashrc` or `.zshrc`:

```bash
# DOP enforcement aliases
alias check-oop='rg "impl.*\{[\s\S]*fn.*\(.*&.*self" src --type rust'
alias check-methods='rg "\..*\(.*self.*\)" src --type rust'
alias check-traits='rg "Box<dyn.*>" src --type rust'
alias check-builders='rg "fn.*build\(self\)" src --type rust'
```

## Code Review Checklist

### ✅ DOP Compliance Checklist

Before approving any PR, verify:

#### Data Layout
- [ ] **SoA Layout**: All data structures use Structure of Arrays layout
- [ ] **No AoS**: No Array of Structs patterns
- [ ] **GPU Compatibility**: Data can be uploaded to GPU buffers
- [ ] **Cache Friendly**: Related data stored together

#### Function Design
- [ ] **Pure Functions**: All kernels are stateless and deterministic
- [ ] **No Methods**: No `fn method(&self)` or `fn method(&mut self)`
- [ ] **External Kernels**: Behavior implemented as external functions
- [ ] **Batch Processing**: Functions operate on arrays, not individual items

#### Memory Management
- [ ] **Pre-allocation**: Buffers allocated once, reused multiple times
- [ ] **No Runtime Allocation**: Hot paths avoid `Vec::push()` or `HashMap::insert()`
- [ ] **Pool-based**: Use object pools for temporary data
- [ ] **Fixed Sizes**: Prefer `[T; N]` over `Vec<T>` where possible

#### Performance
- [ ] **SIMD Friendly**: Functions can be vectorized
- [ ] **GPU Ready**: Data layout matches GPU compute shader expectations
- [ ] **Cache Efficient**: Sequential access patterns where possible
- [ ] **Branch Minimal**: Avoid conditionals in hot loops

### ❌ Red Flags - Immediate Rejection

- Any use of `self.method()` calls
- `impl` blocks with behavior methods (constructors OK)
- `Box<dyn Trait>` or other trait objects
- Builder patterns or fluent APIs
- Stateful objects or classes
- `Vec::push()` in hot paths
- Array of Structs layout

### ⚠️ Yellow Flags - Needs Justification

- `impl` blocks (must be constructors only)
- `HashMap` or `BTreeMap` (prefer flat arrays)
- Dynamic allocation (`Vec::new()`, `HashMap::new()`)
- Conditional logic in loops
- File I/O or network calls in kernels

## Migration Strategies

### Strategy 1: Gradual SoA Conversion

```rust
// Step 1: Original AoS structure
pub struct Player {
    pub position: Vec3,
    pub health: f32,
    pub velocity: Vec3,
}

// Step 2: Create SoA equivalent
pub struct PlayerData {
    pub count: usize,
    pub positions: Vec<Vec3>,
    pub health: Vec<f32>,
    pub velocities: Vec<Vec3>,
}

// Step 3: Conversion helpers (temporary)
impl PlayerData {
    pub fn from_aos(players: Vec<Player>) -> Self {
        let count = players.len();
        let mut data = Self {
            count,
            positions: Vec::with_capacity(count),
            health: Vec::with_capacity(count),
            velocities: Vec::with_capacity(count),
        };
        
        for player in players {
            data.positions.push(player.position);
            data.health.push(player.health);
            data.velocities.push(player.velocity);
        }
        
        data
    }
}

// Step 4: Remove AoS structure and conversion helpers
```

### Strategy 2: Method to Function Migration

```rust
// Before: Method-based
impl Chunk {
    pub fn generate(&mut self, noise: &Perlin) {
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let value = noise.sample(x, y, z);
                    self.set_voxel(x, y, z, value);
                }
            }
        }
    }
}

// After: Kernel function
pub fn generate_chunk(
    chunk_data: &mut ChunkData,
    chunk_pos: ChunkPos,
    noise_params: &NoiseParams,
) {
    let base_pos = chunk_pos.to_world_pos();
    
    for i in 0..CHUNK_SIZE_CUBED {
        let local_pos = index_to_local_pos(i);
        let world_pos = base_pos + local_pos;
        let noise_value = sample_noise(world_pos, noise_params);
        chunk_data.voxels[i] = voxel_from_noise(noise_value);
    }
}
```

### Strategy 3: Buffer Creation Guidelines

```rust
// Create buffers with known maximum sizes
pub fn create_particle_buffers(max_particles: usize) -> ParticleBuffers {
    ParticleBuffers {
        capacity: max_particles,
        active_count: 0,
        
        // Pre-allocate all buffers
        positions: vec![Vec3::ZERO; max_particles],
        velocities: vec![Vec3::ZERO; max_particles],
        ages: vec![0.0; max_particles],
        lifetimes: vec![1.0; max_particles],
    }
}

// Use atomic operations for thread-safe updates
pub fn allocate_particles(
    buffers: &ParticleBuffers,
    count: usize,
) -> Option<Range<usize>> {
    let current = buffers.active_count.load(Ordering::Acquire);
    let new_count = current + count;
    
    if new_count <= buffers.capacity {
        buffers.active_count.store(new_count, Ordering::Release);
        Some(current..new_count)
    } else {
        None // Pool exhausted
    }
}
```

## Performance Validation

### Benchmarking DOP vs OOP

```rust
// benches/dop_vs_oop.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_oop_player_update(c: &mut Criterion) {
    let mut players: Vec<Player> = (0..10000)
        .map(|_| Player::new())
        .collect();
    
    c.bench_function("oop_player_update", |b| {
        b.iter(|| {
            for player in &mut players {
                player.update(0.016); // ❌ Method call
            }
        })
    });
}

fn bench_dop_player_update(c: &mut Criterion) {
    let mut player_data = PlayerData::new(10000);
    
    c.bench_function("dop_player_update", |b| {
        b.iter(|| {
            update_players(&mut player_data, 0.016); // ✅ Kernel function
        })
    });
}

criterion_group!(benches, bench_oop_player_update, bench_dop_player_update);
criterion_main!(benches);
```

### Cache Performance Metrics

```rust
// Profile cache misses
use std::arch::x86_64::*;

pub fn measure_cache_efficiency<F>(f: F) -> CacheStats
where
    F: FnOnce(),
{
    let start_cycles = unsafe { _rdtsc() };
    
    // Enable performance counters
    let perf_fd = perf_event_open(PERF_COUNT_HW_CACHE_MISSES);
    
    f(); // Execute the function
    
    let end_cycles = unsafe { _rdtsc() };
    let cache_misses = read_counter(perf_fd);
    
    CacheStats {
        cycles: end_cycles - start_cycles,
        cache_misses,
        efficiency: 1.0 - (cache_misses as f64 / (end_cycles - start_cycles) as f64),
    }
}
```

### Memory Bandwidth Testing

```rust
pub fn measure_memory_bandwidth() {
    const DATA_SIZE: usize = 1024 * 1024 * 64; // 64MB
    
    // SoA layout (DOP)
    let mut positions_x = vec![0.0f32; DATA_SIZE];
    let mut positions_y = vec![0.0f32; DATA_SIZE];
    let mut positions_z = vec![0.0f32; DATA_SIZE];
    
    let start = std::time::Instant::now();
    
    // Sequential access (cache-friendly)
    for i in 0..DATA_SIZE {
        positions_x[i] = positions_x[i] + 1.0;
    }
    for i in 0..DATA_SIZE {
        positions_y[i] = positions_y[i] + 1.0;
    }
    for i in 0..DATA_SIZE {
        positions_z[i] = positions_z[i] + 1.0;
    }
    
    let soa_time = start.elapsed();
    
    // AoS layout (OOP) - for comparison
    #[repr(C)]
    struct Position { x: f32, y: f32, z: f32 }
    let mut positions = vec![Position { x: 0.0, y: 0.0, z: 0.0 }; DATA_SIZE];
    
    let start = std::time::Instant::now();
    
    // Strided access (cache-hostile)
    for pos in &mut positions {
        pos.x += 1.0;
        pos.y += 1.0;
        pos.z += 1.0;
    }
    
    let aos_time = start.elapsed();
    
    println!("SoA (DOP): {:?}", soa_time);
    println!("AoS (OOP): {:?}", aos_time);
    println!("Speedup: {:.2}x", aos_time.as_nanos() as f64 / soa_time.as_nanos() as f64);
}
```

## Examples and Anti-Examples

### Example 1: Particle System

#### ❌ OOP Version (Forbidden)

```rust
pub struct Particle {
    position: Vec3,
    velocity: Vec3,
    age: f32,
    lifetime: f32,
}

impl Particle {
    pub fn update(&mut self, dt: f32) {
        self.velocity.y -= 9.81 * dt; // Gravity
        self.position += self.velocity * dt;
        self.age += dt;
    }
    
    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn update(&mut self, dt: f32) {
        // ❌ One-by-one processing
        for particle in &mut self.particles {
            particle.update(dt);
        }
        
        // ❌ Expensive filtering
        self.particles.retain(|p| p.is_alive());
    }
}
```

#### ✅ DOP Version (Required)

```rust
pub struct ParticleData {
    pub active_count: usize,
    pub positions_x: Vec<f32>,
    pub positions_y: Vec<f32>,
    pub positions_z: Vec<f32>,
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub velocities_z: Vec<f32>,
    pub ages: Vec<f32>,
    pub lifetimes: Vec<f32>,
}

// ✅ Pure kernel functions
pub fn apply_gravity(data: &mut ParticleData, dt: f32) {
    let gravity_accel = -9.81 * dt;
    for i in 0..data.active_count {
        data.velocities_y[i] += gravity_accel;
    }
}

pub fn integrate_motion(data: &mut ParticleData, dt: f32) {
    for i in 0..data.active_count {
        data.positions_x[i] += data.velocities_x[i] * dt;
        data.positions_y[i] += data.velocities_y[i] * dt;
        data.positions_z[i] += data.velocities_z[i] * dt;
    }
}

pub fn update_ages(data: &mut ParticleData, dt: f32) {
    for i in 0..data.active_count {
        data.ages[i] += dt;
    }
}

pub fn remove_dead_particles(data: &mut ParticleData) {
    let mut write_index = 0;
    
    for read_index in 0..data.active_count {
        if data.ages[read_index] < data.lifetimes[read_index] {
            if write_index != read_index {
                // Move live particle to write position
                data.positions_x[write_index] = data.positions_x[read_index];
                data.positions_y[write_index] = data.positions_y[read_index];
                data.positions_z[write_index] = data.positions_z[read_index];
                data.velocities_x[write_index] = data.velocities_x[read_index];
                data.velocities_y[write_index] = data.velocities_y[read_index];
                data.velocities_z[write_index] = data.velocities_z[read_index];
                data.ages[write_index] = data.ages[read_index];
                data.lifetimes[write_index] = data.lifetimes[read_index];
            }
            write_index += 1;
        }
    }
    
    data.active_count = write_index;
}

// ✅ Batch update function
pub fn update_particles(data: &mut ParticleData, dt: f32) {
    apply_gravity(data, dt);
    integrate_motion(data, dt);
    update_ages(data, dt);
    remove_dead_particles(data);
}
```

### Example 2: Rendering System

#### ❌ OOP Version (Forbidden)

```rust
pub trait Renderable {
    fn render(&self, renderer: &mut Renderer);
    fn update(&mut self, dt: f32);
}

pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    transform: Mat4,
}

impl Renderable for Mesh {
    fn render(&self, renderer: &mut Renderer) {
        renderer.set_transform(self.transform);
        renderer.draw_mesh(&self.vertices, &self.indices);
    }
    
    fn update(&mut self, dt: f32) {
        // Update transform
    }
}

pub struct RenderSystem {
    objects: Vec<Box<dyn Renderable>>,
}

impl RenderSystem {
    pub fn render(&mut self, renderer: &mut Renderer) {
        // ❌ Virtual function calls
        for obj in &mut self.objects {
            obj.update(0.016);
            obj.render(renderer);
        }
    }
}
```

#### ✅ DOP Version (Required)

```rust
pub struct RenderData {
    pub mesh_count: usize,
    pub vertex_buffers: Vec<wgpu::Buffer>,
    pub index_buffers: Vec<wgpu::Buffer>,
    pub transform_data: Vec<Mat4>,
    pub material_indices: Vec<u32>,
}

pub struct InstanceData {
    pub count: usize,
    pub transforms: Vec<Mat4>,
    pub mesh_indices: Vec<u32>,
    pub material_indices: Vec<u32>,
}

// ✅ Batch rendering kernel
pub fn submit_render_batches(
    render_data: &RenderData,
    instance_data: &InstanceData,
    render_pass: &mut wgpu::RenderPass,
) {
    // Group instances by mesh for batching
    let mut current_mesh = u32::MAX;
    let mut batch_start = 0;
    
    for i in 0..instance_data.count {
        let mesh_index = instance_data.mesh_indices[i];
        
        if mesh_index != current_mesh {
            // Submit previous batch
            if current_mesh != u32::MAX {
                submit_mesh_batch(
                    render_data,
                    current_mesh,
                    &instance_data.transforms[batch_start..i],
                    render_pass,
                );
            }
            
            current_mesh = mesh_index;
            batch_start = i;
        }
    }
    
    // Submit final batch
    if current_mesh != u32::MAX {
        submit_mesh_batch(
            render_data,
            current_mesh,
            &instance_data.transforms[batch_start..],
            render_pass,
        );
    }
}

// ✅ Batched GPU submission
pub fn submit_mesh_batch(
    render_data: &RenderData,
    mesh_index: u32,
    transforms: &[Mat4],
    render_pass: &mut wgpu::RenderPass,
) {
    let mesh_idx = mesh_index as usize;
    
    render_pass.set_vertex_buffer(0, render_data.vertex_buffers[mesh_idx].slice(..));
    render_pass.set_index_buffer(render_data.index_buffers[mesh_idx].slice(..), wgpu::IndexFormat::Uint32);
    
    // Upload instance transforms to GPU
    // render_pass.set_vertex_buffer(1, instance_transform_buffer);
    
    render_pass.draw_indexed(0..get_index_count(mesh_idx), 0, 0..transforms.len() as u32);
}
```

## Enforcement Tooling

### Build Script Integration

Create `build.rs`:

```rust
// build.rs
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/");
    
    // Run DOP compliance checks
    let output = Command::new("scripts/check_dop_compliance.sh")
        .output()
        .expect("Failed to run DOP compliance check");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("DOP compliance check failed:\n{}", stderr);
    }
    
    println!("✅ DOP compliance verified");
}
```

### Pre-commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
set -e

echo "🔍 Running DOP compliance checks..."

# Check for OOP anti-patterns
./scripts/check_dop_compliance.sh

# Check for performance regressions
./scripts/run_performance_tests.sh

# Verify code compiles with DOP lints
cargo clippy -- -D warnings -D clippy::methods_on_data_structs

echo "✅ All DOP compliance checks passed"
```

### GitHub Actions Workflow

Create `.github/workflows/dop_enforcement.yml`:

```yaml
name: DOP Enforcement

on: [push, pull_request]

jobs:
  dop-compliance:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        components: clippy
    
    - name: Check DOP Compliance
      run: |
        chmod +x scripts/check_dop_compliance.sh
        ./scripts/check_dop_compliance.sh
    
    - name: Run Custom Clippy Lints
      run: cargo clippy -- -D clippy::methods_on_data_structs -D clippy::trait_objects
    
    - name: Performance Regression Test
      run: |
        cargo bench --bench dop_vs_oop
        # Fail if DOP performance is worse than baseline
```

### VSCode Extension Settings

Create `.vscode/settings.json`:

```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.checkOnSave.extraArgs": [
        "--", 
        "-D", "clippy::methods_on_data_structs",
        "-D", "clippy::trait_objects",
        "-W", "clippy::large_types_passed_by_value"
    ],
    "rust-analyzer.diagnostics.disabled": [],
    "rust-analyzer.diagnostics.warningsAsHint": [
        "dead_code"
    ],
    "files.watcherExclude": {
        "**/target/**": true
    }
}
```

## Maintenance and Evolution

### Regular Audits

Schedule monthly DOP compliance audits:

1. **Pattern Detection**: Run automated tools to find violations
2. **Performance Testing**: Benchmark DOP vs OOP patterns
3. **Code Review Training**: Update team on new patterns
4. **Tool Updates**: Enhance detection capabilities

### Metrics Tracking

Track DOP adoption metrics:

```rust
// metrics/dop_compliance.rs
pub struct DopMetrics {
    pub total_structs: usize,
    pub dop_structs: usize,
    pub oop_structs: usize,
    pub methods_count: usize,
    pub kernel_functions: usize,
    pub cache_efficiency: f64,
    pub gpu_buffer_usage: f64,
}

pub fn generate_dop_report() -> DopMetrics {
    // Analyze codebase and generate metrics
}
```

### Performance Baselines

Maintain performance baselines for DOP patterns:

```rust
// Baseline: Particle system should process 100k particles in <1ms
#[bench]
fn bench_particle_update_baseline(b: &mut Bencher) {
    let mut particles = ParticleData::new(100_000);
    
    b.iter(|| {
        update_particles(&mut particles, 0.016);
    });
    
    // Assert performance requirement
    assert!(b.elapsed().as_millis() < 1);
}
```

## Summary

This guide establishes the foundation for maintaining Earth Engine's data-oriented architecture. By following these patterns and using the provided tools, we ensure:

1. **Consistent Performance**: All code follows cache-friendly patterns
2. **GPU Compatibility**: Data layouts work seamlessly with compute shaders
3. **Maintainability**: Pure functions are easier to test and debug
4. **Scalability**: SIMD and parallel processing comes naturally

Remember: **NO OBJECTS, EVER**. If you're writing `self.method()`, you're doing it wrong.

The future of game engines is data-oriented. Earth Engine leads that future.