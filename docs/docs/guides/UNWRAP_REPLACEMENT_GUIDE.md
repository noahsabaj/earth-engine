# Unwrap Replacement Guide

**Total unwraps to replace**: 373 across 67 files  
**Completed**: ~31 (persistence/save_manager.rs)  
**Remaining**: ~342

## Quick Reference - Common Patterns

### 1. Mutex/RwLock (60% of unwraps)
```rust
// ❌ BAD - Will panic if poisoned
let data = mutex.lock().unwrap();

// ✅ GOOD - Returns error
let data = mutex.lock()?;

// ✅ GOOD - With context
let data = mutex.lock()
    .map_err(|_| EngineError::LockPoisoned("resource_name".into()))?;
```

### 2. Option unwrap (15% of unwraps)
```rust
// ❌ BAD - Will panic if None
let value = map.get(&key).unwrap();

// ✅ GOOD - Returns error
let value = map.get(&key)
    .ok_or_else(|| EngineError::NotFound { key })?;
```

### 3. Channel operations (10% of unwraps)
```rust
// ❌ BAD - Will panic if channel closed
let msg = receiver.recv().unwrap();

// ✅ GOOD - Returns error
let msg = receiver.recv()
    .map_err(|_| EngineError::ChannelClosed { name: "worker" })?;
```

### 4. Array/Vec access (5% of unwraps)
```rust
// ❌ BAD - Will panic if out of bounds
let item = vec[index].unwrap();

// ✅ GOOD - Returns error
let item = vec.get(index)
    .ok_or_else(|| EngineError::BufferAccess { index, size: vec.len() })?;
```

## Module-Specific Error Types

Each module should have its own error variants in EngineError:

### Network Module
```rust
EngineError::ConnectionFailed { addr, error }
EngineError::PacketTooLarge { size, max_size }
EngineError::PlayerNotFound { id }
```

### World Module
```rust
EngineError::ChunkNotLoaded { pos }
EngineError::BlockOutOfBounds { pos, chunk_size }
EngineError::InvalidBlockType { id }
```

### Renderer Module
```rust
EngineError::ShaderCompilation { source, error }
EngineError::TextureNotFound { id }
EngineError::MeshGeneration { chunk_pos, error }
```

## Step-by-Step Process

### 1. Find unwraps in a file
```bash
rg "\.unwrap\(\)" src/module/file.rs -n
```

### 2. Determine error context
- What resource is being accessed?
- Why might it fail?
- What information would help debugging?

### 3. Choose replacement pattern
- Use `?` for simple propagation
- Use `.ok_or_else()` for Options
- Use `.map_err()` to add context

### 4. Update function signatures
```rust
// Before
pub fn process(&self) {
    let data = self.data.lock().unwrap();
}

// After
pub fn process(&self) -> EngineResult<()> {
    let data = self.data.lock()?;
    Ok(())
}
```

### 5. Handle errors at boundaries
```rust
// In main game loop
if let Err(e) = system.process() {
    log::error!("System error: {}", e);
    // Decide: recover, skip frame, or shutdown
}
```

## Priority Order

### 1. Critical Path (Do First)
- Main game loop
- Rendering pipeline  
- Input handling
- Save/Load system ✓

### 2. High Priority
- Network sync
- Physics updates
- World generation
- Asset loading

### 3. Medium Priority
- UI systems
- Audio (if implemented)
- Particle effects
- Weather system

### 4. Low Priority
- Debug tools
- Profiling
- Test utilities
- Example code

## Testing After Replacement

### 1. Unit Test
```rust
#[test]
fn test_error_handling() {
    let result = function_that_might_fail();
    assert!(result.is_err());
    match result {
        Err(EngineError::SpecificError { .. }) => {},
        _ => panic!("Wrong error type"),
    }
}
```

### 2. Integration Test
- Run game for 1 hour without panics
- Trigger error conditions intentionally
- Verify error messages are helpful

### 3. Stress Test
```rust
// Simulate poisoned mutex
let mutex = Arc::new(Mutex::new(42));
let mutex_clone = mutex.clone();
thread::spawn(move || {
    let _lock = mutex_clone.lock().unwrap();
    panic!("Poisoning the mutex");
});
thread::sleep(Duration::from_millis(100));

// Should handle poisoned mutex gracefully
let result = safe_function(&mutex);
assert!(result.is_err());
```

## Common Mistakes to Avoid

### 1. Replacing with panic!()
```rust
// ❌ BAD - Still panics!
let data = mutex.lock().unwrap_or_else(|_| panic!("Lock poisoned"));

// ✅ GOOD - Returns error
let data = mutex.lock()?;
```

### 2. Losing error context
```rust
// ❌ BAD - Generic error
.ok_or(EngineError::Internal { message: "error".into() })?;

// ✅ GOOD - Specific error
.ok_or_else(|| EngineError::ChunkNotLoaded { pos })?;
```

### 3. Ignoring errors
```rust
// ❌ BAD - Silently fails
let _ = potentially_failing_operation();

// ✅ GOOD - Log if can't propagate
if let Err(e) = potentially_failing_operation() {
    log::warn!("Non-critical error: {}", e);
}
```

## Automation Help

### Find all unwraps
```bash
#!/bin/bash
echo "Files with unwrap() calls:"
rg "\.unwrap\(\)" src/ -l | while read file; do
    count=$(rg "\.unwrap\(\)" "$file" -c)
    echo "$file: $count unwraps"
done | sort -t: -k2 -nr
```

### VSCode Search Regex
```
\.unwrap\(\)
```

### Clippy Help
```rust
#![warn(clippy::unwrap_used)]
```

## Progress Tracking

Create a spreadsheet or use this format:

| Module | File | Unwraps | Status | PR |
|--------|------|---------|--------|-----|
| persistence | save_manager.rs | 31 | ✅ Complete | #35.1 |
| network | server.rs | 31 | ❌ TODO | - |
| network | client.rs | 29 | ❌ TODO | - |
| hot_reload | asset_reload.rs | 23 | ❌ TODO | - |

## Remember

Every unwrap() is a future crash report from a user. Take the time to handle errors properly. The users (and future you) will thank you!