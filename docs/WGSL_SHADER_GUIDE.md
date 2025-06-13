# WGSL Shader Guide - Earth Engine

## What is WGSL?

WGSL (WebGPU Shading Language) is a shader programming language designed for WebGPU. Shaders are small programs that run on your GPU in massive parallel execution, processing thousands of vertices or pixels simultaneously.

## Why Shaders Matter

- **CPU**: Processes tasks sequentially (1 core = 1 task at a time)
- **GPU**: Processes tasks in parallel (3000+ cores = 3000+ tasks simultaneously)

For voxel rendering, this means:
- CPU: Loop through each voxel one by one
- GPU: Process thousands of voxels at once

## Earth Engine Shader Architecture

### 1. Rendering Pipeline (Active)

**Core Shaders:**
- `voxel.wgsl` - Standard voxel rendering with lighting, fog, AO
- `gpu_driven.wgsl` - Advanced GPU-driven rendering with instancing
- `gpu_culling.wgsl` - Frustum culling to skip invisible chunks

**How it works:**
1. CPU submits chunk data to GPU
2. GPU culling shader removes chunks outside view
3. GPU driven shader renders visible chunks with instancing
4. Each pixel gets lighting, fog, and color applied

### 2. Compute Shaders (GPU Processing)

**World Generation:**
- `terrain_generation.wgsl` - Generate terrain on GPU
- `chunk_modification.wgsl` - Modify voxels on GPU
- `ambient_occlusion.wgsl` - Calculate shadows in corners

**Fluid Simulation:**
- 12 shaders for realistic water physics
- Handles flow, pressure, viscosity entirely on GPU

**SDF Processing:**
- 15 shaders for smooth terrain generation
- Marching cubes algorithm for organic shapes

### 3. Shader Language Basics

```wgsl
// Example from voxel.wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,    // 3D position
    @location(1) normal: vec3<f32>,      // Surface direction
    @location(2) color: vec3<f32>,       // RGB color
    @location(3) ao: f32,                // Ambient occlusion
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    // Runs once per vertex (corner of triangle)
    var out: VertexOutput;
    out.position = camera.view_proj * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Runs once per pixel
    return vec4<f32>(final_color, 1.0);
}
```

### 4. Performance Impact

Your shaders enable:
- **60,000+ voxels rendered per chunk**
- **Parallel processing of millions of voxels**
- **Real-time lighting and shadows**
- **GPU-based frustum culling**

### 5. Dead Code Found

Only 1 unused shader:
- `particles/gpu_update.wgsl` - Particle system never migrated to GPU

### 6. Future Optimizations

1. **Enable Hot Reload**: Shader code can update without restarting
2. **Add Preprocessing**: Conditional compilation for features
3. **Implement GPU Particles**: Use the dead shader
4. **Profile Shader Performance**: Find bottlenecks

## Common WGSL Patterns

### Buffer Binding
```wgsl
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<storage, read> voxel_data: array<u32>;
```

### Workgroup Compute
```wgsl
@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // Process 8x8x8 block in parallel
}
```

### Atomic Operations
```wgsl
atomicAdd(&counters.visible_count, 1u);  // Thread-safe increment
```

## Debugging Shaders

1. **Compilation Errors**: Check `cargo run` output
2. **Runtime Issues**: Use RenderDoc or GPU debugger
3. **Performance**: Use GPU profiler to find slow shaders

## Summary

Your WGSL shaders are well-organized and mostly functional. The GPU-driven architecture is properly implemented with good separation between rendering and compute workloads. The main improvement would be enabling the hot reload system for faster iteration.