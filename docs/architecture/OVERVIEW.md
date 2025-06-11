# Architecture Overview

## Current Reality
- 228 files still using OOP (not DOP as claimed)
- 268 allocations per frame (not zero)
- Most "GPU-first" code runs on CPU

## Data-Oriented Design (Goal)
See DATA_ORIENTED_ARCHITECTURE.md for principles

## GPU Architecture  
See GPU_DRIVEN_ARCHITECTURE.md for GPU-first design

## Spatial Systems
See SPATIAL_INDEX_ARCHITECTURE.md for indexing

## Technical Debt
- 350 unwrap() calls remaining
- 12 unsafe blocks undocumented
- Missing bounds checking everywhere

For implementation guides, see docs/guides/