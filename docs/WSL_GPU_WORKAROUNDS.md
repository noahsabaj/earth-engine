# WSL GPU Testing Workarounds

## Overview
The Hearth Engine requires GPU rendering capabilities which are not available by default in WSL environments. This document provides several workarounds to enable testing in WSL.

## Option A: Software Rendering with Mesa/LLVMpipe (Recommended)

Mesa provides a software rasterizer that can emulate GPU functionality. Performance will be significantly reduced but it enables basic testing.

```bash
# Install Mesa software renderer
sudo apt-get update
sudo apt-get install mesa-utils libgl1-mesa-dri mesa-vulkan-drivers

# Force software rendering
export LIBGL_ALWAYS_SOFTWARE=1
export GALLIUM_DRIVER=llvmpipe
export MESA_GL_VERSION_OVERRIDE=4.5
export MESA_GLSL_VERSION_OVERRIDE=450

# Test OpenGL functionality
glxinfo | grep "OpenGL renderer"
# Should show: "llvmpipe (LLVM ...)"

# Run the engine with software rendering
cargo run --example minimal_engine
```

### Performance Tips for Software Rendering:
- Reduce render distance in EngineConfig
- Lower window resolution
- Disable advanced rendering features
- Expect 5-10 FPS instead of 60+ FPS

## Option B: WSLg with GPU Support (Windows 11 Only)

Windows 11 includes WSLg which supports GPU passthrough for better performance.

### Requirements:
- Windows 11 (build 22000 or higher)
- WSL2 (not WSL1)
- Latest GPU drivers from your vendor

### Setup:
1. Update Windows 11 to latest version
2. Install latest GPU drivers:
   - NVIDIA: [CUDA WSL drivers](https://developer.nvidia.com/cuda/wsl)
   - AMD: [Radeon Software for WSL](https://www.amd.com/en/support/kb/release-notes/rn-rad-win-wsl-support)
   - Intel: [Intel Graphics WSL drivers](https://www.intel.com/content/www/us/en/develop/documentation/get-started-with-intel-oneapi-base-linux/top/install-gpu-drivers.html)

3. Verify GPU is available:
```bash
# Check if GPU is detected
ls /dev/dri/
# Should show: card0 renderD128

# For NVIDIA:
nvidia-smi

# Test with engine
cargo run --example engine_testbed
```

## Option C: Headless Testing with Virtual Framebuffer

For automated testing or CI/CD, use Xvfb to create a virtual display.

```bash
# Install Xvfb
sudo apt-get install xvfb

# Create virtual display
export DISPLAY=:99
Xvfb :99 -screen 0 1024x768x24 &

# Run tests headlessly
xvfb-run -a cargo test
xvfb-run -a cargo run --example minimal_engine
```

## Option D: CPU Fallback Renderer (Future)

A CPU-only renderer is planned but not yet implemented. This would provide the best compatibility for testing environments.

### Temporary Workaround:
Create a mock renderer for testing non-rendering functionality:

```rust
// In your test code
#[cfg(test)]
mod tests {
    use hearth_engine::*;
    
    // Mock renderer that skips GPU operations
    struct MockRenderer;
    
    impl MockRenderer {
        fn new() -> Self {
            // Skip GPU initialization
            Self
        }
        
        fn render(&mut self) {
            // No-op for testing
        }
    }
}
```

## Option E: Remote Development

Develop in WSL but run the engine on native Windows or Linux with GPU.

### Using VS Code Remote:
1. Install VS Code with Remote-WSL extension
2. Develop in WSL
3. Build for Windows target:
```bash
# Add Windows target
rustup target add x86_64-pc-windows-gnu

# Cross-compile
cargo build --target x86_64-pc-windows-gnu

# Copy to Windows and run natively
```

## Testing Strategies Without GPU

### 1. Unit Test Non-Rendering Code
```bash
# Test everything except renderer
cargo test --lib --exclude gpu_state
```

### 2. Integration Tests with Mocked GPU
```rust
#[cfg(test)]
mod integration_tests {
    // Test physics without rendering
    #[test]
    fn test_physics_system() {
        // Physics doesn't need GPU
    }
    
    // Test world generation
    #[test] 
    fn test_world_gen() {
        // CPU-based world generation
    }
}
```

### 3. Benchmarks Without Rendering
```bash
# Run performance benchmarks that don't need GPU
cargo bench --bench dop_vs_oop
```

## Troubleshooting

### "No surface formats available" Error
This means no GPU adapter was found. Try:
1. Set software rendering environment variables
2. Install Mesa drivers
3. Use Xvfb for headless mode

### "Failed to create adapter" Error
The wgpu backend couldn't initialize. Solutions:
1. Update Mesa to latest version
2. Try different backend: `export WGPU_BACKEND=gl`
3. Use software rendering

### Poor Performance
Software rendering is 100x slower than GPU. This is expected. For performance testing:
1. Use native Linux/Windows with GPU
2. Reduce test complexity
3. Focus on logic testing, not rendering performance

## Recommended Approach for Development

1. **Primary Development**: Use software rendering for logic/gameplay
2. **Performance Testing**: Use native Windows/Linux with GPU
3. **CI/CD**: Use Xvfb for automated tests
4. **Final Testing**: Always test on target hardware

## Future Improvements

The engine team is working on:
1. CPU-only fallback renderer
2. Reduced GPU requirements mode
3. Better WSL detection and auto-configuration
4. Vulkan software rendering support

## Summary

While the Hearth Engine is designed for GPU acceleration, these workarounds enable development and testing in WSL environments. Choose the option that best fits your workflow:

- **Quick Testing**: Mesa software rendering (Option A)
- **Better Performance**: WSLg with GPU (Option B)
- **Automated Testing**: Xvfb (Option C)
- **Full Performance**: Native Windows/Linux

Remember: These are workarounds. For production testing and gameplay, always use a system with proper GPU support.