# Hearth Engine Development Philosophy

## The Three-Stage Architecture

### 1. ENGINE (The HOW) - Technical Foundation
- **Purpose**: Pure technical implementation of voxel world capabilities
- **Focus**: Performance, data structures, GPU computing, rendering
- **Philosophy**: Data-oriented, zero objects, shared buffers
- **Example**: "Here's how to render 100 million voxels at 144 FPS"
- **Status**: Sprints 1-38 focus on this layer

### 2. FRAMEWORK (The WHAT) - Game Systems Layer
- **Purpose**: Reusable game systems built on the engine
- **Focus**: Game mechanics, entity behaviors, world rules
- **Philosophy**: Still data-oriented, but game-specific patterns
- **Example**: "Here's what an inventory system looks like in our architecture"
- **Status**: To be developed after engine completion

### 3. GAME (The WHY) - Earth MMO Implementation
- **Purpose**: The actual game experience and content
- **Focus**: Physical information economy, handwriting, planetary servers
- **Philosophy**: Leverages framework to create unique gameplay
- **Example**: "Here's why information has physical weight in our world"
- **Status**: Final phase after framework

## Why This Order Matters

### 1. Clean Separation of Concerns
- Engine knows nothing about games
- Framework knows nothing about Earth MMO specifically
- Game can be swapped out without touching engine

### 2. Maximum Reusability
- Engine can power any voxel game
- Framework provides common patterns
- Multiple games could use same engine/framework

### 3. Performance First
- Optimize at the engine level once
- All games benefit from improvements
- No game-specific hacks polluting core

### 4. Easier Testing
- Engine can be benchmarked purely
- Framework can be tested generically
- Game testing focuses on gameplay

## Data-Oriented Design Throughout

### Engine Level (Current Focus)
```
WorldBuffer → GPU Compute → RenderBuffer
    ↑             ↑             ↑
    └─────────────┴─────────────┘
         Pure Data Flow
```

### Framework Level (Future)
```
EntityData → BehaviorKernel → EntityData'
    ↑             ↑              ↑
    └─────────────┴──────────────┘
      Still Pure Data
```

### Game Level (Far Future)
```
BookData → HandwritingKernel → RenderedText
    ↑             ↑                ↑
    └─────────────┴────────────────┘
    Game-Specific Data Flow
```

## Current Status

### ✅ Engine Phase (Sprints 1-45)
- Core systems: Complete through Sprint 29
- Optimizations: Sprints 27-29 done
- Remaining: Sprints 30-45 for final engine features
- Version 1.0: Sprint 45

### ❌ Framework Phase
- Not started
- Will include: Inventory, Crafting, Combat, AI, etc.
- All as data transformations, no objects

### ❌ Game Phase  
- Not started
- Will implement Earth MMO vision
- Physical information economy, etc.

## Key Principles

1. **No Objects, Ever**: Even in game phase, everything is data + kernels
2. **GPU First**: Every system asks "can GPU do this?"
3. **Zero Coupling**: Systems communicate through shared buffers only
4. **Cache Friendly**: Data layout matters more than code elegance
5. **Measure Everything**: Performance metrics drive decisions

## Success Metrics

### Engine Success = 
- 1000x faster than traditional engines
- Scales to planet-size worlds
- Runs in browser at full speed
- Supports 10,000+ players technically

### Framework Success =
- Common game features "just work"
- Modders can extend easily
- Still maintains performance
- Clean data-oriented patterns

### Game Success =
- Revolutionary gameplay emerges
- Physical information economy works
- Players create emergent stories
- Technical excellence enables new experiences