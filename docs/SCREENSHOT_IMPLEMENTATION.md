# Screenshot Implementation Documentation

## Overview
This document describes the screenshot capture functionality integrated into the Earth Engine's render loop.

## Implementation Details

### 1. **GpuState Modifications**

Added the following fields to `GpuState` struct:
- `debug_capture_enabled: bool` - Toggle for automatic screenshot capture (F5)
- `capture_timer: f32` - Timer for automatic capture intervals
- `capture_interval: f32` - Interval between automatic captures (default: 0.25s)
- `screenshot_counter: u32` - Counter for single screenshot requests (F6)
- `last_capture_time: Option<std::time::Instant>` - Timestamp of last capture

### 2. **Render Loop Integration**

Modified `GpuState::render()` method to:
- Accept `delta_time: f32` parameter for timer updates
- Check if screenshot should be captured before submitting commands
- Call `capture_screenshot()` when conditions are met

### 3. **Key Bindings**

- **F5**: Toggle automatic screenshot capture mode
  - When enabled, captures screenshots every 0.25 seconds
  - Logs enable/disable status
  
- **F6**: Single screenshot capture
  - Increments screenshot counter
  - Captures on next frame render

### 4. **Screenshot Methods**

#### `should_capture_screenshot(&mut self, delta_time: f32) -> bool`
- Checks if single screenshot requested (F6 pressed)
- Updates timer for automatic capture mode
- Prevents multiple captures from single key press

#### `capture_screenshot(&mut self, encoder: &mut CommandEncoder, texture: &Texture)`
- Generates unique filename with timestamp
- Creates `debug/photos/` directory if needed
- Copies GPU texture to staging buffer
- Spawns background thread for async processing
- Updates capture state

#### `process_screenshot_async(...) -> Result<()>`
- Maps GPU buffer for reading
- Converts buffer data to image
- Saves PNG file to disk
- Runs in background thread to avoid blocking render loop

#### `generate_screenshot_filename(&self) -> String`
- Format: `screenshot_YYYYMMDD_HHMMSS_NNN.png`
- Uses chrono for timestamp
- Atomic counter for uniqueness

### 5. **Performance Considerations**

1. **Minimal Render Loop Impact**:
   - Screenshot processing done in background thread
   - GPU->CPU transfer is queued, not blocking
   - File I/O happens asynchronously

2. **Memory Management**:
   - Staging buffer created per capture
   - Buffer unmapped after use
   - No persistent memory overhead

3. **Throttling**:
   - 0.5s cooldown between F6 captures
   - Configurable interval for F5 mode
   - Prevents accidental spam

### 6. **Error Handling**

- Directory creation failures logged but don't crash
- Screenshot save failures logged but don't interrupt rendering
- Texture format validation ensures compatibility

### 7. **File Organization**

Screenshots saved to: `debug/photos/`
- Directory created automatically
- Files sorted by timestamp
- Counter prevents overwrites

## Usage

1. Press **F5** to enable automatic capture mode
2. Press **F6** for single screenshot
3. Screenshots appear in `debug/photos/` directory
4. Check logs for capture status

## Dependencies

- `screenshot` module (already implemented)
- `chrono` for timestamps
- `image` for PNG encoding
- `wgpu` for GPU operations
- `pollster` for async runtime