# Earth Engine Web - WebGPU Implementation

This directory contains the complete WebGPU implementation of Earth Engine that runs in the browser.

## Structure

```
web/
├── index.html          # Main entry point
├── src/
│   ├── index.js       # JavaScript entry point
│   ├── core/          # Core engine systems
│   ├── world/         # World generation
│   └── renderer/      # GPU rendering
└── README.md          # This file
```

## Running

From the repository root:

```bash
python3 serve.py
```

Then open: http://localhost:8080/web/

## Requirements

- Chrome Canary or Edge Canary
- Enable WebGPU: `chrome://flags/#enable-unsafe-webgpu`
- Modern GPU with updated drivers

## Architecture

This JavaScript implementation uses the same GPU-first architecture as the Rust engine:
- All world data lives on GPU
- Single draw call rendering
- GPU compute shaders for everything
- Zero CPU-GPU sync points