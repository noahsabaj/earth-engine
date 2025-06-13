# DOP Code Review Checklist

## Version 1.0 - Sprint 37: DOP Reality Check
**Last Updated**: June 13, 2025  
**Purpose**: Ensure all code reviews enforce Earth Engine's data-oriented programming standards

## Critical Review Standards

### ❌ IMMEDIATE REJECTION CRITERIA

If any PR contains these patterns, **REJECT IMMEDIATELY** with no further review needed:

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
   let system = SystemBuilder::new()
       .with_capacity(1000)
       .build();
   ```

4. **Array of Structs in Hot Paths**
   ```rust
   // ❌ IMMEDIATE REJECTION (in performance-critical code)
   struct Entity { pos: Vec3, vel: Vec3, health: f32 }
   let entities: Vec<Entity> = vec![];
   ```

### ⚠️ WARNING CRITERIA (Requires Strong Justification)

These patterns require explicit justification and domain expert approval:

1. **HashMap Usage** (prefer flat arrays)
2. **Vec::push in loops** (prefer pre-allocation)
3. **Complex control flow in kernels** (prefer data-driven)
4. **File I/O in hot paths** (prefer buffered batch operations)

## Review Process

### Step 1: Automated Checks (Pre-Review)

Before human review, PR must pass:

```bash
# Run automated DOP compliance
./scripts/check_dop_compliance.sh

# Ensure compilation with strict lints
cargo clippy -- -D warnings -D clippy::methods_on_data_structs

# Run performance regression tests
cargo bench --bench dop_patterns
```

### Step 2: Data Layout Review

#### ✅ Required: Structure of Arrays (SoA)

```rust
// ✅ APPROVE: SoA layout
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
```

**Review Questions:**
- [ ] Are related arrays the same length?
- [ ] Is capacity pre-allocated?
- [ ] Are arrays aligned for SIMD?
- [ ] Can this be uploaded to GPU?

#### ❌ Reject: Array of Structs (AoS)

```rust
// ❌ REJECT: AoS layout
pub struct Player {
    pub position: Vec3,
    pub health: f32,
    pub velocity: Vec3,
}
pub struct PlayerList {
    players: Vec<Player>, // ❌ Cache-hostile
}
```

### Step 3: Function Design Review

#### ✅ Required: Kernel Functions (Stateless)

```rust
// ✅ APPROVE: Pure kernel function
pub fn update_player_physics(
    player_data: &mut PlayerData,
    input_data: &InputData,
    dt: f32,
) {
    for i in 0..player_data.count {
        // Apply forces
        player_data.velocities_x[i] += input_data.forces_x[i] * dt;
        player_data.velocities_y[i] += input_data.forces_y[i] * dt;
        player_data.velocities_z[i] += input_data.forces_z[i] * dt;
        
        // Update positions
        player_data.positions_x[i] += player_data.velocities_x[i] * dt;
        player_data.positions_y[i] += player_data.velocities_y[i] * dt;
        player_data.positions_z[i] += player_data.velocities_z[i] * dt;
    }
}
```

**Review Questions:**
- [ ] Function is pure (no hidden state)?
- [ ] Same inputs always produce same outputs?
- [ ] No side effects except explicit data mutation?
- [ ] Operates on arrays, not individual items?
- [ ] SIMD-friendly loop structure?

#### ❌ Reject: Methods or Stateful Functions

```rust
// ❌ REJECT: Method with state
impl PlayerSystem {
    pub fn update(&mut self, dt: f32) {
        for player in &mut self.players {
            player.update(dt); // ❌ Method call!
        }
    }
}
```

### Step 4: Memory Management Review

#### ✅ Required: Pre-allocated Pools

```rust
// ✅ APPROVE: Pre-allocated pool
pub fn create_particle_pool(capacity: usize) -> ParticlePool {
    ParticlePool {
        capacity,
        active_count: 0,
        // Pre-allocate all arrays
        positions_x: vec![0.0; capacity],
        positions_y: vec![0.0; capacity],
        positions_z: vec![0.0; capacity],
        ages: vec![0.0; capacity],
        lifetimes: vec![1.0; capacity],
    }
}

