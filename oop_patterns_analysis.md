# OOP Patterns Analysis - Data-Oriented Design Conversion

## 1. Methods that take &mut self (excluding simple setters/getters)

### High Priority Conversions:

#### ChunkManager (src/world/chunk_manager.rs)
- `update_loaded_chunks(&mut self, player_pos: Point3<f32>)` - Complex state management
- `set_block(&mut self, pos: VoxelPos, block: BlockId)` - Modifies internal state and dirty tracking
- `take_dirty_chunks(&mut self)` - State modification

#### Camera (src/camera/mod.rs)
- `move_forward(&mut self, amount: f32)` - Movement logic
- `move_right(&mut self, amount: f32)` - Movement logic
- `rotate(&mut self, delta_yaw: f32, delta_pitch: f32)` - Rotation with clamping logic

#### ParticleSystem
- Various update methods that modify particle states

#### WeatherSystem
- Update methods that modify weather state

## 2. Trait Implementations with Complex Logic

### WorldGenerator trait implementations
- `DefaultWorldGenerator::generate_chunk()` - Complex terrain generation logic
- Should be converted to free functions with data passed as parameters

### Other trait implementations
- Most Display/Debug/From implementations are fine to keep
- Focus on traits that perform complex computations

## 3. Structs with Internal State Management Methods

Found extensive use of update/process/tick methods in:
- Physics systems
- Particle systems
- Weather systems
- Time/day-night cycle systems
- Network systems
- ECS systems

These should be refactored to:
- Separate data structures from processing logic
- Use free functions that operate on data
- Pass state explicitly rather than encapsulating it

## 4. "new" Constructors that Allocate on Every Call

### ParticleEmitter::new()
- Creates new emitter with default values
- Should use object pools or pre-allocated emitters

### ChunkMesh::new()
- Allocates new Vec instances
- Should reuse mesh buffers from a pool

### Many other constructors allocating:
- HashMap::new()
- Vec::new()
- Complex nested structures

## 5. Vec::new() or HashMap::new() in Non-initialization Code

### ChunkMesh (src/renderer/mesh.rs)
```rust
pub fn new() -> Self {
    Self {
        vertices: Vec::new(),  // Allocation on every mesh creation
        indices: Vec::new(),   // Allocation on every mesh creation
    }
}
```

### Found in many files:
- Mesh generation creating new vectors
- Temporary collections in update loops
- Per-frame allocations in render code

## 6. Box, Rc, or Arc Usage Outside One-time Setup

### BlockRegistry (src/world/registry.rs)
- Uses `Arc<dyn Block>` for block storage - this is acceptable for one-time setup
- `get_block()` clones Arc on every call - could return reference instead

### Other concerning usage:
- Chunk management using Arc<RwLock<Chunk>>
- Network systems using Arc for message passing
- ECS systems using Arc for component storage

## Recommendations for Conversion:

1. **Convert ChunkManager to data-oriented design:**
   - Separate chunk storage from management logic
   - Use flat arrays for chunk data
   - Process chunks in batches

2. **Convert Camera to POD structure:**
   - Make Camera a plain data struct
   - Move update logic to free functions
   - Pass camera data to functions that need it

3. **Pool all temporary allocations:**
   - Create mesh buffer pools
   - Reuse Vec/HashMap instances
   - Pre-allocate particle arrays

4. **Flatten nested structures:**
   - Convert object hierarchies to flat arrays
   - Use indices instead of pointers/references
   - Process data in cache-friendly order

5. **Remove unnecessary Arc/Rc usage:**
   - Use indices into arrays instead
   - Pass references where lifetime allows
   - Only use Arc for truly shared, long-lived data

6. **Convert update methods to batch operations:**
   - Process all particles at once
   - Update all chunks in a single pass
   - Batch similar operations together