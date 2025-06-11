# Earth Engine JavaScript - WebGPU Implementation

This is a parallel implementation of Earth Engine in JavaScript that demonstrates the same GPU-first architecture as the Rust version. Instead of compiling Rust to WASM, we run JavaScript on the CPU for orchestration while keeping all heavy computation on the GPU through WebGPU.

## Key Insight

WebGPU in browsers provides the **exact same** GPU compute capabilities as wgpu in Rust. The only difference is the orchestration language:

- **Rust Engine**: Rust → wgpu → GPU (WGSL shaders)
- **JS Engine**: JavaScript → WebGPU → GPU (same WGSL shaders)

## Features

- **GPU-First Architecture**: All world data lives on GPU
- **Single Draw Call**: GPU decides what to render
- **Zero Allocations**: No heap allocations in render loop
- **Compute Shaders**: Terrain generation, physics, mesh generation all on GPU
- **Data-Oriented Design**: Pure functions and immutable data structures

## Quick Start

1. **Requirements**:
   - Chrome Canary or Edge Canary
   - Enable WebGPU: `chrome://flags/#enable-unsafe-webgpu`
   - Modern GPU with updated drivers

2. **Run the server**:
   ```bash
   cd earth-engine
   python3 serve.py
   ```

3. **Open in browser**:
   ```
   http://localhost:8080/web/
   ```

## Project Structure

```
earth-engine-js/
├── src/
│   ├── core/
│   │   ├── earth-engine.js    # Main engine class
│   │   ├── gpu-context.js     # WebGPU initialization
│   │   ├── shader-loader.js   # Shader management
│   │   └── math.js           # Vector/matrix math
│   ├── world/
│   │   ├── world-buffer.js    # GPU buffer management
│   │   └── terrain-generator.js # GPU terrain generation
│   ├── renderer/
│   │   ├── camera.js          # First-person camera
│   │   ├── mesh-generator.js  # GPU mesh generation
│   │   └── gpu-renderer.js    # GPU-driven rendering
│   └── index.js               # Entry point
└── IMPLEMENTATION_PLAN.md     # Detailed implementation notes
```

## Performance

The JavaScript implementation targets the same performance as the Rust version:

- 60+ FPS with millions of voxels
- Single draw call per frame
- All decisions made on GPU
- Zero CPU-GPU sync points

## Development

The codebase follows functional programming principles:
- Immutable data structures
- Pure functions
- No side effects in core logic
- GPU kernels handle all mutations

## Status

This implementation demonstrates that the Earth Engine architecture is language-agnostic. The GPU does all the heavy lifting, while the CPU language (Rust or JavaScript) is just orchestration.