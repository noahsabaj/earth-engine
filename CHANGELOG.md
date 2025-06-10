# Changelog

All notable changes to Earth Engine will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) with sprint-based pre-1.0 versioning.

## [Unreleased]

### Added
- Sprint 38: HybridGPUGrid networking concept

### Changed
- Updated versioning strategy to be more honest about pre-release status
- Removed hard 1.0 claims from roadmap

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

[Unreleased]: https://github.com/noahsabaj/earth-engine/compare/v0.26.0...HEAD
[0.26.0]: https://github.com/noahsabaj/earth-engine/compare/v0.25.0...v0.26.0
[0.25.0]: https://github.com/noahsabaj/earth-engine/compare/v0.24.0...v0.25.0
[0.24.0]: https://github.com/noahsabaj/earth-engine/compare/v0.23.0...v0.24.0
[0.23.0]: https://github.com/noahsabaj/earth-engine/compare/v0.21.0...v0.23.0
[0.21.0]: https://github.com/noahsabaj/earth-engine/compare/v0.16.0...v0.21.0
[0.16.0]: https://github.com/noahsabaj/earth-engine/compare/v0.1.0...v0.16.0
[0.1.0]: https://github.com/noahsabaj/earth-engine/releases/tag/v0.1.0