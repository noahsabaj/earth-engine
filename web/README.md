# Earth Engine WebGPU - Data-Oriented Edition

This is a pure JavaScript implementation of the Earth Engine using 100% Data-Oriented Programming (DOP) principles.

## Architecture

**NO Object-Oriented Programming (OOP):**
- ❌ No classes
- ❌ No inheritance
- ❌ No `this` keyword
- ❌ No prototypes
- ❌ No methods

**Pure Data-Oriented Design:**
- ✅ Data structures (plain objects with typed arrays)
- ✅ Pure functions operating on data
- ✅ GPU buffers as single source of truth
- ✅ Clear separation of data and functions
- ✅ Explicit side effects

## Running the Demo

1. **Start the development server:**
   ```bash
   cd web
   python3 serve.py
   # or
   ./serve.py
   ```

2. **Open in a WebGPU-capable browser:**
   - Chrome Canary or Chrome 113+
   - Navigate to: http://localhost:8080
   - Enable WebGPU if needed: chrome://flags/#enable-unsafe-webgpu

## Project Structure

```
web/
├── index.html          # Main entry point
├── serve.py           # Development server
└── src/
    ├── gpu-state.js        # GPU device and buffer management
    ├── world-state.js      # World data structures
    ├── terrain-generation.js # Terrain generation functions
    ├── mesh-generation.js  # Mesh generation from voxels
    ├── camera-state.js     # Camera data and transforms
    ├── renderer.js         # Rendering functions
    ├── shader-snippets.js  # WGSL shader code
    ├── engine.js          # Main orchestration
    └── index.js           # Entry point
```

## Data Flow

1. **Initialization:**
   - GPU state initialized
   - World buffers allocated on GPU
   - Camera state initialized
   - Renderer pipeline created

2. **World Generation:**
   - Terrain generated directly on GPU
   - Mesh generated from voxels on GPU
   - All data stays on GPU

3. **Rendering:**
   - Camera matrices uploaded to GPU
   - Single draw call renders entire world
   - No CPU-GPU data transfer during runtime

## Key Data Structures

```javascript
// Example: World State
export const worldState = {
    palette: new Uint32Array(256),
    buffers: {
        voxel: null,      // GPU buffer
        metadata: null,   // GPU buffer
        palette: null,    // GPU buffer
        pageTable: null   // GPU buffer
    },
    initialized: false
};

// Example: Pure Function
export function generateTerrain(device, seed) {
    // Operates on worldState, returns nothing
    // Side effects are explicit
}
```

## Performance

- Zero-copy architecture
- All heavy computation on GPU
- Minimal CPU overhead
- Single draw call for entire world