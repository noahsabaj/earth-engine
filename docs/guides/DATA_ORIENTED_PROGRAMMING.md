# Data-Oriented Programming Guide

## Version 2.0 - Consolidated Guide
**Last Updated**: June 17, 2025  
**Status**: Active Enforcement

## Table of Contents
1. [Philosophy: NO OBJECTS, EVER](#philosophy-no-objects-ever)
2. [DOP vs OOP: Core Differences](#dop-vs-oop-core-differences)
3. [Approved DOP Patterns](#approved-dop-patterns)
4. [Forbidden OOP Patterns](#forbidden-oop-patterns)
5. [Code Review Checklist](#code-review-checklist)
6. [Enforcement Tools](#enforcement-tools)
7. [Migration Strategies](#migration-strategies)
8. [Performance Validation](#performance-validation)

## Philosophy: NO OBJECTS, EVER

Hearth Engine follows strict Data-Oriented Programming (DOP) principles:

> **❌ NO classes, objects, or OOP patterns**  
> **❌ NO methods - only functions that transform data**  
> **✅ Data lives in shared buffers (WorldBuffer, RenderBuffer, etc.)**  
> **✅ Systems are stateless kernels that read/write buffers**  
> **✅ GPU-first architecture - data lives where it's processed**  
> **✅ If you're writing `self.method()`, you're doing it wrong**

### Core Principle: Data + Kernels
- **Data**: Lives in buffers, organized for efficient access
- **Kernels**: Stateless functions that transform data
- **No Hidden State**: Everything is explicit and visible

## DOP vs OOP: Core Differences

### Object-Oriented (FORBIDDEN)
```rust
// ❌ NEVER DO THIS
pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self { /* ... */ }
    pub fn update(&mut self) { /* ... */ }
    pub fn render(&self, renderer: &Renderer) { /* ... */ }
}
```

### Data-Oriented (REQUIRED)
```rust
// ✅ ALWAYS DO THIS
pub struct MeshData {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<u32>,
}

pub fn create_mesh_data(vertex_count: usize) -> MeshData { /* ... */ }
pub fn update_mesh_positions(positions: &mut [Vec3], time: f32) { /* ... */ }
pub fn render_meshes(mesh_data: &MeshData, render_commands: &mut RenderCommands) { /* ... */ }
```

## Code Review Checklist

### ❌ IMMEDIATE REJECTION CRITERIA

If any PR contains these patterns, **REJECT IMMEDIATELY**:

1. **Methods with Self**
   ```rust
   // ❌ IMMEDIATE REJECTION
   impl SomeStruct {
       pub fn update(&mut self, data: &Data) { } // NO METHODS EVER
       pub fn process(&self) -> Result<()> { }   // NO METHODS EVER
   }
   ```

2. **Trait Objects (Dynamic Dispatch)**
   ```rust
   // ❌ IMMEDIATE REJECTION
   let renderable: Box<dyn Drawable> = Box::new(mesh);
   fn process(items: &[Box<dyn Processable>]) { }
   ```

3. **Builder Patterns**
   ```rust
   // ❌ IMMEDIATE REJECTION
   Mesh::builder()
       .with_vertices(vertices)
       .with_indices(indices)
       .build()
   ```

4. **Internal State Management**
   ```rust
   // ❌ IMMEDIATE REJECTION
   struct System {
       internal_cache: HashMap<Id, Data>,  // NO INTERNAL STATE
       update_count: usize,                // NO COUNTERS IN STRUCTS
   }
   ```

5. **Method Chaining**
   ```rust
   // ❌ IMMEDIATE REJECTION
   entity
       .set_position(pos)
       .set_velocity(vel)
       .update()
   ```

### ✅ REQUIRED PATTERNS

All code MUST follow these patterns:

1. **Pure Functions Only**
   ```rust
   // ✅ CORRECT - Pure function, no hidden state
   pub fn update_positions(
       positions: &mut [Vec3],
       velocities: &[Vec3],
       dt: f32
   ) {
       for (pos, vel) in positions.iter_mut().zip(velocities.iter()) {
           *pos += *vel * dt;
       }
   }
   ```

2. **Explicit Data Structures**
   ```rust
   // ✅ CORRECT - Plain data, no methods
   pub struct PhysicsData {
       pub positions: Vec<Vec3>,
       pub velocities: Vec<Vec3>,
       pub masses: Vec<f32>,
   }
   ```

3. **Stateless Systems**
   ```rust
   // ✅ CORRECT - System is just a namespace for functions
   pub mod physics_system {
       pub fn integrate(data: &mut PhysicsData, dt: f32) { }
       pub fn apply_forces(data: &mut PhysicsData, forces: &[Vec3]) { }
   }
   ```

4. **Buffer-Oriented Design**
   ```rust
   // ✅ CORRECT - Data in contiguous buffers
   pub struct WorldBuffers {
       pub chunk_data: Buffer,
       pub entity_transforms: Buffer,
       pub physics_state: Buffer,
   }
   ```

## Enforcement Tools

### Automated Detection

1. **Clippy Lints**
   ```toml
   # In .clippy.toml
   disallowed-methods = [
       "std::rc::Rc",
       "std::sync::Arc",
       "std::cell::RefCell",
   ]
   ```

2. **Custom Lints**
   ```rust
   // forbid_impl_blocks.rs
   if item.has_impl_block() && !item.is_trait_impl() {
       span_lint!(FORBIDDEN_IMPL, "impl blocks are forbidden");
   }
   ```

3. **Pre-commit Hooks**
   ```bash
   #!/bin/bash
   # Check for forbidden patterns
   if rg "impl.*\{" --type rust; then
       echo "ERROR: Found impl blocks"
       exit 1
   fi
   ```

### Manual Review Points

1. **Data Layout**
   - Is data organized for cache efficiency?
   - Are hot and cold data separated?
   - Is data accessed sequentially?

2. **Function Purity**
   - No hidden state?
   - No side effects beyond declared parameters?
   - Deterministic results?

3. **System Design**
   - Stateless transformation?
   - Clear input/output?
   - No temporal coupling?

## Migration Strategies

### Converting OOP to DOP

#### Step 1: Extract Data
```rust
// Before (OOP)
class Entity {
    position: Vec3,
    velocity: Vec3,
    fn update(&mut self, dt: f32) {
        self.position += self.velocity * dt;
    }
}

// After (DOP) - Step 1
struct EntityData {
    positions: Vec<Vec3>,
    velocities: Vec<Vec3>,
}
```

#### Step 2: Extract Methods as Functions
```rust
// After (DOP) - Step 2
fn update_entities(data: &mut EntityData, dt: f32) {
    for (pos, vel) in data.positions.iter_mut()
        .zip(data.velocities.iter()) {
        *pos += *vel * dt;
    }
}
```

#### Step 3: Optimize Data Layout
```rust
// After (DOP) - Step 3 (SoA layout)
struct EntityDataSoA {
    positions_x: Vec<f32>,
    positions_y: Vec<f32>,
    positions_z: Vec<f32>,
    velocities_x: Vec<f32>,
    velocities_y: Vec<f32>,
    velocities_z: Vec<f32>,
}
```

## Performance Validation

### Required Metrics

Every DOP conversion MUST show:

1. **Cache Performance**
   - L1 hit rate > 85%
   - L2 hit rate > 70%
   - Measure with `perf stat -e cache-misses`

2. **Memory Bandwidth**
   - Utilization > 80% of theoretical max
   - Sequential access patterns
   - Measure with `likwid-perfctr`

3. **Instruction Performance**
   - SIMD utilization > 70%
   - Branch prediction > 95%
   - No virtual calls

### Benchmark Requirements

```rust
#[bench]
fn bench_entity_update_dop(b: &mut Bencher) {
    let mut data = create_entity_data(10_000);
    b.iter(|| {
        update_entities(&mut data, 0.016);
    });
}
```

Expected improvements:
- 2-5x performance increase
- Linear scaling with core count
- Predictable memory usage

## Common Pitfalls

### Hidden OOP Patterns

1. **Fake DOP** - OOP with extra steps
   ```rust
   // ❌ Still OOP!
   fn update_mesh(mesh_id: usize, meshes: &mut Vec<Mesh>) {
       meshes[mesh_id].update(); // Still calling methods!
   }
   ```

2. **State Hiding**
   ```rust
   // ❌ Hidden state
   static mut COUNTER: usize = 0;
   fn process() {
       unsafe { COUNTER += 1; } // NO!
   }
   ```

3. **Encapsulation Theater**
   ```rust
   // ❌ Pretending to be DOP
   mod mesh {
       struct MeshInternal { } // Private state
       pub fn update() { }      // Hiding data
   }
   ```

## Best Practices

### Data Organization

1. **Hot/Cold Separation**
   ```rust
   // Frequently accessed together
   struct HotData {
       positions: Vec<Vec3>,
       velocities: Vec<Vec3>,
   }
   
   // Rarely accessed
   struct ColdData {
       names: Vec<String>,
       metadata: Vec<MetaInfo>,
   }
   ```

2. **Parallel-Friendly Layout**
   ```rust
   // Designed for parallel processing
   struct ChunkData {
       // Each chunk can be processed independently
       chunks: Vec<[u8; CHUNK_SIZE]>,
   }
   ```

3. **GPU-Ready Structures**
   ```rust
   #[repr(C)]
   struct GpuVertex {
       position: [f32; 3],
       normal: [f32; 3],
       uv: [f32; 2],
   }
   ```

## Conclusion

Data-Oriented Programming is not optional in Hearth Engine - it's mandatory. Every line of code must follow these principles. No exceptions, no compromises, no "just this once" moments.

Remember: **Objects are the enemy of performance.**