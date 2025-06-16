# Cargo Commands Reference Guide

A comprehensive guide to Cargo commands for Hearth Engine development.

## Table of Contents
- [Essential Commands](#essential-commands)
- [Build & Run Commands](#build--run-commands)
- [Testing Commands](#testing-commands)
- [Debugging & Inspection](#debugging--inspection)
- [Package Management](#package-management)
- [Advanced Commands](#advanced-commands)
- [Performance Analysis](#performance-analysis)
- [Development Workflow](#development-workflow)
- [Hearth Engine Specific](#hearth-engine-specific)

## Essential Commands

These are the commands you'll use daily when developing with Hearth Engine:

```bash
cargo check      # Quick syntax and type checking (no compilation)
cargo build      # Compile the project in debug mode
cargo run        # Build and run the default binary
cargo test       # Run all tests
cargo clippy     # Run the Rust linter for code quality
cargo fmt        # Auto-format your code
```

## Build & Run Commands

### Basic Building
```bash
cargo build              # Build in debug mode (fast compile, slow runtime)
cargo build --release    # Build in release mode (slow compile, fast runtime)
cargo clean              # Remove all build artifacts
```

### Running Binaries
```bash
cargo run                      # Run the default binary
cargo run --bin <name>         # Run a specific binary
cargo run --example <name>     # Run an example
cargo run -- <args>            # Pass arguments to your program
cargo run --release            # Run in release mode
```

### Build Options
```bash
cargo build --all-features     # Build with all optional features
cargo build --no-default-features  # Build without default features
cargo build --features "feature1 feature2"  # Build with specific features
cargo build --target wasm32-unknown-unknown  # Cross-compile for WASM
```

## Testing Commands

### Running Tests
```bash
cargo test                     # Run all tests
cargo test <pattern>           # Run tests matching a pattern
cargo test --lib               # Test only the library
cargo test --bins              # Test all binaries
cargo test --examples          # Test all examples
cargo test --doc               # Run documentation tests
```

### Test Options
```bash
cargo test -- --nocapture      # Show println! output during tests
cargo test -- --test-threads=1 # Run tests sequentially
cargo test --release           # Run tests in release mode
cargo test --ignored           # Run only ignored tests
cargo test -- --exact <name>   # Run exact test by name
```

### Benchmarking
```bash
cargo bench                    # Run all benchmarks
cargo bench <pattern>          # Run specific benchmarks
```

## Debugging & Inspection

### Code Quality Tools
```bash
cargo check              # Fast syntax/type checking without building
cargo clippy             # Advanced linting with helpful suggestions
cargo clippy -- -W clippy::all  # Run with all warnings enabled
cargo fmt                # Format code according to Rust style
cargo fmt -- --check     # Check formatting without changing files
```

### Dependency Inspection
```bash
cargo tree               # Display dependency tree
cargo tree -i <crate>    # Show what depends on a crate
cargo tree -d            # Show duplicate dependencies
cargo audit              # Check for security vulnerabilities
cargo outdated           # Check for outdated dependencies
```

### Documentation
```bash
cargo doc                # Generate documentation
cargo doc --open         # Generate and open docs in browser
cargo doc --no-deps      # Document only your crate
cargo rustdoc -- --help  # Advanced documentation options
```

## Package Management

### Adding Dependencies
```bash
cargo add <crate>              # Add latest version
cargo add <crate>@<version>    # Add specific version
cargo add <crate> --features "feature1,feature2"  # Add with features
cargo add <crate> --dev        # Add as dev-dependency
cargo add <crate> --build      # Add as build-dependency
```

### Managing Dependencies
```bash
cargo remove <crate>           # Remove a dependency
cargo update                   # Update all dependencies
cargo update <crate>           # Update specific dependency
cargo search <term>            # Search crates.io
```

### Publishing
```bash
cargo publish --dry-run        # Test publishing without uploading
cargo publish                  # Publish to crates.io
cargo package                  # Create a .crate file
cargo install <crate>          # Install binary crate globally
cargo install --path .         # Install current project globally
```

## Advanced Commands

### Macro Expansion
```bash
cargo expand                   # Show macro expansions (requires cargo-expand)
cargo expand <module>          # Expand specific module
```

### Build Scripts
```bash
cargo build --verbose          # Show build script output
cargo build -vv                # Very verbose output
```

### Configuration
```bash
cargo metadata                 # Output project metadata as JSON
cargo verify-project           # Verify Cargo.toml validity
cargo locate-project           # Show path to Cargo.toml
```

## Performance Analysis

### Profiling Tools
```bash
cargo flamegraph              # Generate flamegraph (requires cargo-flamegraph)
cargo profdata                # CPU profiling data
cargo asm <function>          # Show assembly for function
cargo llvm-ir <function>      # Show LLVM IR
```

### Binary Analysis
```bash
cargo bloat                   # Analyze binary size (requires cargo-bloat)
cargo bloat --release         # Analyze release binary
cargo size                    # Show section sizes (requires cargo-size)
```

### Compilation Time
```bash
cargo build -Z timings        # Generate timing report (nightly)
cargo clean && cargo build    # Measure full rebuild time
```

## Development Workflow

### Continuous Development
```bash
cargo watch -x check          # Auto-check on file changes
cargo watch -x test           # Auto-test on file changes
cargo watch -x run            # Auto-run on file changes
cargo watch -x "check --tests"  # Auto-check including tests
```

### Project Creation
```bash
cargo new <name>              # Create new binary project
cargo new <name> --lib        # Create new library project
cargo init                    # Initialize in existing directory
cargo generate <template>     # Create from template (requires cargo-generate)
```

## Hearth Engine Specific

### Common Workflows
```bash
# Quick development cycle
cargo check && cargo clippy && cargo test

# Performance testing
cargo build --release && cargo run --release

# Before committing
cargo fmt && cargo clippy && cargo test

# Full validation
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```

### Hearth Engine Features
```bash
# Build with specific rendering backend
cargo build --features "vulkan"
cargo build --features "dx12"

# Build with debugging features
cargo build --features "debug-ui,profiler"

# Run benchmarks
cargo bench --features "benchmarks"
```

### Recommended Aliases

Add these to your shell configuration:

```bash
alias cc='cargo check'
alias cb='cargo build'
alias cr='cargo run'
alias ct='cargo test'
alias cf='cargo fmt'
alias ccl='cargo clippy'
alias cw='cargo watch -x check -x test'
```

## Tips and Best Practices

1. **Use `cargo check` frequently** - It's much faster than `cargo build` for catching errors
2. **Run `cargo clippy` before committing** - It catches many common mistakes
3. **Use `--release` for performance testing** - Debug builds can be 100x slower
4. **Install `cargo-watch`** - It greatly improves the development experience
5. **Learn `cargo tree`** - Essential for debugging dependency conflicts
6. **Use `cargo doc --open`** - Great for exploring dependencies
7. **Profile before optimizing** - Use `cargo flamegraph` to find real bottlenecks

## Troubleshooting

### Common Issues

**Slow compilation?**
```bash
cargo check              # Use check instead of build during development
cargo build --jobs 8     # Increase parallel jobs
cargo build -Z threads=8 # Use parallel frontend (nightly)
```

**Dependency conflicts?**
```bash
cargo tree -d            # Find duplicate dependencies
cargo update -p <crate>  # Update specific problematic crate
cargo clean              # Sometimes helps with weird errors
```

**Out of disk space?**
```bash
cargo clean              # Clean current project
# Clean global cache (careful!)
rm -rf ~/.cargo/registry/cache
rm -rf ~/.cargo/git/checkouts
```

## Additional Resources

- [The Cargo Book](https://doc.rust-lang.org/cargo/)
- [Cargo Command Reference](https://doc.rust-lang.org/cargo/commands/)
- [Awesome Cargo Extensions](https://github.com/rust-unofficial/awesome-rust#development-tools)
- [Hearth Engine Documentation](https://noahsabaj.github.io/hearth-website/)