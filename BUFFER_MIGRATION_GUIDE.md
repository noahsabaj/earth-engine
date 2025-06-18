# GPU Buffer Layout Migration Guide

This guide shows how to migrate existing code to use the new centralized buffer layout system.

## Migration Examples

### 1. World Buffer Migration

**Before (world_buffer.rs):**
```rust
pub struct VoxelData(pub u32);

impl WorldBuffer {
    pub fn slot_offset(&self, slot: u32) -> u64 {
        slot as u64 * VOXELS_PER_CHUNK as u64 * std::mem::size_of::<VoxelData>() as u64
    }
}
```

**After:**
```rust
use crate::gpu::buffer_layouts::{VoxelData, calculations, WorldBufferLayout};

impl WorldBuffer {
    pub fn slot_offset(&self, slot: u32) -> u64 {
        calculations::chunk_slot_offset(slot)
    }
}
```

### 2. Instance Buffer Migration

**Before (instance_buffer.rs):**
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InstanceData {
    pub model_matrix: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub custom_data: [f32; 4],
}

let buffer_size = (std::mem::size_of::<InstanceData>() * capacity as usize) as u64;
```

**After:**
```rust
use crate::gpu::buffer_layouts::{InstanceData, InstanceBufferLayout};

let buffer_size = InstanceBufferLayout::buffer_size(capacity);
```

### 3. Bind Group Creation Migration

**Before:**
```rust
let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("World Buffer Bind Group Layout"),
    entries: &[
        wgpu::BindGroupLayoutEntry {
            binding: 0, // Hardcoded binding index
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ],
});
```

**After:**
```rust
use crate::gpu::buffer_layouts::{bindings, layouts};

let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("World Buffer Bind Group Layout"),
    entries: &[
        layouts::storage_buffer_entry(
            bindings::world::VOXEL_BUFFER,
            false,
            wgpu::ShaderStages::COMPUTE
        ),
    ],
});
```

### 4. Camera Uniform Migration

**Before (culling_pipeline.rs):**
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraData {
    pub view_proj: [[f32; 4]; 4],
    pub position: [f32; 3],
    pub _padding0: f32,
    pub frustum_planes: [[f32; 4]; 6],
    pub _padding1: [f32; 8],
}
```

**After:**
```rust
use crate::gpu::buffer_layouts::{CameraUniform, CullingCameraData};

// Use CameraUniform for standard rendering
// Use CullingCameraData for GPU culling operations
```

### 5. Buffer Size Calculations Migration

**Before:**
```rust
// Scattered throughout the codebase
const INSTANCE_SIZE: u64 = 96; // Hope this is right!
const CHUNK_SIZE: u64 = 32768 * 4; // Voxels * sizeof(u32)
```

**After:**
```rust
use crate::gpu::buffer_layouts::constants::*;

// All sizes defined in one place
let instance_size = INSTANCE_DATA_SIZE;
let chunk_size = CHUNK_BUFFER_SLOT_SIZE;
```

## Step-by-Step Migration Process

### Phase 1: Update Imports
1. Add `use crate::gpu::buffer_layouts;` to files using GPU buffers
2. Import specific types as needed

### Phase 2: Replace Struct Definitions
1. Remove local buffer struct definitions
2. Import from `buffer_layouts` module instead
3. Update any custom methods to use the centralized versions

### Phase 3: Update Buffer Calculations
1. Replace hardcoded size calculations with constants
2. Use helper functions from `calculations` module
3. Update offset calculations to use centralized functions

### Phase 4: Update Bind Groups
1. Replace hardcoded binding indices with constants from `bindings`
2. Use helper functions from `layouts` module for bind group entries
3. Update shader bindings to match

### Phase 5: Test and Validate
1. Run GPU validation tests
2. Check buffer alignment with debug assertions
3. Verify shader compatibility

## Benefits After Migration

1. **Single Source of Truth**: All buffer layouts in one place
2. **Type Safety**: Compile-time validation of buffer compatibility
3. **Easier Maintenance**: Update layouts in one location
4. **Better Documentation**: Centralized documentation of memory layouts
5. **Reduced Bugs**: No more mismatched buffer sizes or binding indices

## Common Pitfalls to Avoid

1. **Don't Mix Old and New**: Fully migrate a module at once
2. **Update Shaders**: Ensure WGSL matches new binding indices
3. **Check Alignment**: Use provided alignment helpers
4. **Test Thoroughly**: GPU bugs can be subtle

## Shader Updates Required

When migrating, update your WGSL shaders to use the new binding indices:

**Before:**
```wgsl
@group(0) @binding(0) var<storage, read_write> voxels: array<u32>;
@group(0) @binding(1) var<storage, read> metadata: array<ChunkMetadata>;
```

**After:**
```wgsl
// Use constants from generated constants.wgsl
@group(0) @binding(WORLD_VOXEL_BUFFER) var<storage, read_write> voxels: array<u32>;
@group(0) @binding(WORLD_METADATA_BUFFER) var<storage, read> metadata: array<ChunkMetadata>;
```