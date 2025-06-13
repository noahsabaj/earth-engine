# Earth Engine Examples

This directory contains runnable examples demonstrating how to use various features of the Earth Engine.

## Purpose

Examples serve multiple purposes:
- **Documentation**: Show developers how to use APIs correctly
- **Testing**: Informal testing of features in isolation
- **Learning**: Help new contributors understand the codebase
- **Validation**: Ensure APIs remain usable and ergonomic

## Running Examples

To run any example:
```bash
cargo run --example <example_name>
```

For example:
```bash
cargo run --example dop_particles
```

## Organization

Examples are organized by category:

### `/rendering`
GPU rendering, mesh generation, and visual effects examples.

### `/world_generation`
Terrain generation, chunk management, and noise function examples.

### `/gameplay`
Game mechanics like spawn positions, inventory, and player systems.

### `/particles`
Particle system demonstrations and effects.

### `/debugging`
Debugging tools and performance analysis examples.

## Guidelines for Examples

1. **Keep Updated**: Examples must be updated when APIs change
2. **Self-Contained**: Each example should be runnable independently
3. **Well-Commented**: Include comments explaining what's happening
4. **Realistic Usage**: Show real-world usage patterns
5. **Performance**: Examples should complete quickly (< 10 seconds)

## Adding New Examples

When adding a new example:
1. Place it in the appropriate category directory
2. Add an entry to `Cargo.toml` under `[[example]]`
3. Include a header comment explaining what the example demonstrates
4. Keep it focused on demonstrating one specific feature

## Maintenance

Examples are tested during CI to ensure they continue to compile. If an API change breaks examples, they must be updated in the same PR.