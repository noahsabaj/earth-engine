# Sprint 35.2 - Critical Performance and Rendering Fixes

## Issues Identified and Fixed

### 1. Player Spawning Inside Terrain
**Problem**: Player was spawning inside mountains despite spawn finder implementation
**Root Cause**: Coordinate system mismatch - verify_spawn_position was checking blocks at physics body center instead of feet position
**Fix**: 
- Updated spawn height offset from 5 to 10 blocks above surface
- Fixed verify_spawn_position to account for 0.9m offset from body center to feet
- Updated debug logging to show correct player positioning

### 2. CPU Running Hot ("Jet Engine" Mode)
**Problem**: Computer running at 100% CPU despite GPU optimization claims
**Root Cause**: Busy-wait loop in event handling - `window.request_redraw()` called unconditionally on every `Event::AboutToWait`
**Fix**: 
- Added frame rate limiting to target 60 FPS
- Only request redraw when sufficient time has elapsed
- Added `last_frame_time` tracking to GpuState

### 3. Chunks Not Rendering (Instance Buffer Disconnect)
**Problem**: Logs showed "0 drawn" despite chunks being generated and submitted
**Root Cause**: Instance data was never uploaded to GPU after submission
**Fix**: 
- Added `upload_instances()` method to GpuDrivenRenderer
- Called upload after submit_objects to ensure GPU has instance data
- This was a critical missing step in the GPU-driven rendering pipeline

## Architectural Issues Found

The CPU->GPU and OOP->DOP transitions left several integration gaps:

1. **Missing Data Upload**: Instance buffers were populated on CPU but never uploaded to GPU
2. **Excessive CPU Usage**: No frame limiting after GPU transition
3. **Coordinate System Confusion**: Mix of assumptions about position (feet vs body center)

## Testing Recommendations

1. Verify player spawns above terrain
2. Monitor CPU usage - should be significantly reduced
3. Confirm chunks are actually rendering (visual confirmation)
4. Check frame rate is limited to ~60 FPS

## Performance Impact

These fixes should result in:
- Dramatically reduced CPU usage and heat generation
- Proper chunk rendering 
- Correct player spawn positioning
- Stable 60 FPS instead of unlimited frame rate