# Debugging Examples

This directory contains debugging tools and diagnostic examples.

## Examples

### `debug_screenshot_issue.rs`
Helper for debugging rendering issues by capturing screenshots at specific moments.

### `gpu_memory_leak_fix.rs`
Demonstrates techniques for finding and fixing GPU memory leaks.

### `trace_spawn_detail.rs`
Detailed tracing of spawn position calculations for debugging spawn issues.

### `trace_terrain_at_origin.rs`
Traces terrain generation at world origin to understand height calculations.

## Running

```bash
cargo run --example trace_spawn_detail
cargo run --example gpu_memory_leak_fix
```

## Topics Covered

- Performance profiling
- Memory leak detection
- Render debugging
- Algorithm tracing
- GPU diagnostics
- Issue reproduction