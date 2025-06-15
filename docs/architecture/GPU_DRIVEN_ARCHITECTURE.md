# GPU-Driven Rendering Architecture

## Overview

The GPU-driven rendering pipeline represents a fundamental shift in how we submit work to the GPU. Instead of the CPU deciding what to draw, the GPU makes these decisions using compute shaders, eliminating the CPU bottleneck and enabling massive object counts.

## Key Concepts

### Traditional vs GPU-Driven

**Traditional Rendering:**
```
CPU: foreach object {
    if (visible) {
        SetTransform(object.matrix)
        DrawMesh(object.mesh)  // Driver overhead!
    }
}
```

**GPU-Driven Rendering:**
```
CPU: UploadAllObjectData()
GPU: CullAndDrawEverything()  // One command!
```

## Architecture Components

### 1. Indirect Draw Commands (`indirect_commands.rs`)

GPU buffers containing draw parameters:
```rust
struct IndirectDrawCommand {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}
```

- **Zero CPU draw calls**: GPU reads commands directly
- **Dynamic generation**: Compute shaders write commands
- **Multiple passes**: Opaque, transparent, shadow

### 2. Instance Data (`instance_buffer.rs`)

Per-instance data stored on GPU:
```rust
struct InstanceData {
    model_matrix: [[f32; 4]; 4],  // World transform
    color: [f32; 4],              // Instance color
    custom_data: [f32; 4],        // LOD, flags, etc.
}
```

- **Persistent storage**: Data stays on GPU
- **Efficient updates**: Only changed instances uploaded
- **Massive counts**: Designed for 100K+ instances (performance varies by hardware)

### 3. GPU Culling (`culling_pipeline.rs`)

Compute shader performs visibility tests:
```wgsl
@compute @workgroup_size(64)
fn cull_instances() {
    // Frustum culling
    if (!sphere_inside_frustum(center, radius)) {
        return; // Culled!
    }
    
    // Distance culling
    if (distance > max_distance) {
        return; // Too far!
    }
    
    // Write draw command
    indirect_commands[draw_index] = ...
}
```

- **Parallel culling**: 64 objects per workgroup
- **Atomic counters**: Thread-safe command generation
- **Statistics**: Track culled vs drawn

### 4. LOD System (`lod_system.rs`)

Automatic level-of-detail selection:
```rust
// Distance-based LOD
LOD 0: 0-50m    (full detail)
LOD 1: 50-150m  (half detail)
LOD 2: 150-400m (quarter detail)
LOD 3: 400m+    (minimal detail)
```

- **Screen-space metrics**: Pixel-accurate LOD
- **Smooth transitions**: Fade/dither between levels
- **Global bias**: Quality vs performance control

### 5. Multi-threaded Command Building

CPU threads prepare data in parallel:
```rust
Thread 1: Update chunk instances
Thread 2: Update entity instances  
Thread 3: Update particle instances
Thread 4: Build culling metadata
```

## Rendering Flow

### Frame Lifecycle

1. **CPU Preparation** (Multi-threaded)
   ```rust
   renderer.begin_frame(camera);
   renderer.submit_objects(chunks);
   renderer.submit_objects(entities);
   renderer.build_commands();
   ```

2. **GPU Culling**
   ```wgsl
   // Reset counters
   reset_counters();
   
   // Cull all objects in parallel
   cull_instances();
   ```

3. **GPU Drawing**
   ```rust
   // Single indirect draw for thousands of objects
   render_pass.multi_draw_indirect(
       command_buffer,
       draw_count
   );
   ```

## Performance Characteristics

### CPU Performance
- **Minimal driver overhead**: 1 draw call vs 10,000
- **Parallel data preparation**: Linear scaling with cores
- **No state changes**: GPU manages everything

### GPU Performance
- **Efficient culling**: Only visible objects processed
- **Cache coherent**: Sequential memory access
- **Parallel execution**: Thousands of threads

### Memory Usage
```
Commands:  100K × 20 bytes = 2 MB
Instances: 100K × 96 bytes = 9.6 MB
Metadata:  100K × 48 bytes = 4.8 MB
Total:     ~16.4 MB for 100K objects
```

## Integration Example

```rust
// Setup
let mut renderer = GpuDrivenRenderer::new(device, queue, format);

// Submit objects
for chunk in visible_chunks {
    renderer.submit_objects(&[RenderObject {
        position: chunk.world_position(),
        scale: 1.0,
        color: [1.0, 1.0, 1.0, 1.0],
        bounding_radius: 32.0,
        mesh_id: chunk.lod_mesh_id(),
        material_id: chunk.material_id(),
    }]);
}

// Render
renderer.build_commands();
renderer.render(&mut encoder, &mut render_pass, camera_bind_group);
```

## Advantages

1. **Scalability**: Architecture supports many objects with reduced CPU usage
2. **Efficiency**: GPU decides what to draw
3. **Flexibility**: Easy to add new object types
4. **Future-proof**: Ready for mesh shaders

## Limitations

1. **GPU memory**: All data must fit in VRAM
2. **Indirect support**: Requires modern GPU features
3. **Debugging**: Harder to trace GPU decisions

## Future Enhancements

### Sprint 21 Integration
- Mesh generation on GPU
- Direct voxel to draw commands
- Zero CPU involvement

### Advanced Features
- **GPU-driven LOD generation**: Create LODs on demand
- **Temporal culling**: Reuse previous frame results
- **Hierarchical culling**: Cull groups before individuals
- **Variable rate shading**: Reduce shading cost for distant objects

## Best Practices

1. **Batch similar objects**: Group by material/mesh
2. **Update only changes**: Don't reupload static data
3. **Profile GPU time**: Monitor culling cost
4. **Balance workgroups**: Even distribution across SMs

## Debugging

### Enable debug stats:
```rust
let stats = renderer.stats();
println!("Drawn: {}/{}", stats.objects_drawn, stats.objects_submitted);
println!("Culled: {} frustum, {} distance", 
    stats.frustum_culled, 
    stats.distance_culled
);
```

### Visualize LOD levels:
Use `fs_main_lod_debug` fragment shader to color by LOD.

## Conclusion

GPU-driven rendering eliminates the CPU as a bottleneck, enabling unprecedented object counts and paving the way for fully GPU-resident worlds in Sprint 21. This architecture scales with GPU power rather than CPU, future-proofing the engine for years to come.