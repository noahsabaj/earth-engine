# ⚠️ DEPRECATED: Legacy World_GPU Module

This module is **DEPRECATED** and has been fully integrated into `world_unified`.

## Why Deprecated?

The `world_gpu` module was a transitional step between the CPU-only `world` module and the unified architecture. All GPU functionality now lives in `world_unified/compute/`.

## Migration Complete

✅ WorldBuffer → world_unified/storage/WorldBuffer
✅ GpuLightPropagator → world_unified/compute/effects
✅ TerrainGeneratorSOA → world_unified/generation/terrain_gpu
✅ All GPU kernels → world_unified/compute/kernels

## Current Usage

This module is only referenced by 3 files:
- gpu_state.rs (via GpuLightPropagator re-export)
- Legacy generator imports

Once these references are updated, this directory will be deleted entirely.

## DO NOT ADD NEW CODE HERE

All new GPU functionality should be added to `world_unified/compute/`.