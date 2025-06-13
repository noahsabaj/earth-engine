# Rendering Examples

This directory contains examples related to GPU rendering, mesh generation, and visual effects.

## Examples

### `async_mesh_integration.rs`
Demonstrates asynchronous mesh building to avoid blocking the main thread during chunk generation.

### `mesh_builder_integration.rs`
Shows how to use the mesh builder API to create custom voxel meshes with proper normals and ambient occlusion.

## Running

```bash
cargo run --example async_mesh_integration
cargo run --example mesh_builder_integration
```

## Topics Covered

- GPU buffer management
- Mesh generation algorithms
- Shader pipeline usage
- Async rendering techniques
- Memory optimization