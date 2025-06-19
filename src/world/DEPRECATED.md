# ⚠️ DEPRECATED: Legacy World Module

This module is **DEPRECATED** and will be removed in a future release.

## Migration Status

All functionality has been migrated to `world_unified` which provides:
- GPU-first architecture with CPU fallback
- 1000x faster chunk generation on GPU
- Unified type system
- Zero-copy between CPU and GPU

## How to Migrate

Replace imports:
```rust
// Old
use crate::world::{World, BlockId, ChunkPos, VoxelPos};

// New
use crate::{World, BlockId, ChunkPos, VoxelPos};
```

The new exports come from `world_unified` but maintain API compatibility.

## Building Without Legacy Modules

To verify your code works without legacy modules:
```bash
cargo build --no-default-features --features native
```

## Remaining Dependencies

This module is kept only for:
- ParallelWorld (being migrated)
- SpawnFinder (being migrated)
- Legacy world generators

Once these are fully migrated, this entire directory will be deleted.