pub fn spawn_particles(
    pool: &mut ParticlePool,
    spawn_positions: &[Vec3],
) -> Result<(), PoolExhausted> {
    let available = pool.capacity - pool.active_count;
    if spawn_positions.len() > available {
        return Err(PoolExhausted);
    }
    
    // Copy into pre-allocated arrays
    for (i, &pos) in spawn_positions.iter().enumerate() {
        let index = pool.active_count + i;
        pool.positions_x[index] = pos.x;
        pool.positions_y[index] = pos.y;
        pool.positions_z[index] = pos.z;
        pool.ages[index] = 0.0;
    }
    
    pool.active_count += spawn_positions.len();
    Ok(())
}
```

**Review Questions:**
- [ ] Are allocations done once at startup?
- [ ] No `Vec::push()` in hot paths?
- [ ] No `HashMap::insert()` during gameplay?
- [ ] Fixed-size pools for dynamic objects?
- [ ] Clear strategy for pool exhaustion?

#### ❌ Reject: Runtime Allocation

```rust
// ❌ REJECT: Runtime allocation
pub fn spawn_particle(&mut self, position: Vec3) {
    self.particles.push(Particle { // ❌ Runtime allocation!
        position,
        velocity: Vec3::ZERO,
        age: 0.0,
    });
}
```

### Step 5: GPU Compatibility Review

#### ✅ Required: GPU-Ready Data

```rust
// ✅ APPROVE: GPU-compatible layout
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexData {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct MeshData {
    pub vertex_count: u32,
    pub vertices: Vec<VertexData>,
    pub indices: Vec<u32>,
    // Direct GPU buffer mapping
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
}

pub fn upload_mesh_to_gpu(
    mesh_data: &mut MeshData,
    device: &wgpu::Device,
) {
    mesh_data.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&mesh_data.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    }));
}
```

**Review Questions:**
- [ ] Data structures are `#[repr(C)]`?
- [ ] Implement `Pod` and `Zeroable` for GPU upload?
- [ ] No padding issues in structs?
- [ ] Buffer creation is explicit, not hidden?
- [ ] Compute shader compatibility verified?

#### ❌ Reject: GPU-Incompatible Patterns

```rust
// ❌ REJECT: Complex data that can't be uploaded
pub struct ComplexMesh {
    vertices: HashMap<String, Vec<CustomVertex>>, // ❌ Can't upload
    render_fn: Box<dyn Fn(&Device)>,              // ❌ Function pointers
}
```

## Specialized Review Checklists

### Performance-Critical Files

For files in `src/renderer/`, `src/world_gpu/`, `src/physics_data/`, `src/particles/`, `src/lighting/`:

#### Extra Requirements:
- [ ] **Zero methods with self** (stricter than general code)
- [ ] **SIMD-friendly loops** (power-of-2 iteration, aligned data)
- [ ] **GPU compute shader integration** planned or implemented
- [ ] **Cache line alignment** for hot data structures
- [ ] **Benchmark results** showing performance improvement

#### Benchmark Requirements:
```rust
#[bench]
fn bench_your_new_kernel(b: &mut Bencher) {
    let mut data = YourData::new(10000);
    
    b.iter(|| {
        your_kernel_function(&mut data);
    });
    
    // Performance assertion
    assert!(b.elapsed().as_nanos() < PERFORMANCE_THRESHOLD);
}
```

### Networking/Protocol Files

For files in `src/network/`:

#### Extra Requirements:
- [ ] **Packet formats are data-only** (no behavior methods)
- [ ] **Serialization is zero-copy** where possible
- [ ] **Buffer-based protocols** instead of object hierarchies
- [ ] **Batch processing** for multiple packets

### Memory Management Files

For files in `src/memory/`, `src/streaming/`:

#### Extra Requirements:
- [ ] **Pool-based allocation** strategy
- [ ] **Fixed-size budgets** with clear overflow handling
- [ ] **Memory-mapped I/O** for large data
- [ ] **GPU memory synchronization** strategy

## Review Approval Process

### Level 1: Automatic Approval
- **Criteria**: Passes all automated checks + simple DOP patterns
- **Reviewers**: Any team member familiar with DOP
- **Turnaround**: Same day

### Level 2: Standard Review
- **Criteria**: Complex DOP patterns, performance implications
- **Reviewers**: Senior team members + domain expert
- **Turnaround**: 2-3 days

### Level 3: Architecture Review
- **Criteria**: New patterns, cross-system changes, performance critical
- **Reviewers**: Tech lead + architecture team
- **Turnaround**: 1 week
- **Requirements**: Design document, benchmarks, migration plan

## Enforcement

### PR Review Template

Use this template for all PR reviews:

