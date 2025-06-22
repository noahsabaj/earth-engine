# Hearth Engine Documentation

## Quick Links

- [Project Status](status/PROJECT_STATUS.md) - Current state and metrics
- [Master Roadmap](MASTER_ROADMAP.md) - Full development timeline  
- [Architecture Overview](architecture/OVERVIEW.md) - Technical architecture
- [Vision](vision/HEARTH_ENGINE_VISION.md) - Project vision and philosophy

## Documentation Structure

### `/architecture`
Technical architecture documentation
- [Overview](architecture/OVERVIEW.md) - Current architecture state
- [Data-Oriented Design](architecture/DATA_ORIENTED_DESIGN.md) - Complete DOP architecture
- [GPU-Driven Architecture](architecture/GPU_DRIVEN_ARCHITECTURE.md) - GPU-first design
- [Spatial Index Architecture](architecture/SPATIAL_INDEX_ARCHITECTURE.md) - Spatial systems with Morton encoding
- [Physics Data Layout](architecture/PHYSICS_DATA_LAYOUT.md) - Physics architecture
- [DOP Callback System](architecture/DOP_CALLBACK_SYSTEM.md) - Pure function callbacks replacing Gateway

### `/guides`  
Developer guides and how-tos
- [Data-Oriented Programming](guides/DATA_ORIENTED_PROGRAMMING.md) - DOP principles and enforcement
- [Git Setup](guides/GIT_SETUP_INSTRUCTIONS.md) - Repository setup
- [Cargo Commands](guides/CARGO_COMMANDS_GUIDE.md) - Build and test commands
- [Documentation Guide](guides/DOCUMENTATION_GUIDE.md) - How to write docs
- [World Guide](WORLD_GUIDE.md) - World system architecture
- [WGSL Shader Guide](WGSL_SHADER_GUIDE.md) - Shader development

### `/sprints`
Sprint documentation
- [Sprint History](sprints/SPRINT_HISTORY.md) - Consolidated sprint 12-34 history
- [Sprint 35 Complete](sprints/SPRINT_35_COMPLETE.md) - Architecture finalization
- [Sprint 37 Complete](sprints/SPRINT_37_COMPLETE.md) - Zero-allocation achievement
- Recent integration reports and assessments

### `/status`
Project status tracking
- [Project Status](status/PROJECT_STATUS.md) - Comprehensive status, metrics, and assessments
- [Changelog](status/CHANGELOG.md) - Version history

### `/vision`
Project vision and philosophy
- [Hearth Engine Vision](vision/HEARTH_ENGINE_VISION.md) - Complete vision document

### `/performance`
Performance documentation
- [Performance Audit](performance/PERFORMANCE_AUDIT.md) - Complete performance analysis

### `/audits`
System audits
- [Phase 1 Engine Audit](audits/PHASE_1_ENGINE_AUDIT.md) - Initial audit results

## Key Achievements

✅ **Pure Data-Oriented Architecture** - Zero objects, only data transformations
✅ **10x Performance Improvement** - Verified through extensive benchmarking
✅ **Zero Runtime Allocations** - Achieved in Sprint 37
✅ **Linear Scaling** - Up to 32 cores tested
✅ **GPU-First Design** - 89% GPU utilization

## Current Focus

The engine has achieved its architectural goals and now operates at theoretical hardware limits. Future work focuses on:
- Neural architecture explorations
- Multi-GPU support
- Advanced optimization techniques
- New feature development within DOP constraints