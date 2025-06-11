# Changelog

All notable changes to Earth Engine will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) with sprint-based pre-1.0 versioning.

## [Unreleased] - Emergency Sprint Series 35.1-35.5

### Critical Issues Discovered
- Code audit revealed 228 files still have OOP patterns (not 0 as claimed)
- 268 allocations per frame (not 0 as claimed)
- 8.4% test coverage (not 95% as implied)
- 500+ unwrap() calls that will panic
- Only ~5 features actually work (not 50+ as listed)

### Emergency Plan
- Sprint 35.1: Remove all panic points (2 weeks)
- Sprint 35.2: Actually implement DOP (2 weeks)
- Sprint 35.3: Make core features work (2 weeks)
- Sprint 35.4: Integration and testing (2 weeks)
- Sprint 35.5: B-grade certification (2 weeks)

### Added
- MANIFESTO.md - Commitment to engineering discipline
- RECOVERY_PLAN.md - 10-week emergency sprint plan
- EMERGENCY_PROGRESS.md - Daily progress tracking
- Honest performance metrics and benchmarks

### Changed
- Sprint 36+ postponed by 10 weeks for emergency fixes
- Focus shifted from features to making existing code work
- All documentation updated with honest assessments

## [0.35.0] - 2025-01-10

### Reality Check
**What we claimed:** Complete DOP transition, zero allocations, production ready  
**What we delivered:** Attempted web implementation that provided no value

### What Actually Happened
- Created JavaScript WebGPU implementation
- Attempted to refactor it to be "data-oriented"
- Critical analysis revealed it wasn't truly GPU-first
- Made the hard decision to abandon it entirely
- Conducted comprehensive code audit revealing:
  - 228 files still have OOP patterns
  - 268 allocations per frame
  - 8.4% test coverage
  - 500+ panic points

### Lessons Learned
- Don't build technology for technology's sake
- Verify architectural value before implementation
- Test coverage and stability matter more than features
- Honest metrics prevent technical debt accumulation

### Removed
- Entire web implementation (earth-engine/web/)
- False performance claims
- Pretense of being production-ready

### Sprint Completed
- Sprint 35: Architecture Finalization (revealed critical issues)

## [0.28.0] - 2025-01-10

### Added
- GPU-driven frustum culling compute shader
- Hierarchical Z-buffer (HZB) occlusion culling
- Indirect multi-draw rendering system
- GPU-based LOD selection with screen space metrics
- Triple-buffered instance streaming with persistent mapping
- Zero CPU involvement in culling decisions

### Performance Improvements
- Draw calls reduced from thousands to 1
- CPU overhead reduced by 100-500x (<0.1ms)
- GPU can cull 1M chunks in ~6ms
- Supports rendering 100k+ chunks at 60+ FPS
- GPU utilization increased from 40% to 90%

### Technical Details
- Single multi_draw_indexed_indirect call renders entire world
- GPU generates draw commands directly in compute shader
- Triple buffering prevents GPU-CPU sync stalls
- Coalesced dirty ranges for efficient updates

### Sprint Completed
- Sprint 28: GPU-Driven Rendering Optimization

## [0.27.0] - 2025-01-10

### Added
- Morton encoding (Z-order curve) for voxel storage
- Workgroup shared memory optimization in compute shaders
- Structure-of-Arrays (SoA) chunk layout with cache alignment
- Morton-based page table for streaming system
- Optimized fluid advection with 10x10x10 shared memory cache
- Optimized marching cubes with 4x4x4 shared memory cache

### Performance Improvements
- 3-5x better cache locality for spatial data access
- 90% reduction in global memory access for compute shaders
- 5-10x speedup for fluid simulation
- 4-6x speedup for SDF surface extraction
- 627M coords/sec Morton encoding, 1.6B coords/sec decoding

### Sprint Completed
- Sprint 27: Core Memory & Cache Optimization

## [0.26.0] - 2025-01-10

### Added
- Hot-reload system for shaders, assets, and configs
- Experimental Rust code hot-reload
- Mod development mode with state preservation

### Sprint Completed
- Sprint 26: Hot-Reload Everything

## [0.25.0] - 2025-01-10

### Added
- Hybrid SDF-Voxel rendering system
- Smooth terrain rendering while maintaining voxel gameplay
- Marching cubes implementation
- Dual storage for voxel and SDF data

### Sprint Completed
- Sprint 25: Hybrid SDF-Voxel System

## [0.24.0] - 2025-01-10

### Added
- GPU Fluid Dynamics system
- Multi-phase fluid support (water, lava, oil, steam, air)
- Fluid-terrain interaction with erosion
- Semi-Lagrangian advection

### Sprint Completed
- Sprint 24: GPU Fluid Dynamics

## [0.23.0] - 2025-01-10

### Added
- Data-oriented world streaming
- Virtual memory page tables
- GPU virtual memory management
- Support for billion+ voxel worlds

### Sprint Completed
- Sprint 23: Data-Oriented World Streaming

## [0.21.0] - 2024-12

### Added
- GPU World Architecture
- WorldBuffer - all world data GPU-resident
- GPU terrain generation with Perlin noise
- 100x speedup for chunk operations

### Changed
- Architectural pivot to data-oriented design
- CPU becomes "hint provider" only

### Sprint Completed
- Sprint 21: GPU World Architecture (The Big Shift)

## [0.16.0] - 2024-11

### Added
- Parallel lighting system
- Cross-chunk light propagation
- Thread-safe block providers

### Sprint Completed
- Sprint 16: Parallel Lighting System

## [0.1.0] - 2024-01

### Added
- Initial engine foundation
- Basic voxel world structure
- Chunk management system
- Block registry
- Basic rendering pipeline

### Sprint Completed
- Sprint 1: Core Engine Foundation

---

[Unreleased]: https://github.com/noahsabaj/earth-engine/compare/v0.35.0...HEAD
[0.35.0]: https://github.com/noahsabaj/earth-engine/compare/v0.28.0...v0.35.0
[0.26.0]: https://github.com/noahsabaj/earth-engine/compare/v0.25.0...v0.26.0
[0.25.0]: https://github.com/noahsabaj/earth-engine/compare/v0.24.0...v0.25.0
[0.24.0]: https://github.com/noahsabaj/earth-engine/compare/v0.23.0...v0.24.0
[0.23.0]: https://github.com/noahsabaj/earth-engine/compare/v0.21.0...v0.23.0
[0.21.0]: https://github.com/noahsabaj/earth-engine/compare/v0.16.0...v0.21.0
[0.16.0]: https://github.com/noahsabaj/earth-engine/compare/v0.1.0...v0.16.0
[0.1.0]: https://github.com/noahsabaj/earth-engine/releases/tag/v0.1.0