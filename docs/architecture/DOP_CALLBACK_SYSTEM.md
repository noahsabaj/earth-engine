# DOP Callback System

## Overview

The DOP (Data-Oriented Programming) callback system replaces the old OOP Gateway pattern with a pure function-pointer based approach. This system allows games to integrate with the engine without inheritance, virtual dispatch, or object-oriented patterns.

## Architecture

The callback system is implemented in `src/game/callbacks.rs` and provides a clean interface for game logic integration through pure functions.

### Core Components

1. **GameCallbacks Structure** - A data structure holding function pointers
2. **Global Registration** - Thread-safe storage for callbacks
3. **Execution Functions** - Helper functions to invoke callbacks

## Implementation Details

### GameCallbacks Structure

```rust
pub struct GameCallbacks {
    /// Register game-specific blocks
    pub register_blocks: fn(&mut BlockRegistry),
    
    /// Update game state each frame
    pub update_game: fn(&mut dyn std::any::Any, &mut GameContext, f32),
    
    /// Handle when a block is broken
    pub on_block_break: fn(&mut dyn std::any::Any, VoxelPos, BlockId),
    
    /// Handle when a block is placed
    pub on_block_place: fn(&mut dyn std::any::Any, VoxelPos, BlockId),
    
    /// Get the currently active block for placement
    pub get_active_block: fn(&dyn std::any::Any) -> BlockId,
}
```

### Key Design Principles

1. **Pure Functions** - All callbacks are pure function pointers, not methods
2. **No Inheritance** - No base classes or trait implementations required
3. **Type Erasure** - Uses `dyn std::any::Any` for game state flexibility
4. **Zero Overhead** - Function pointers have no virtual dispatch cost
5. **Thread Safety** - Global registration uses Mutex for safe access

## Usage Example

### Game Implementation

```rust
// Define your game state (pure data)
struct MyGame {
    player_health: f32,
    score: u32,
    active_block: BlockId,
}

// Define callback functions
fn my_register_blocks(registry: &mut BlockRegistry) {
    registry.register(BlockId(100), BlockData {
        name: "custom_block",
        properties: BlockProperties::default(),
    });
}

fn my_update_game(game: &mut dyn std::any::Any, ctx: &mut GameContext, delta: f32) {
    let game = game.downcast_mut::<MyGame>().unwrap();
    // Update game logic here
}

fn my_on_block_break(game: &mut dyn std::any::Any, pos: VoxelPos, block: BlockId) {
    let game = game.downcast_mut::<MyGame>().unwrap();
    game.score += 10;
}

// Register callbacks during initialization
let callbacks = GameCallbacks {
    register_blocks: my_register_blocks,
    update_game: my_update_game,
    on_block_break: my_on_block_break,
    on_block_place: my_on_block_place,
    get_active_block: my_get_active_block,
};

register_game_callbacks(callbacks);
```

### Engine Integration

The engine automatically invokes registered callbacks at appropriate times:

```rust
// During block registry initialization
execute_register_blocks(&mut block_registry);

// During game update loop
execute_update_game(&mut game_state, &mut game_context, delta_time);

// When player breaks a block
execute_on_block_break(&mut game_state, position, block_id);
```

## Benefits Over Gateway Pattern

### Old Gateway Pattern (OOP)
```rust
trait GameGateway {
    fn update(&mut self, ctx: &mut GameContext, delta: f32);
    fn on_block_break(&mut self, pos: VoxelPos, block: BlockId);
}

struct MyGame;
impl GameGateway for MyGame {
    // Virtual dispatch overhead
}
```

### New Callback System (DOP)
- No virtual dispatch overhead
- No trait implementations required
- Functions can be defined anywhere
- Easier to test (pure functions)
- Better cache locality
- Simpler mental model

## Thread Safety

The callback system uses a global Mutex-protected storage:

```rust
static GAME_CALLBACKS: Mutex<Option<GameCallbacks>> = Mutex::new(None);
```

This ensures:
- Safe registration from any thread
- Consistent callback retrieval
- No data races

## Default Implementations

All callbacks have sensible defaults, allowing games to only implement what they need:

```rust
fn default_register_blocks(_registry: &mut BlockRegistry) {}
fn default_update_game(_game: &mut dyn std::any::Any, _ctx: &mut GameContext, _delta: f32) {}
fn default_on_block_break(_game: &mut dyn std::any::Any, _pos: VoxelPos, _block: BlockId) {}
fn default_on_block_place(_game: &mut dyn std::any::Any, _pos: VoxelPos, _block: BlockId) {}
fn default_get_active_block(_game: &dyn std::any::Any) -> BlockId { typed_blocks::GRASS }
```

## Performance Characteristics

- **Zero-cost abstraction** - Function pointers compile to direct calls
- **No allocations** - Callbacks stored in static memory
- **Cache-friendly** - No pointer chasing through vtables
- **Predictable performance** - No hidden virtual dispatch

## Future Extensions

The callback system can be extended with:
1. Additional callback points (e.g., `on_player_spawn`, `on_world_save`)
2. Async callbacks for long-running operations
3. Priority-based callback ordering
4. Hot-reload support for development

## Summary

The DOP callback system demonstrates how data-oriented design can provide the same extensibility as OOP patterns while maintaining better performance, simpler code, and easier reasoning about behavior. It's a key component of Hearth Engine's 100% DOP architecture.