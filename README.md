# Earth Engine

A state-of-the-art voxel game engine built with Rust, designed to push the boundaries of what's possible with voxel technology through data-oriented design and GPU-first architecture.

## 🚀 Features

- **12x faster** chunk generation through parallelization
- **GPU-first architecture** with data-oriented design
- **Thread-safe** concurrent systems
- **Modern rendering** with wgpu (Vulkan/DirectX/Metal)
- **Planet-scale worlds** with efficient streaming
- **Cross-platform** - Native (Windows/Linux/Mac) + Web (WebAssembly)

## 📋 Current Status

**Sprint 16 Complete** - Parallel Lighting System ✅

See [MASTER_ROADMAP.md](MASTER_ROADMAP.md) for full development timeline.

## 🏗️ Architecture

Earth Engine uses a revolutionary data-oriented architecture where:
- Systems don't know about each other - they read/write shared memory
- GPU computes stay on GPU - no unnecessary transfers
- "The best system is no system" philosophy

See [DATA_ORIENTED_TRANSITION_PLAN.md](DATA_ORIENTED_TRANSITION_PLAN.md) for details.

## 🎮 Vision

Building the first voxel engine that truly uses modern hardware. Not competing with Minecraft - competing with what Minecraft COULD have been if built in 2025.

See [EARTH_ENGINE_VISION_2025.md](EARTH_ENGINE_VISION_2025.md) for the revolutionary game design.

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
3. See [ENVIRONMENT_COHERENCE.md](ENVIRONMENT_COHERENCE.md) for details

## 📚 Documentation

- [MASTER_ROADMAP.md](MASTER_ROADMAP.md) - Complete development roadmap (Sprints 1-34)
- [ENGINE_VISION.md](ENGINE_VISION.md) - High-level vision and unique selling points
- [EARTH_ENGINE_VISION_2025.md](EARTH_ENGINE_VISION_2025.md) - Revolutionary game design vision
- [DATA_ORIENTED_TRANSITION_PLAN.md](DATA_ORIENTED_TRANSITION_PLAN.md) - Architecture transition strategy

## 🎯 Next Sprint

**Sprint 17: Performance & Data Layout Analysis**
- Profile with focus on cache misses
- Introduce struct-of-arrays layouts
- Begin data-oriented transition

## 📈 Performance

| System | Original | Current | Target |
|--------|----------|---------|--------|
| Chunk Generation | 10.40s | 0.85s (12x) | 0.008s (1300x) |
| Mesh Building | 2.89s | 0.55s (5x) | 0.005s (580x) |
| Lighting | N/A | 0.30s | 0.003s (100x) |

## 🤝 Contributing

This is currently a personal project, but contributions are welcome! Please read the documentation and understand the data-oriented philosophy before contributing.

## 📄 License

[License information to be added]

---

Built with ❤️ and a belief that software can be 100x faster through better thinking.