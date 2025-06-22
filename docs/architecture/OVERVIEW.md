# Architecture Overview

## Current State
- 100% Data-Oriented Programming (DOP) architecture
- Zero allocations per frame in steady-state gameplay
- GPU-first design with minimal CPU overhead
- 16 unwrap() calls (mostly in initialization code)
- Pre-allocated memory pools for all hot paths

## Core Architecture Achievements

### Data-Oriented Design
- All game logic uses pure functions and data structures
- No OOP inheritance or virtual dispatch
- Component data stored in contiguous arrays
- Cache-friendly memory layouts throughout
See DATA_ORIENTED_ARCHITECTURE.md for principles

### GPU-First Rendering
- GPU-driven culling and LOD selection
- Hierarchical Z-Buffer (HZB) occlusion culling
- Indirect draw calls with GPU-generated command buffers
- Zero CPU involvement in visibility determination
See GPU_DRIVEN_ARCHITECTURE.md for GPU-first design

### Spatial Indexing
- Morton encoding for cache-efficient spatial queries
- GPU-accelerated spatial lookups
- Hierarchical chunk organization
- Lock-free concurrent access patterns
See SPATIAL_INDEX_ARCHITECTURE.md for indexing

### Memory Management
- Pre-allocated buffer pools for all dynamic data
- Ring buffer allocation for transient data
- GPU memory staging with zero-copy paths
- Predictable memory usage patterns

## Key Systems

### World Management
- Unified world system with GPU-CPU coherence
- Chunk-based terrain with adaptive LOD
- Streaming architecture for infinite worlds
See WORLD_GUIDE.md for world system details

### Callback System
- Replaced OOP Gateway pattern with DOP callbacks
- Pure function pointers for game integration
- Zero-overhead extensibility
See DOP_CALLBACK_SYSTEM.md for callback architecture

For implementation guides, see docs/guides/