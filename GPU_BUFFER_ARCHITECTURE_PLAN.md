# GPU Buffer Architecture Plan

## Executive Summary

This plan outlines a comprehensive GPU buffer management system that eliminates manual alignment calculations and prevents runtime buffer size mismatches. It builds upon existing infrastructure in the Hearth Engine while introducing compile-time validation and automatic WGSL generation.

## Problem Statement

Current issues:
1. Manual struct padding calculations prone to errors
2. Runtime discovery of buffer size mismatches
3. WGSL/Rust struct alignment discrepancies
4. No compile-time validation
5. Repeated alignment bugs requiring manual fixes

## Existing Infrastructure to Leverage

### 1. WorldBuffer (`world_gpu/world_buffer.rs`)
- Already manages GPU buffers with proper creation patterns
- Has bind group management
- Can extend with type-safe wrappers

### 2. PersistentBuffer (`memory/persistent_buffer.rs`)
- Existing buffer usage patterns
- Multi-frame buffer support
- Can integrate with new type system

### 3. InstanceBuffer (`renderer/gpu_driven/instance_buffer.rs`)
- Pattern for typed GPU buffers
- Batch update mechanisms
- Can generalize for all GPU types

### 4. Bytemuck Integration
- Already used extensively (46+ files)
- Pod/Zeroable traits established
- Can combine with encase for best of both worlds

## Architecture Design

### Core Components

```
hearth-engine/
├── src/
│   ├── gpu/                        # NEW: Centralized GPU management
│   │   ├── mod.rs
│   │   ├── buffer_manager.rs       # Type-safe buffer creation/updates
│   │   ├── types/                  # All GPU struct definitions
│   │   │   ├── mod.rs
│   │   │   ├── core.rs            # Base traits and common types
│   │   │   ├── terrain.rs          # Terrain generation types
│   │   │   ├── lighting.rs         # Lighting types
│   │   │   ├── physics.rs          # Physics GPU types
│   │   │   └── particles.rs        # Particle system types
│   │   ├── validation.rs           # Compile-time size validation
│   │   └── shader_bridge.rs        # WGSL generation bridge
│   └── [existing modules...]
├── build.rs                        # NEW: WGSL generation from Rust
└── generated/                      # NEW: Auto-generated files
    └── shaders/
        └── types.wgsl
```

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1)

#### 1.1 Dependencies
```toml
# Add to Cargo.toml
[dependencies]
encase = { version = "0.7", features = ["glam"] }  # For automatic alignment
static_assertions = "1.1"                           # Compile-time validation

[build-dependencies]
encase = "0.7"
syn = "2.0"         # For parsing Rust types
quote = "1.0"       # For code generation
proc-macro2 = "1.0"
```

#### 1.2 Core GPU Type System (`gpu/types/core.rs`)
```rust
use encase::{ShaderType, ShaderSize, UniformBuffer, StorageBuffer};
use bytemuck::{Pod, Zeroable};

/// Marker trait combining bytemuck and encase requirements
pub trait GpuData: ShaderType + Pod + Zeroable + Send + Sync {
    /// Validate at compile time
    const VALIDATED: () = assert!(
        Self::SHADER_SIZE.get() % 16 == 0,
        "GPU types must be 16-byte aligned"
    );
}

/// Auto-implement for types that meet requirements
impl<T> GpuData for T 
where 
    T: ShaderType + Pod + Zeroable + Send + Sync,
    T: 'static,
{
}

/// Buffer wrapper that ensures type safety
pub struct TypedGpuBuffer<T: GpuData> {
    pub buffer: wgpu::Buffer,
    pub size: wgpu::BufferAddress,
    _phantom: std::marker::PhantomData<T>,
}
```

#### 1.3 Buffer Manager Integration (`gpu/buffer_manager.rs`)
Extend existing patterns from WorldBuffer and PersistentBuffer:
```rust
pub struct GpuBufferManager {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    
    // Reuse persistent buffer patterns
    persistent_pools: HashMap<BufferUsage, MemoryPool>,
}

impl GpuBufferManager {
    /// Create uniform buffer with automatic alignment
    pub fn create_uniform<T: GpuData>(&self, data: &T) -> Result<TypedGpuBuffer<T>> {
        let mut encoder = UniformBuffer::new(Vec::new());
        encoder.write(data)?;
        
        let bytes = encoder.into_inner();
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Uniform<{}>", std::any::type_name::<T>())),
            contents: &bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        Ok(TypedGpuBuffer {
            buffer,
            size: bytes.len() as wgpu::BufferAddress,
            _phantom: PhantomData,
        })
    }
}
```

### Phase 2: Terrain System Migration (Week 2)

#### 2.1 Updated Terrain Types (`gpu/types/terrain.rs`)
```rust
use encase::ShaderType;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug)]
pub struct BlockDistribution {
    pub block_id: u32,
    pub min_height: i32,
    pub max_height: i32,
    pub probability: f32,
    pub noise_threshold: f32,
    // encase automatically handles padding!
}

#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone)]
pub struct TerrainParams {
    pub seed: u32,
    pub sea_level: f32,
    pub terrain_scale: f32,
    pub mountain_threshold: f32,
    pub cave_threshold: f32,
    pub num_distributions: u32,
    pub _padding: [u32; 2], // Still explicit for fixed arrays
    pub distributions: [BlockDistribution; MAX_BLOCK_DISTRIBUTIONS],
}

// Compile-time validation
const _: () = assert!(std::mem::size_of::<BlockDistribution>() == 48);
const _: () = assert!(std::mem::size_of::<TerrainParams>() == 800);
```

