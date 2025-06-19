<div align="center">
  <img src="logo.png" alt="Hearth Engine Logo" width="200"/>
  
  # Hearth Engine
</div>

A high-performance voxel game engine built with Rust, designed for modern GPU architectures and data-oriented programming.

## Features

- **GPU-first architecture** with compute shader generation
- **Data-oriented design** for optimal cache efficiency
- **Cross-platform rendering** via wgpu (Vulkan/DirectX/Metal)
- **Unified world system** with GPU-resident data
- **8-phase GPU automation** eliminating manual GPU operations
- **Planet-scale worlds** with efficient chunk streaming

## Quick Start

```bash
# Clone and build
git clone https://github.com/noahsabaj/hearth-engine.git
cd hearth-engine
cargo build --release

# Run the engine testbed
cargo run --example engine_testbed

# Run tests
cargo test
```

## Architecture

Hearth Engine follows strict data-oriented programming principles:
- No classes or OOP patterns
- Data lives in shared GPU/CPU buffers
- Systems are stateless compute kernels
- GPU-first computation model

### Key Systems

- **World**: Voxel storage and generation
- **Renderer**: GPU-driven rendering with automatic culling
- **Physics**: Parallel collision detection and response
- **GPU Automation**: Automatic shader generation and binding management

## Using as a Library

```toml
[dependencies]
hearth-engine = { path = "../hearth-engine" }
```

```rust
use hearth_engine::{Engine, EngineConfig};

fn main() {
    let config = EngineConfig::default();
    let engine = Engine::new(config);
    engine.run(MyGame).expect("Engine failed");
}
```

## Documentation

- [Architecture Overview](docs/architecture/OVERVIEW.md)
- [Data-Oriented Design Guide](docs/guides/DATA_ORIENTED_PROGRAMMING.md)
- [GPU Programming Guide](docs/architecture/GPU_DRIVEN_ARCHITECTURE.md)
- [API Documentation](https://docs.rs/hearth-engine)

## Performance

- Chunk generation: < 100ms
- Frame time: < 16ms (60+ FPS)
- Zero runtime allocations in hot paths
- GPU-resident data eliminates CPU/GPU sync

## License

Copyright © 2025 Noah Sabaj. All rights reserved.

This software is proprietary and confidential. See [LICENSE](LICENSE) for details.

For licensing inquiries, contact Noah Sabaj.