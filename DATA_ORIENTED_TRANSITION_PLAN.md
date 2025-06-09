# Data-Oriented Transition Plan

## Strategy: Ship of Theseus
Gradually replace components while keeping the engine functional at all times.

## Key Insight: Sprint 21 is the Pivot
- Before Sprint 21: Laying groundwork, learning concepts
- Sprint 21: Build the new world on GPU
- After Sprint 21: Everything new is data-oriented

## Transition Timeline

### Pre-Pivot Foundation (Sprints 17-20)
**Goal**: Introduce data-oriented concepts without breaking existing code

- **Sprint 17**: Learn by profiling - see where cache misses hurt
- **Sprint 18**: Physics as data tables - practice the new thinking
- **Sprint 19**: Spatial hashing - already naturally data-oriented
- **Sprint 20**: GPU-driven rendering - GPU starts making decisions

### The Pivot (Sprint 21)
**Goal**: Establish the new architecture alongside the old

- Build complete WorldBuffer system on GPU
- All NEW chunks use this system
- OLD chunks continue using CPU path
- Both systems run in parallel
- This is our "proof of concept" in production

### Post-Pivot Development (Sprints 22-29)
**Goal**: All new features are data-oriented from birth

- **Sprint 22**: WebGPU version is pure data-oriented (no legacy)
- **Sprint 23**: Streaming built on buffers, not objects
- **Sprints 24-29**: Every feature uses WorldBuffer

### Migration Phase (Sprints 30-32)
**Goal**: Remove the old architecture

- **Sprint 30**: Migrate existing chunks to GPU
- **Sprint 31**: Unify all systems into one kernel
- **Sprint 32**: Delete all OOP code

## Why This Works

1. **Always Shippable**: Every sprint produces a working engine
2. **Gradual Learning**: Team learns data-oriented design over time
3. **Proof Points**: Can benchmark old vs new at each step
4. **Risk Mitigation**: Problems found early, not after rewrite

## Success Metrics Per Sprint

- Sprint 17: 20% reduction in cache misses
- Sprint 18: Physics 5x faster
- Sprint 20: 50% reduction in draw calls
- Sprint 21: New chunks generate 100x faster
- Sprint 30: Entire world runs 100x faster

## The North Star Throughout

**"The best system is no system"**

Every decision should move us toward:
- Fewer abstractions
- More direct data access
- GPU doing more work
- CPU doing less work

## Critical Success Factors

1. **Document everything** - This is pioneering work
2. **Benchmark obsessively** - Prove the gains
3. **Stay disciplined** - Don't slip back to OOP comfort
4. **Celebrate wins** - Each sprint will show massive gains

## The Beautiful Truth

We're not adding complexity - we're removing it. Each sprint makes the engine simpler, faster, and more powerful.

By Sprint 34, we'll have built what Minecraft could have been with 2025 knowledge.