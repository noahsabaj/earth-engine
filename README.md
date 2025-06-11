# Earth Engine

**Version: 0.35.0** ([Versioning Strategy](docs/VERSIONING.md))

A state-of-the-art voxel game engine built with Rust, designed to push the boundaries of what's possible with voxel technology through data-oriented design and GPU-first architecture.

## 🚀 Features

- **12x faster** chunk generation through parallelization
- **GPU-first architecture** with data-oriented design
- **Thread-safe** concurrent systems
- **Modern rendering** with wgpu (Vulkan/DirectX/Metal)
- **Planet-scale worlds** with efficient streaming
- **Cross-platform** - Native (Windows/Linux/Mac) + Web (WebAssembly)

## 📋 Current Status

**Sprint 35 Complete** - Architecture Finalization ✅  
**Emergency Sprint Series 35.1-35.5** - In Planning 🚨

### Reality Check:
After completing Sprint 35, we conducted a comprehensive code audit that revealed significant gaps between our claims and reality. We are now entering an emergency sprint series focused on **engineering discipline and honesty**.

### What We Claimed vs Reality:
- **Claimed**: "Zero-allocation architecture" | **Reality**: 268 allocations per frame
- **Claimed**: "Complete DOP transition" | **Reality**: 228 files still have OOP patterns  
- **Claimed**: "Production ready" | **Reality**: 8.4% test coverage, 500+ unwrap() calls
- **Claimed**: "All features working" | **Reality**: ~5 features actually work

See:
- [MANIFESTO.md](MANIFESTO.md) - Our commitment to honesty and working code
- [RECOVERY_PLAN.md](RECOVERY_PLAN.md) - 10-week emergency sprint plan
- [docs/](docs/README.md) - All documentation (now organized!)
- [docs/status/CURRENT.md](docs/status/CURRENT.md) - Current emergency status
- [docs/MASTER_ROADMAP.md](docs/MASTER_ROADMAP.md) - Full development timeline

## ⚠️ Current State - Emergency Recovery Mode

### What Actually Works (Verified):
- ✅ Basic camera movement (with allocations)
- ✅ Simple chunk rendering (not optimized)
- ✅ Block placement (sometimes crashes)
- ✅ Some parallel systems (with race conditions)
- ✅ Shader compilation (when lucky)

### What Doesn't Work (Honest Truth):
- ❌ "Zero-allocation" - allocates 268 times per frame
- ❌ "Data-oriented" - 228 files still use OOP
- ❌ "Production ready" - panics on bad input
- ❌ "GPU-first" - most computation still on CPU
- ❌ Web implementation - abandoned due to no real value
- ❌ Most claimed optimizations - not actually implemented
- ❌ Test coverage - only 8.4%

### Emergency Sprint Plan (10 weeks):
1. **Weeks 1-2**: Remove all panic points, establish honest metrics
2. **Weeks 3-4**: Actually implement DOP, prove zero allocations
3. **Weeks 5-6**: Make core features work reliably
4. **Weeks 7-8**: Integration testing, real benchmarks
5. **Weeks 9-10**: B-grade certification, honest documentation

**Brutal Honesty**: We built impressive architecture but failed at basic engineering discipline. The next 10 weeks focus on making things actually work. No new features. No hype. Just engineering.

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

## 🚨 Emergency Sprints 35.1-35.5

**Sprint 35.1: Emergency Honesty & Stability** (Weeks 1-2)
- Replace all 500+ unwrap() calls with proper error handling
- Create comprehensive error types
- Document actual state vs claims
- Establish crash telemetry

**Sprint 35.2: DOP Reality Check** (Weeks 3-4)
- Convert 228 files from OOP to actual DOP
- Prove zero allocations with benchmarks
- Document DOP patterns clearly
- Memory profiling and verification

**Sprint 35.3: Core Systems That Work** (Weeks 5-6)
- Implement stable game loop
- Working player controller
- Reliable chunk loading
- Actual save/load functionality

**Sprint 35.4: Integration & Testing** (Weeks 7-8)
- Connect all systems properly
- Achieve 60% test coverage
- Real performance benchmarks
- Continuous integration that blocks bad code

**Sprint 35.5: B-Grade Certification** (Weeks 9-10)
- Complete API documentation
- Working example projects
- 1-hour stability demonstration
- Honest performance report

## 📈 Performance (Reality Check)

| System | Claimed | Actual | Honest Target |
|--------|---------|--------|---------------|
| Chunk Generation | 0.008s | 0.85s | 0.1s |
| Allocations/Frame | 0 | 268 | 0 |
| Test Coverage | 95% | 8.4% | 60% |
| OOP Files | 0 | 228 | 0 |
| Panic Points | 0 | 500+ | 0 |
| Working Features | 50+ | ~5 | 20+ |

**Note**: Previous performance claims were aspirational. These are real measurements.

## 🤝 Contributing

This is currently a personal project, but contributions are welcome! Please read the documentation and understand the data-oriented philosophy before contributing.

## 📄 License

[License information to be added]

---

Built with ❤️ and a commitment to **honest engineering**.

---

## 📢 Our Commitment

**"From Pretense to Performance"**

We choose:
- Tests over talk
- Proof over promises  
- Stability over speed
- Honesty over hype

Follow our journey from D-grade to B-grade execution in [docs/EMERGENCY_PROGRESS.md](docs/EMERGENCY_PROGRESS.md).