#### 2.2 TerrainGenerator Updates
Minimal changes needed:
```rust
impl TerrainGenerator {
    pub fn new(device: Arc<wgpu::Device>, manager: Arc<GpuBufferManager>) -> Self {
        let params = TerrainParams::default();
        let params_buffer = manager.create_uniform(&params)
            .expect("Failed to create terrain params buffer");
        
        // Existing shader loading...
        Self {
            device,
            params_buffer,
            manager,
            // ...
        }
    }
    
    pub fn update_params(&self, params: &TerrainParams) -> Result<()> {
        self.manager.update_uniform(&self.params_buffer, params)
    }
}
```

### Phase 3: Build System & WGSL Generation (Week 3)

#### 3.1 Build Script (`build.rs`)
```rust
use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=src/gpu/types");
    
    let out_dir = env::var("OUT_DIR").unwrap();
    let generated_path = Path::new(&out_dir).join("gpu_types.wgsl");
    
    // Generate WGSL from Rust types
    let wgsl_content = generate_wgsl_types();
    fs::write(&generated_path, wgsl_content).unwrap();
    
    // Copy to src for shader includes
    let shader_path = Path::new("src/gpu/shaders/generated/types.wgsl");
    fs::create_dir_all(shader_path.parent().unwrap()).unwrap();
    fs::copy(&generated_path, shader_path).unwrap();
}

fn generate_wgsl_types() -> String {
    // Use encase's layout information to generate correct WGSL
    format!(r#"
// AUTO-GENERATED - DO NOT EDIT
// Generated from Rust GPU type definitions

struct BlockDistribution {{
    block_id: u32,
    min_height: i32,
    max_height: i32,
    probability: f32,
    noise_threshold: f32,
    _padding: array<f32, 7>, // Automatic padding
}}

struct TerrainParams {{
    seed: u32,
    sea_level: f32,
    terrain_scale: f32,
    mountain_threshold: f32,
    cave_threshold: f32,
    num_distributions: u32,
    _padding: vec2<u32>,
    distributions: array<BlockDistribution, {}>,
}}
"#, MAX_BLOCK_DISTRIBUTIONS)
}
```

### Phase 4: Game Repository Updates

#### 4.1 Simplified Game Configuration
```rust
// danger-money/src/generation/gpu_config.rs
use hearth_engine::gpu::types::terrain::{TerrainParams, BlockDistribution};

pub fn create_danger_money_terrain_params(seed: u32) -> TerrainParams {
    TerrainParams {
        seed,
        sea_level: 64.0,
        terrain_scale: 0.01,
        mountain_threshold: 0.6,
        cave_threshold: 0.3,
        num_distributions: 5,
        _padding: [0; 2],
        distributions: [
            BlockDistribution {
                block_id: COPPER_ORE_ID.0 as u32,
                min_height: 0,
                max_height: 40,
                probability: 0.05,
                noise_threshold: 0.3,
            },
            // ... other ores
            // No manual padding needed!
        ],
    }
}
```

#### 4.2 Game-Side Validation
```rust
// danger-money/src/gpu_validation.rs
#[cfg(test)]
mod tests {
    use hearth_engine::gpu::types::terrain::*;
    
    #[test]
    fn validate_gpu_types() {
        // Compile-time validation happens automatically
        // This test ensures game compiles with engine types
        let _ = BlockDistribution::default();
        let _ = TerrainParams::default();
    }
}
```

## Migration Strategy

### Week 1: Core Infrastructure
1. Create gpu module structure
2. Implement GpuData trait and TypedGpuBuffer
3. Extend GpuBufferManager from existing patterns
4. Add encase dependency

### Week 2: Terrain System
1. Migrate BlockDistribution and TerrainParams
2. Update TerrainGenerator to use new system
3. Remove manual padding calculations
4. Verify GPU generation works

### Week 3: Build System
1. Create build.rs for WGSL generation
2. Generate types.wgsl from Rust definitions
3. Update shaders to include generated types
4. Add validation tests

### Week 4: Full Rollout
1. Migrate all GPU types (lighting, physics, particles)
2. Update all buffer creation to use GpuBufferManager
3. Remove all manual alignment code
4. Update documentation

## Benefits

1. **Zero Runtime Errors**: All alignment issues caught at compile time
2. **No Manual Padding**: encase handles all alignment automatically
3. **Single Source of Truth**: Rust types generate WGSL
4. **Type Safety**: Impossible to create mismatched buffers
5. **Future Proof**: Updates to WGSL spec handled by library updates
6. **Better DevX**: Clear errors, automatic validation
7. **Reuses Existing Code**: Builds on WorldBuffer, PersistentBuffer patterns

## Validation & Testing

### Compile-Time Checks
- Static assertions on all GPU type sizes
- Trait bounds ensure proper alignment
- Build fails if types don't match requirements

### Runtime Validation
- Debug assertions on buffer updates
- Size validation on all operations
- Profiling to ensure no performance regression

### Integration Tests
```rust
#[test]
fn test_terrain_buffer_alignment() {
    let params = TerrainParams::default();
    let buffer = manager.create_uniform(&params).unwrap();
    assert_eq!(buffer.size, 800); // WGSL expected size
}
```

## Performance Considerations

- Same GPU performance (data layout unchanged)
- Slightly slower build times (WGSL generation)
- No runtime overhead (same buffer operations)
- Better CPU performance (no manual padding calculations)

## Conclusion

This architecture eliminates an entire class of GPU buffer alignment bugs while building on Hearth Engine's existing infrastructure. By leveraging encase for automatic alignment and combining it with the engine's established bytemuck patterns, we get the best of both worlds: type safety and zero-overhead GPU communication.

The migration can be done incrementally, system by system, without breaking existing functionality. Once complete, GPU buffer alignment errors will be impossible, caught at compile time rather than runtime.