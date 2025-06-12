# GPU Memory Leak Fix for Async Renderer

## Problem
The `SimpleAsyncRenderer` had a memory leak where GPU buffers for chunk meshes were never released when chunks were unloaded from the world. The TODO comment in `cleanup_unloaded_chunks` indicated that `ParallelWorld` didn't expose the necessary methods to track loaded chunks.

## Solution
The fix involved three main changes:

### 1. Added method to ParallelChunkManager
```rust
/// Get loaded chunk positions
pub fn get_loaded_chunk_positions(&self) -> Vec<ChunkPos> {
    self.chunks
        .iter()
        .map(|entry| *entry.key())
        .collect()
}
```

### 2. Exposed method through ParallelWorld
```rust
/// Get loaded chunk positions for cleanup purposes
pub fn get_loaded_chunk_positions(&self) -> Vec<ChunkPos> {
    self.chunk_manager.get_loaded_chunk_positions()
}
```

### 3. Implemented cleanup in SimpleAsyncRenderer
```rust
/// Remove meshes for unloaded chunks
pub fn cleanup_unloaded_chunks(&mut self, world: &Arc<ParallelWorld>) {
    // Get currently loaded chunk positions
    let loaded_positions: std::collections::HashSet<ChunkPos> = world
        .get_loaded_chunk_positions()
        .into_iter()
        .collect();
    
    // Find GPU meshes that no longer have corresponding loaded chunks
    let unloaded_chunks: Vec<ChunkPos> = self.gpu_meshes
        .keys()
        .filter(|pos| !loaded_positions.contains(pos))
        .cloned()
        .collect();
    
    // Remove GPU buffers for unloaded chunks
    for chunk_pos in unloaded_chunks {
        self.gpu_meshes.remove(&chunk_pos);
    }
}
```

### 4. Integrated cleanup into render loop
In `gpu_state.rs`, the `update_chunk_renderer` method now calls cleanup:
```rust
fn update_chunk_renderer(&mut self, input: &InputState) {
    // Queue dirty chunks for async mesh building
    self.chunk_renderer.queue_dirty_chunks(&self.world, &self.camera);
    
    // Update the async renderer (process queue and upload meshes)
    self.chunk_renderer.update(&self.device);
    
    // Clean up GPU buffers for unloaded chunks
    self.chunk_renderer.cleanup_unloaded_chunks(&self.world);
    
    // World update handles chunk loading/unloading automatically
    self.world.update(self.camera.position);
}
```

## Benefits
- **Memory efficiency**: GPU buffers are properly released when chunks unload
- **No architecture changes**: The fix respects the existing architecture boundaries
- **Performance**: Cleanup is efficient using HashSet for O(1) lookups
- **Automatic**: Cleanup happens automatically in the render loop

## Testing
See `examples/gpu_memory_leak_fix.rs` for a demonstration of the fix in action.

## Future Improvements
- Consider adding memory usage metrics to track GPU buffer allocation
- Implement buffer pooling to reuse GPU buffers instead of deallocating
- Add configuration for cleanup frequency (e.g., every N frames)