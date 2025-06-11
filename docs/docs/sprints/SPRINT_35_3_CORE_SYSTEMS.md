# Sprint 35.3: Core Systems Implementation

## Status: MAKING IT REAL 🎮

### Overview
The engine has no game loop, no player controller, no actual gameplay. This sprint implements the BASICS.

### Goals (Week 5-6)

#### Week 5: The Heartbeat
- [ ] Implement ACTUAL game loop (not just render loop)
- [ ] Fixed timestep with interpolation
- [ ] State synchronization between systems
- [ ] Performance profiling integration
- [ ] Command buffer system
- [ ] Event queue (zero-allocation)

#### Week 6: Minimum Viable Game
- [ ] Player controller (movement, look)
- [ ] Chunk loading around player
- [ ] Basic voxel interactions
- [ ] World persistence (save/load)
- [ ] Network packet definitions
- [ ] Basic UI system (debug overlay)

### Game Loop Architecture

```rust
// Real game loop with fixed timestep
pub fn run_engine(state: &mut EngineState) -> Result<(), EngineError> {
    const FIXED_TIMESTEP: Duration = Duration::from_millis(16); // 60 Hz
    let mut accumulator = Duration::ZERO;
    let mut last_frame = Instant::now();
    
    loop {
        let current = Instant::now();
        let delta = current - last_frame;
        last_frame = current;
        
        accumulator += delta;
        
        // Fixed timestep updates
        while accumulator >= FIXED_TIMESTEP {
            update_physics(&mut state.physics, FIXED_TIMESTEP)?;
            update_game_logic(&mut state.game, FIXED_TIMESTEP)?;
            accumulator -= FIXED_TIMESTEP;
        }
        
        // Interpolation alpha
        let alpha = accumulator.as_secs_f32() / FIXED_TIMESTEP.as_secs_f32();
        
        // Render with interpolation
        render(&state, alpha)?;
        
        // Metrics
        state.metrics.frame_time = delta;
        state.metrics.fps = 1.0 / delta.as_secs_f32();
    }
}
```

### Player Controller (DOP Style)

```rust
pub struct PlayerInput {
    pub movement: Vec3,
    pub look_delta: Vec2,
    pub actions: BitFlags<PlayerAction>,
}

pub fn update_players(
    inputs: &Buffer<PlayerInput>,
    positions: &mut Buffer<Vec3>,
    velocities: &mut Buffer<Vec3>,
    count: usize,
    dt: f32,
) {
    parallel_compute(|i| {
        // Apply movement
        velocities[i] += inputs[i].movement * MOVE_SPEED * dt;
        velocities[i] *= FRICTION;
        positions[i] += velocities[i] * dt;
    });
}
```

### Success Criteria
- Game loop runs at stable 60Hz ✓
- Player can move and look ✓
- Chunks load/unload properly ✓
- World saves and loads ✓
- Zero crashes in 1 hour ✓

### Deliverables
1. Working game loop with metrics
2. Basic player controller
3. Chunk management system
4. Save/load functionality
5. Video proof of gameplay