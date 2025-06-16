# JavaScript Engine Structure

The parallel JavaScript implementation of Earth Engine should be created as a separate repository or folder at the same level as the Rust engine:

```
hearth-engine-workspace/
├── hearth-engine/          # Rust engine (this repo)
├── hearth-engine-js/       # JavaScript engine (new)
└── shared/               # Shared assets (optional)
    └── shaders/          # Symlinked from both engines
```

## Initial Structure Created

```
hearth-engine-js/
├── package.json
├── index.html
└── src/
    ├── index.js              # Main engine entry point
    └── world/
        └── world-buffer.js   # Direct port of WorldBuffer
```

## Key Files

### package.json
- Name: hearth-engine-js
- Version: 0.35.0 (same as Rust)
- Type: module (ES6 imports)

### src/index.js
- Main EarthEngine class
- WebGPU initialization
- Game loop
- Subsystem coordination

### src/world/world-buffer.js
- Direct port of world_buffer.rs
- Same buffer layouts
- Same Morton encoding
- Same bind group structure

## How to Start

1. Copy the hearth-engine-js folder from the workspace
2. Link to shared shaders: `ln -s ../hearth-engine/shaders shaders`
3. Run: `python3 -m http.server 8080`
4. Open: http://localhost:8080/

The JavaScript engine will use the EXACT same GPU architecture as the Rust version!