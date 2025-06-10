# Earth Engine

**Version: 0.26.0** ([Versioning Strategy](docs/VERSIONING.md))

A state-of-the-art voxel game engine built with Rust, designed to push the boundaries of what's possible with voxel technology through data-oriented design and GPU-first architecture.

## 🚀 Features

- **12x faster** chunk generation through parallelization
- **GPU-first architecture** with data-oriented design
- **Thread-safe** concurrent systems
- **Modern rendering** with wgpu (Vulkan/DirectX/Metal)
- **Planet-scale worlds** with efficient streaming
- **Cross-platform** - Native (Windows/Linux/Mac) + Web (WebAssembly)

## 📋 Current Status

**Sprint 26 Complete** - Hot-Reload Everything ✅

### Recent Achievements:
- GPU Fluid Dynamics with multi-phase support (Sprint 24)
- Hybrid SDF-Voxel smooth terrain rendering (Sprint 25)
- Complete hot-reload system for rapid development (Sprint 26)
- GPU World Architecture with 100x speedup (Sprint 21)

See [docs/MASTER_ROADMAP.md](docs/MASTER_ROADMAP.md) for full development timeline.

## ⚠️ Current State - Pre-Alpha

### What Works:
- ✅ Basic voxel world with chunk generation
- ✅ Block placement and breaking
- ✅ Camera movement and controls
- ✅ GPU-accelerated terrain generation
- ✅ Parallel processing systems
- ✅ Hot-reload for shaders and assets
- ✅ Foundation for advanced features

### What's In Progress:
- 🚧 WebGPU support (Sprint 22)
- 🚧 Critical performance optimizations (Sprints 27-29)
- 🚧 Legacy system migration (Sprint 33)
- 🚧 Multiplayer functionality

### What's Planned but Not Started:
- ❌ The revolutionary gameplay (physical information economy)
- ❌ Planet-scale worlds in practice
- ❌ 10,000 concurrent players
- ❌ Most advanced game features

**Honest Assessment**: The engine has impressive technical foundations but is 6-12 months from a true 1.0 release. Consider this a technical preview of revolutionary architecture, not a complete game engine.

## 🏗️ Architecture

Earth Engine uses a revolutionary data-oriented architecture where:
- Systems don't know about each other - they read/write shared memory
- GPU computes stay on GPU - no unnecessary transfers
- "The best system is no system" philosophy

See [docs/DATA_ORIENTED_TRANSITION_PLAN.md](docs/DATA_ORIENTED_TRANSITION_PLAN.md) for details.

## 🎮 Vision

Building the first voxel engine that truly uses modern hardware. Not competing with Minecraft - competing with what Minecraft COULD have been if built in 2025.

See [docs/EARTH_ENGINE_VISION_2025.md](docs/EARTH_ENGINE_VISION_2025.md) for the revolutionary game design.

## 🛠️ Development

### Setup
```bash
# Clone the repository
git clone https://github.com/yourusername/earth-engine.git
cd earth-engine

# Build the engine
cargo build --release

# Run tests
cargo test

# Run GPU detection test
cargo run --bin gpu_test
```

### Development Workflow
1. Develop in WSL/Linux environment
2. Sync to Windows for GPU testing using `sync_to_windows.sh`
3. See [docs/ENVIRONMENT_COHERENCE.md](docs/ENVIRONMENT_COHERENCE.md) for details

## 📚 Documentation

- [docs/MASTER_ROADMAP.md](docs/MASTER_ROADMAP.md) - Complete development roadmap (Sprints 1-38)
- [docs/ENGINE_VISION.md](docs/ENGINE_VISION.md) - High-level vision and unique selling points
- [docs/EARTH_ENGINE_VISION_2025.md](docs/EARTH_ENGINE_VISION_2025.md) - Revolutionary game design vision
- [docs/DATA_ORIENTED_TRANSITION_PLAN.md](docs/DATA_ORIENTED_TRANSITION_PLAN.md) - Architecture transition strategy
- [docs/GPU_DRIVEN_ARCHITECTURE.md](docs/GPU_DRIVEN_ARCHITECTURE.md) - GPU-first design principles

## 🎯 Next Sprints

**Sprint 27: Core Memory & Cache Optimization**
- Morton encoding for 3-5x memory bandwidth improvement
- Workgroup shared memory in compute shaders
- Cache line alignment optimization

**Sprint 28: GPU-Driven Rendering Optimization**
- Single indirect draw call for entire world
- GPU frustum and occlusion culling
- Zero CPU intervention in render loop

**Sprint 29: Mesh Optimization & Advanced LOD**
- Greedy meshing for 10-100x triangle reduction
- Progressive LOD with smooth transitions
- GPU-accelerated mesh generation

## 📈 Performance

| System | Original | Current | Post-Optimization Target |
|--------|----------|---------|--------------------------|
| Chunk Generation | 10.40s | 0.008s (1300x) | 0.002s (5200x) |
| Mesh Building | 2.89s | 0.005s (580x) | 0.0005s (5800x) |
| Lighting | N/A | 0.003s (100x) | 0.0003s (1000x) |
| Fluid Simulation | N/A | 0.1s/step | 0.01s/step (10x) |
| Draw Calls | 5000 | 100 | 1 (5000x) |
| Network Players | 100 | 100 | 10,000 (100x) |

## 🤝 Contributing

This is currently a personal project, but contributions are welcome! Please read the documentation and understand the data-oriented philosophy before contributing.

## 📄 License

[License information to be added]

---

Built with ❤️ and a belief that software can be 100x faster through better thinking.