```markdown
## DOP Compliance Review

### Automated Checks
- [ ] `./scripts/check_dop_compliance.sh` passes
- [ ] `cargo clippy` with DOP lints passes
- [ ] Performance benchmarks pass

### Data Layout
- [ ] Structure of Arrays (SoA) layout used
- [ ] No Array of Structs (AoS) in hot paths
- [ ] GPU-compatible data structures
- [ ] Pre-allocated pools for dynamic data

### Function Design
- [ ] Kernel functions (no methods with self)
- [ ] Pure functions (stateless)
- [ ] Batch processing of arrays
- [ ] SIMD-friendly loops

### Performance
- [ ] No runtime allocation in hot paths
- [ ] Cache-friendly memory access patterns
- [ ] Benchmarks demonstrate improvement
- [ ] GPU compatibility verified

### Domain-Specific Requirements
- [ ] [Performance-critical] Zero methods, SIMD optimization
- [ ] [Networking] Buffer-based protocols
- [ ] [Memory] Pool-based allocation

### Final Approval
- [ ] All DOP patterns followed
- [ ] Performance requirements met
- [ ] Architecture consistent with engine design
```

### Review Training

#### New Team Members
1. **Study**: Read `docs/guides/DOP_ENFORCEMENT.md`
2. **Practice**: Review 10 approved DOP PRs
3. **Shadow**: Co-review 5 PRs with senior reviewer
4. **Certification**: Successfully review 3 PRs independently

#### Ongoing Training
- **Monthly**: Review DOP compliance metrics
- **Quarterly**: Update review criteria based on new patterns
- **Annually**: Major architecture review and guideline updates

## Common Mistakes and How to Catch Them

### 1. Hidden OOP Patterns

**Mistake**: Converting method to function but keeping object thinking
```rust
// ❌ Still OOP thinking
pub fn update_player(player: &mut Player, dt: f32) {
    player.velocity += player.forces * dt;
    player.position += player.velocity * dt;
}
```

**Correct DOP**:
```rust
// ✅ True DOP thinking
pub fn update_players(
    positions_x: &mut [f32],
    positions_y: &mut [f32],
    velocities_x: &mut [f32],
    velocities_y: &mut [f32],
    forces_x: &[f32],
    forces_y: &[f32],
    dt: f32,
) {
    for i in 0..positions_x.len() {
        velocities_x[i] += forces_x[i] * dt;
        velocities_y[i] += forces_y[i] * dt;
        positions_x[i] += velocities_x[i] * dt;
        positions_y[i] += velocities_y[i] * dt;
    }
}
```

### 2. Premature Abstraction

**Mistake**: Creating abstractions before understanding patterns
```rust
// ❌ Premature abstraction
trait Updatable {
    fn update(&mut self, dt: f32);
}
```

**Correct Approach**: Write 3+ concrete kernel functions first, then extract common patterns

### 3. Hidden Allocations

**Mistake**: Allocations disguised as innocent operations
```rust
// ❌ Hidden allocation
pub fn get_nearby_entities(&self, pos: Vec3) -> Vec<EntityId> {
    self.entities.iter()
        .filter(|e| e.distance_to(pos) < 10.0)
        .map(|e| e.id)
        .collect() // ❌ Allocation!
}
```

**Correct DOP**:
```rust
// ✅ Pre-allocated output buffer
pub fn find_nearby_entities(
    entity_positions_x: &[f32],
    entity_positions_y: &[f32],
    entity_ids: &[EntityId],
    search_pos: Vec3,
    radius: f32,
    output_buffer: &mut [EntityId],
) -> usize {
    let mut count = 0;
    let radius_sq = radius * radius;
    
    for i in 0..entity_positions_x.len() {
        let dx = entity_positions_x[i] - search_pos.x;
        let dy = entity_positions_y[i] - search_pos.y;
        let dist_sq = dx * dx + dy * dy;
        
        if dist_sq <= radius_sq && count < output_buffer.len() {
            output_buffer[count] = entity_ids[i];
            count += 1;
        }
    }
    
    count
}
```

## Success Metrics

Track these metrics for each reviewed PR:

### Code Quality Metrics
- **OOP Violations**: 0 (required)
- **Methods with Self**: 0 (required)
- **Trait Objects**: 0 (required)
- **SoA Adoption**: >90% of data structures
- **Pre-allocation Usage**: >90% of dynamic operations

### Performance Metrics
- **Cache Efficiency**: >95% for sequential access
- **SIMD Utilization**: >50% of numeric operations
- **GPU Buffer Usage**: >80% of graphics data
- **Memory Allocations**: <10 per frame in hot paths

### Review Quality Metrics
- **Review Turnaround**: <2 days average
- **Post-Review Issues**: <5% of approved PRs
- **Team DOP Knowledge**: 100% certified reviewers

Remember: **The goal is not just to prevent OOP, but to actively promote and recognize excellent DOP patterns.**