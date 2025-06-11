# Debug Notes - WebGPU Rendering Issue

## Current Status
- Engine initializes successfully
- Terrain generation runs (8.3M voxels in ~45ms)
- Mesh generation runs but produces 0 vertices/indices
- Screen shows sky blue but no terrain

## Fixed Issues
1. ✅ Module loading (index.html was looking for old OOP code)
2. ✅ GPU device limits (reduced from 2GB to 2GB-1MB)
3. ✅ Bind group mismatch (split generate/finalize bind groups)
4. ✅ Buffer usage flags (added COPY_DST to counter buffer)

## Remaining Issues
1. **No mesh being generated** - 0 vertices despite terrain generation
2. **Morton encoding producing invalid offsets** - Need to debug the encoding function
3. **Not fully DOP** - Still using object literals for API organization

## Pure DOP Question
Currently we have:
```javascript
export const engine = {
    initialize: initializeEngine,
    start: startEngine,
    // etc
};
```

For PURE DOP, we would just export functions directly:
```javascript
export { initializeEngine, startEngine, stopEngine, generateWorld };
```

But this loses the nice namespacing. What's preferred?

## Next Steps
1. Debug why mesh generation finds no voxels to mesh
2. Fix Morton encoding function
3. Decide on pure function exports vs organized objects
4. Get terrain rendering!