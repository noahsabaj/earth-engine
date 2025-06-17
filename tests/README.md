# Hearth Engine Integration Tests

This directory contains integration tests that verify the behavior of multiple modules working together.

## Purpose

Integration tests serve a different purpose than unit tests:
- **Scope**: Test interactions between multiple modules
- **Independence**: Each test file is compiled as a separate binary
- **Real-world scenarios**: Test complete workflows as users would experience them
- **External dependencies**: Can test with actual files, network, etc.

## Running Tests

To run all integration tests:
```bash
cargo test
```

To run a specific integration test:
```bash
cargo test --test test_parallel_chunk_manager
```

To run with output displayed:
```bash
cargo test -- --nocapture
```

## Current Tests

### `test_parallel_chunk_manager.rs`
Tests the parallel chunk loading system with multiple threads, ensuring chunks are generated and consumed correctly.

### `cursor_lock_test.rs`
Tests the cursor locking mechanism for first-person controls.

## Writing Integration Tests

Integration tests should:
1. **Test complete features**: Not just individual functions
2. **Use public APIs**: Only test through the public interface
3. **Be deterministic**: Avoid timing-dependent tests
4. **Clean up resources**: Don't leave temporary files
5. **Run quickly**: Keep total test time under 30 seconds

## Difference from Unit Tests

- **Unit tests** (in `/src` with `#[test]`): Test individual functions/modules
- **Integration tests** (in `/tests`): Test multiple modules together
- **Examples** (in `/examples`): Demonstrate usage, not automated testing

## Adding New Tests

When adding a new integration test:
1. Create a new `.rs` file in this directory
2. Import the crate with `use earth_engine::*;`
3. Write test functions with `#[test]` attribute
4. Ensure the test is self-contained and cleans up

## CI/CD Integration

All integration tests are run automatically on:
- Pull requests
- Commits to main branch
- Release builds

Failed tests will block merging and deployment.