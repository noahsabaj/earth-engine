// Minimal test compute shader - simplest possible working example
@group(0) @binding(0) var<storage, read_write> test_buffer: array<u32>;

@compute @workgroup_size(1)
fn test_compute(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index < arrayLength(&test_buffer)) {
        test_buffer[index] = index + 42u;
    }
}