# GPU Buffer Layout Refactoring Plan

## Overview
This document outlines a comprehensive plan to centralize and standardize all GPU buffer layout definitions across the hearth-engine codebase.

## Current State Analysis

### 1. Buffer Definitions are Scattered Across:
- `src/world_gpu/world_buffer.rs` - Voxel data and chunk slot calculations
- `src/renderer/gpu_driven/instance_buffer.rs` - Instance data structures
- `src/renderer/gpu_driven/indirect_commands.rs` - Indirect draw commands
- `src/renderer/gpu_driven/culling_pipeline.rs` - Camera and culling data
- `src/gpu/types/core.rs` - Basic GPU type definitions
- `src/gpu/constants.rs` - GPU constants but no buffer layouts
- `src/world_gpu/terrain_generator_soa.rs` - Terrain metadata buffers

### 2. Key Issues Identified:
- **Duplicated buffer size calculations** across multiple files
- **Hardcoded binding indices** scattered throughout the codebase
- **Inconsistent naming conventions** for buffer types
- **No centralized documentation** of memory layouts
- **Buffer slot calculations** duplicated in multiple places
- **Alignment requirements** handled inconsistently

### 3. Buffer Types to Centralize:
- **World Buffer**: Voxel data storage, chunk metadata
- **Instance Buffers**: Instance data, culling instance data
- **Command Buffers**: Indirect draw commands (regular and indexed)
- **Camera Buffer**: Camera uniforms and frustum data
- **Terrain Buffers**: Terrain parameters (AOS and SOA variants)
- **Mesh Buffers**: Vertex and index buffer layouts
- **Compute Buffers**: Various compute shader buffers

## Proposed Solution

### 1. Create New Centralized Module Structure
```
src/gpu/buffer_layouts/
├── mod.rs              # Main module file with exports
├── world.rs            # World buffer layouts
├── instance.rs         # Instance buffer layouts
├── commands.rs         # Indirect command layouts
├── camera.rs           # Camera and view layouts
├── terrain.rs          # Terrain generation layouts
├── mesh.rs             # Vertex/index buffer layouts
├── compute.rs          # Compute shader layouts
└── constants.rs        # Buffer constants and calculations
```

### 2. Centralized Buffer Layout Module Features

#### A. Buffer Structure Definitions
- All GPU buffer structures in one place
- Consistent use of `#[repr(C)]` and bytemuck traits
- Clear documentation of memory layout

#### B. Buffer Size Constants
```rust
pub mod sizes {
    pub const VOXEL_DATA_SIZE: u64 = 4;
    pub const INSTANCE_DATA_SIZE: u64 = 96;
    pub const INDIRECT_COMMAND_SIZE: u64 = 16;
    pub const CAMERA_UNIFORM_SIZE: u64 = 256;
    // etc...
}
```

#### C. Binding Indices
```rust
pub mod bindings {
    pub const WORLD_VOXEL_BUFFER: u32 = 0;
    pub const WORLD_METADATA_BUFFER: u32 = 1;
    pub const CAMERA_UNIFORM: u32 = 0;
    pub const INSTANCE_BUFFER: u32 = 1;
    // etc...
}
```

#### D. Buffer Offset Calculations
```rust
pub fn calculate_chunk_slot_offset(slot: u32) -> u64 {
    slot as u64 * VOXELS_PER_CHUNK as u64 * VOXEL_DATA_SIZE
}

pub fn calculate_instance_offset(index: u32) -> u64 {
    index as u64 * INSTANCE_DATA_SIZE
}
```

#### E. Bind Group Layout Builders
```rust
pub fn create_world_buffer_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    // Centralized bind group layout creation
}
```

### 3. Implementation Steps

#### Phase 1: Create New Module Structure
1. Create `src/gpu/buffer_layouts/` directory
2. Implement base module files with proper exports
3. Define common traits and utilities

#### Phase 2: Migrate Buffer Definitions
1. Move `VoxelData` from `world_buffer.rs` to `buffer_layouts/world.rs`
2. Move `InstanceData` structures to `buffer_layouts/instance.rs`
3. Move indirect command structures to `buffer_layouts/commands.rs`
4. Move camera data structures to `buffer_layouts/camera.rs`

#### Phase 3: Centralize Constants and Calculations
1. Extract all buffer size calculations
2. Define all binding indices in one place
3. Implement centralized offset calculation functions
4. Add alignment helper functions

#### Phase 4: Update Existing Code
1. Update imports to use new module
2. Replace hardcoded values with constants
3. Use centralized calculation functions
4. Remove duplicated definitions

#### Phase 5: Add Documentation
1. Create memory layout diagrams
2. Document alignment requirements
3. Add usage examples
4. Include performance considerations

### 4. Code Examples

#### Before:
```rust
// In world_buffer.rs
pub fn slot_offset(&self, slot: u32) -> u64 {
    slot as u64 * VOXELS_PER_CHUNK as u64 * std::mem::size_of::<VoxelData>() as u64
}

// In instance_buffer.rs
let buffer_size = (std::mem::size_of::<InstanceData>() * capacity as usize) as u64;
```

#### After:
```rust
// In buffer_layouts/calculations.rs
use super::constants::*;

pub fn calculate_chunk_slot_offset(slot: u32) -> u64 {
    slot as u64 * CHUNK_BUFFER_SLOT_SIZE
}

pub fn calculate_instance_buffer_size(capacity: u32) -> u64 {
    capacity as u64 * INSTANCE_DATA_SIZE
}

// Usage:
let offset = buffer_layouts::calculations::calculate_chunk_slot_offset(slot);
```

### 5. Benefits

1. **Single Source of Truth**: All buffer layouts in one place
2. **Type Safety**: Compile-time validation of buffer compatibility
3. **Maintainability**: Easy to update and extend
4. **Documentation**: Clear understanding of GPU memory layout
5. **Performance**: Optimized alignment and packing
6. **Debugging**: Easier to debug buffer-related issues

### 6. Migration Strategy

1. **Backward Compatibility**: Keep old definitions temporarily with deprecation warnings
2. **Incremental Migration**: Update one module at a time
3. **Testing**: Comprehensive tests for each migrated component
4. **Validation**: GPU validation layer testing

### 7. Future Enhancements

1. **Code Generation**: Generate WGSL struct definitions from Rust
2. **Runtime Validation**: Add debug assertions for buffer sizes
3. **Profiling Integration**: Track buffer usage statistics
4. **Dynamic Layouts**: Support for runtime-configurable layouts

## Timeline

- **Week 1**: Create new module structure and base utilities
- **Week 2**: Migrate core buffer definitions
- **Week 3**: Update all code references
- **Week 4**: Documentation and testing
- **Week 5**: Performance validation and cleanup

## Success Metrics

1. All buffer definitions centralized in one module
2. Zero duplicated buffer calculations
3. Consistent naming conventions throughout
4. Comprehensive documentation coverage
5. No performance regressions
6. Improved code maintainability scores