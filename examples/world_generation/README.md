# World Generation Examples

This directory contains examples demonstrating terrain generation, chunk management, and procedural world creation.

## Examples

### `chunk_loading_demo.rs`
Shows the chunk loading system in action, demonstrating how chunks are loaded and unloaded based on player position.

### `test_chunk_boundaries.rs`
Tests edge cases at chunk boundaries to ensure seamless world generation.

### `test_noise_detailed.rs`
Detailed analysis of the Perlin noise functions used for terrain generation.

### `test_noise_values.rs`
Validates noise function outputs for expected ranges and distributions.

### `test_spawn_pregeneration.rs`
Demonstrates pre-generating chunks around spawn to ensure smooth game start.

### `test_terrain_generation.rs`
Complete terrain generation pipeline from noise to voxels.

## Running

```bash
cargo run --example chunk_loading_demo
cargo run --example test_terrain_generation
```

## Topics Covered

- Perlin/Simplex noise
- Chunk management
- Biome generation
- Cave systems
- Ore distribution
- Parallel generation