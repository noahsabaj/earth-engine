# Physics Data Layout Documentation

## Overview

The data-oriented physics system uses Struct-of-Arrays (SoA) layout to maximize cache efficiency and enable parallel processing. This document describes the data structures and access patterns.

## Core Design Principles

1. **Data over Objects**: No PhysicsBody objects - just arrays of data
2. **Cache Efficiency**: Sequential memory access for better performance
3. **Parallelism First**: All operations designed for parallel execution
4. **GPU Ready**: Data layout compatible with GPU compute shaders

## Data Structures

### PhysicsData (SoA Layout)

The main physics storage using separate arrays for each attribute:

```rust
pub struct PhysicsData {
    // Transform arrays
    pub positions: Vec<[f32; 3]>,         // World position
    pub velocities: Vec<[f32; 3]>,        // Linear velocity
    pub rotations: Vec<[f32; 4]>,         // Quaternion rotation
    pub angular_velocities: Vec<[f32; 3]>, // Angular velocity
    
    // Physical properties
    pub masses: Vec<f32>,                 // Mass in kg
    pub inverse_masses: Vec<f32>,         // 1/mass (pre-computed)
    pub restitutions: Vec<f32>,           // Bounciness (0-1)
    pub frictions: Vec<f32>,              // Friction coefficient
    
    // Collision data
    pub bounding_boxes: Vec<AABB>,        // Axis-aligned bounds
    pub collision_groups: Vec<u32>,       // What group am I in?
    pub collision_masks: Vec<u32>,        // What groups do I collide with?
    
    // Status flags
    pub flags: Vec<PhysicsFlags>,         // Active, static, sleeping, etc.
}
```

### Memory Layout Comparison

#### Array-of-Structs (Traditional)
```
Entity0: [pos|vel|mass|flags|...]
Entity1: [pos|vel|mass|flags|...]
Entity2: [pos|vel|mass|flags|...]
```
- Poor cache utilization when accessing single property
- Cache line loads unnecessary data

#### Struct-of-Arrays (Our Approach)
```
Positions:  [pos0|pos1|pos2|...]
Velocities: [vel0|vel1|vel2|...]
Masses:     [mass0|mass1|mass2|...]
```
- Excellent cache utilization for property-specific operations
- Sequential memory access patterns
- SIMD-friendly layout

## Collision Data Tables

Collisions are stored as tuples rather than objects:

```rust
pub struct CollisionData {
    // Collision pairs
    pub contact_pairs: Vec<ContactPair>,      // (EntityA, EntityB)
    pub contact_points: Vec<ContactPoint>,    // Position, normal, depth
    pub contact_counts: Vec<u32>,             // Contacts per pair
    
    // Solver data
    pub normal_impulses: Vec<f32>,            // Impulse cache
    pub tangent_impulses: Vec<[f32; 2]>,      // Friction impulses
    
    // Material properties
    pub combined_restitutions: Vec<f32>,      // Average bounciness
    pub combined_frictions: Vec<f32>,         // Average friction
}
```

## Spatial Hash Structure

The spatial hash divides the world into a 3D grid for efficient collision detection:

```rust
pub struct SpatialHash {
    cells: HashMap<CellCoord, Vec<EntityId>>,    // Grid cells
    entity_cells: HashMap<EntityId, Vec<CellCoord>>, // Reverse lookup
}
```

### Cell Size Optimization

Optimal cell size depends on entity density:
- **Small cells**: More cells to check, but fewer entities per cell
- **Large cells**: Fewer cells to check, but more entities per cell
- **Rule of thumb**: Cell size = 2-4x average entity size

## Access Patterns

### Sequential Access (Cache-Friendly)

```rust
// Integrate all positions - sequential memory access
for i in 0..count {
    positions[i] += velocities[i] * dt;
}
```

### Parallel Access (Thread-Friendly)

```rust
// Process in parallel chunks
positions.par_chunks_mut(64)
    .zip(velocities.par_chunks(64))
    .for_each(|(pos_chunk, vel_chunk)| {
        for (pos, vel) in pos_chunk.iter_mut().zip(vel_chunk) {
            *pos += *vel * dt;
        }
    });
```

## Performance Characteristics

### Cache Efficiency

| Operation | AoS Efficiency | SoA Efficiency |
|-----------|----------------|----------------|
| Update positions only | ~25% | ~100% |
| Update velocities only | ~25% | ~100% |
| Update all properties | ~100% | ~100% |

### Memory Usage

For 10,000 entities:
- **SoA Layout**: ~1.2 MB (tightly packed)
- **AoS Layout**: ~2.4 MB (with padding/alignment)
- **Savings**: ~50% memory reduction

### Parallel Scalability

- Broad phase: O(n) with parallel spatial hash
- Narrow phase: O(k) where k = collision pairs
- Integration: O(n) perfectly parallel
- Memory bandwidth limited, not compute limited

## GPU Compatibility

The data layout is designed for GPU compute shaders:

```wgsl
struct PhysicsData {
    positions: array<vec3<f32>>,
    velocities: array<vec3<f32>>,
    masses: array<f32>,
    flags: array<u32>,
}

@compute @workgroup_size(64)
fn integrate_positions(
    @builtin(global_invocation_id) id: vec3<u32>
) {
    let idx = id.x;
    if (flags[idx] & DYNAMIC_FLAG != 0) {
        positions[idx] += velocities[idx] * dt;
    }
}
```

## Best Practices

1. **Batch Operations**: Process multiple entities together
2. **Avoid Random Access**: Use spatial structures for queries
3. **Prefetch Data**: Warm up cache lines before use
4. **Align Data**: Ensure arrays are aligned to cache lines
5. **Use Atomics Sparingly**: Prefer parallel reduction patterns

## Migration Guide

Converting from object-oriented to data-oriented:

```rust
// Old way
for body in &mut physics_bodies {
    body.position += body.velocity * dt;
}

// New way
for i in 0..count {
    positions[i] += velocities[i] * dt;
}
```

## Future Optimizations

1. **SIMD Operations**: Use explicit SIMD for vector math
2. **GPU Compute**: Move integration to GPU
3. **Compression**: Pack flags into bits
4. **Quantization**: Use fixed-point for some properties
5. **Temporal Coherence**: Exploit frame-to-frame similarity

## Debugging Tips

1. **Validate Indices**: Always check entity indices are in bounds
2. **Track Array Sizes**: Ensure all arrays have same length
3. **Monitor Cache Misses**: Use profiler to track efficiency
4. **Verify Alignment**: Check data is properly aligned
5. **Test Parallel Safety**: Run with thread sanitizer