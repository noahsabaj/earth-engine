# Sprint 35.2: DOP Reality Check

## Status: FIXING THE LIE 🔧

### Overview
Sprint 35 claimed "complete DOP transition". Reality: 228 files still have methods. This sprint makes DOP REAL.

### Goals (Week 3-4)

#### Week 3: DOP Exemplar Module
- [ ] Choose camera.rs as exemplar
- [ ] Document BEFORE state (methods, self, allocations)
- [ ] Convert to pure functions + data
- [ ] Document AFTER state with benchmarks
- [ ] Create DOP_PATTERNS.md guide
- [ ] Measure performance delta

#### Week 4: Systematic Conversion
- [ ] List all 228 files with impl blocks
- [ ] Create conversion checklist
- [ ] Convert 10 highest-impact modules
- [ ] Verify zero allocations in hot paths
- [ ] Add allocation tests
- [ ] Update architecture docs

### DOP Conversion Pattern

```rust
// BEFORE (OOP):
impl Camera {
    pub fn new(fov: f32) -> Self {
        Self { fov, view: Matrix4::identity() }
    }
    
    pub fn update(&mut self, dt: f32) {
        self.view = self.calculate_view();
    }
}

// AFTER (DOP):
pub struct CameraData {
    pub fov: f32,
    pub position: Vec3,
    pub rotation: Quat,
}

pub struct CameraBuffers {
    pub data: Buffer<CameraData>,
    pub matrices: Buffer<CameraMatrices>,
}

pub fn update_cameras(
    data: &Buffer<CameraData>,
    matrices: &mut Buffer<CameraMatrices>,
    count: usize,
) {
    // Pure function, no self, no allocations
    parallel_compute(|i| {
        matrices[i] = calculate_view_matrix(&data[i]);
    });
}
```

### Success Criteria
- 10 modules fully DOP ✓
- Zero allocations verified ✓
- Performance improved or equal ✓
- Pattern guide complete ✓

### Tracking Metrics
- Files with impl: 228 → 218 (goal)
- Allocations per frame: 268 → 0 (goal)
- Cache miss rate: Measure before/after