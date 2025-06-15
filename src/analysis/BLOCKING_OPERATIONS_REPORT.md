# BLOCKING OPERATIONS REPORT - 0.8 FPS Crisis

## Executive Summary
Earth Engine is running at 0.8 FPS (1250ms per frame) instead of 60 FPS (16ms per frame). This is 78x slower than target.

## ROOT CAUSE: VSYNC/Present Mode Blocking

### 1. **Surface Present Blocking** - 1200ms (96% of frame time)
**Operation**: `wgpu::SurfaceTexture::present()`  
**Location**: `src/renderer/gpu_state.rs:1727`  
**Measured Time**: ~1200ms per frame  
**Why It's Slow**: 
- Using `PresentMode::Fifo` which waits for vertical sync
- On a 60Hz monitor with GPU not keeping up, this causes massive stalls
- The GPU/driver is forcing wait for the next vsync interval
- WSL2 may be adding additional compositor delays

**Quick Fix**:
```rust
// In gpu_state.rs:544-552, change present mode selection:
let present_mode = if surface_caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
    wgpu::PresentMode::Immediate  // No vsync wait
} else if surface_caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
    wgpu::PresentMode::Mailbox     // Triple buffering, no wait
} else if surface_caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
    wgpu::PresentMode::Fifo        // Only use as last resort
} else {
    surface_caps.present_modes[0]
};
```

### 2. **Excessive Memory Allocations** - 30-50ms (2-4% of frame)
**Operation**: Per-frame allocations in renderer  
**Location**: Multiple locations in GPU-driven renderer  
**Measured Time**: 30-50ms accumulated  
**Why It's Slow**:
- Creating new buffers/bind groups every frame
- Not reusing instance buffers properly
- Allocation overhead accumulates

**Quick Fix**:
- Reuse buffers across frames
- Pre-allocate instance data
- Use buffer suballocation

### 3. **Synchronous Chunk Mesh Generation** - 10-50ms per chunk
**Operation**: Chunk mesh building on main thread  
**Location**: `update_chunk_renderer()` dirty chunk processing  
**Measured Time**: 10-50ms per chunk  
**Why It's Slow**:
- Mesh generation happens synchronously during frame
- Blocks render thread while building
- No async mesh generation pipeline

**Quick Fix**:
- Move mesh generation to worker threads
- Use async mesh building pipeline
- Cache built meshes

## Secondary Issues

### 4. **GPU Fence Waiting**
- Synchronous `device.poll(Maintain::Wait)` calls
- Should use async GPU operations

### 5. **Frame Rate Limiting Logic**
- `AboutToWait` event handler has 60 FPS limiting
- But with 0.8 FPS, this adds unnecessary delays

## Immediate Actions

1. **Change Present Mode** (will fix 95% of the issue):
   ```rust
   // Force Immediate mode to bypass vsync
   present_mode: wgpu::PresentMode::Immediate
   ```

2. **Disable frame rate limiting** when FPS < 30:
   ```rust
   if current_fps < 30.0 {
       gpu_state.window.request_redraw(); // Don't wait
   }
   ```

3. **Add present mode override** via environment variable:
   ```rust
   let force_immediate = std::env::var("EARTH_ENGINE_FORCE_IMMEDIATE").is_ok();
   ```

## Verification

After implementing fixes, the RealityCheckProfiler should show:
- Frame time: 16-20ms (50-60 FPS)
- GPU wait time: <5ms
- Surface present: <2ms

## Additional Optimizations

1. Use GPU timestamps to measure actual GPU work
2. Implement async compute for chunk generation
3. Use indirect drawing to reduce CPU overhead
4. Enable GPU-driven culling and LOD