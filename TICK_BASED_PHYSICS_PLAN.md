# Tick-Based Physics System Implementation Plan

## Executive Summary

This document outlines a comprehensive plan to implement a tick-based physics system for Hearth Engine, replacing traditional accumulator-based timesteps with discrete, predictable physics ticks. This approach aligns perfectly with our voxel-based world and GPU-first architecture, providing deterministic simulation for 10,000+ concurrent players.

## Problem Statement

### Current Timestep Issues
- **Accumulator Complexity**: The `while (accumulator >= timestep)` pattern fights against real time
- **Interpolation Overhead**: Visual smoothing requires complex state blending
- **Spiral of Death**: When physics can't keep up, the accumulator grows unbounded
- **Network Desync**: Floating-point time accumulation causes determinism issues
- **GPU Inefficiency**: Variable update counts per frame prevent optimal GPU scheduling

### Philosophical Mismatch
- Voxels are discrete, but we use continuous time
- Positions snap to grid, but we interpolate between them
- GPU prefers fixed workloads, but we have variable physics steps

## Proposed Solution

A discrete tick-based physics system that embraces the discrete nature of voxels and provides predictable, deterministic simulation.

### Core Architecture

```rust
/// Tick-based physics system with spatial awareness
pub struct TickPhysicsSystem {
    /// Base tick rate (e.g., 20 Hz = 50ms per tick)
    pub base_tick_rate: u32,
    
    /// Global tick counter
    pub current_tick: u64,
    
    /// Next tick deadline
    pub next_tick_time: Instant,
    
    /// Tick interval duration
    pub tick_interval: Duration,
    
    /// Spatial tick scheduler
    pub spatial_scheduler: SpatialTickScheduler,
    
    /// Event queue for tick-triggered events
    pub tick_events: VecDeque<TickEvent>,
}

/// Spatial scheduling for chunk-based ticking
pub struct SpatialTickScheduler {
    /// Chunk tick states indexed by Morton code
    pub chunk_ticks: HashMap<u64, ChunkTickState>,
    
    /// Priority queue for next chunks to tick
    pub tick_queue: BinaryHeap<(Reverse<u64>, ChunkPos)>,
    
    /// Temporal LOD zones
    pub lod_zones: TemporalLodZones,
}

/// Per-chunk tick tracking
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ChunkTickState {
    /// Last tick this chunk was updated
    pub last_tick: u64,
    
    /// Tick rate divisor (1 = every tick, 2 = every other, etc.)
    pub tick_divisor: u8,
    
    /// Activity level (for adaptive ticking)
    pub activity_level: u8,
    
    /// Flags for special tick behaviors
    pub flags: u16,
}
```

## Implementation Phases

### Phase 1: Core Tick System (2 weeks)

**Goal**: Replace accumulator with event-driven ticks

#### 1.1 Event-Driven Tick Loop
```rust
impl TickPhysicsSystem {
    pub fn update(&mut self, current_time: Instant) -> Option<u64> {
        // Check if it's time for a tick
        if current_time >= self.next_tick_time {
            // Calculate next tick time immediately (no drift)
            self.next_tick_time += self.tick_interval;
            
            // Increment global tick
            self.current_tick += 1;
            
            // Generate tick event
            self.tick_events.push_back(TickEvent {
                tick: self.current_tick,
                time: current_time,
            });
            
            return Some(self.current_tick);
        }
        
        None
    }
    
    pub fn execute_tick(&mut self, world: &mut World, tick: u64) {
        // Dispatch physics compute shader
        let chunks_to_tick = self.spatial_scheduler.get_chunks_for_tick(tick);
        
        dispatch_physics_compute(
            world,
            &chunks_to_tick,
            self.tick_interval.as_secs_f32(),
        );
        
        // Update chunk tick states
        for chunk_pos in &chunks_to_tick {
            self.spatial_scheduler.mark_ticked(*chunk_pos, tick);
        }
    }
}
```

#### 1.2 GPU Physics Kernel
```wgsl
struct TickParams {
    tick_number: u32,
    tick_duration: f32,
    chunk_count: u32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> params: TickParams;
@group(0) @binding(1) var<storage, read> chunk_list: array<ChunkPos>;
@group(0) @binding(2) var<storage, read_write> physics_data: array<PhysicsBody>;
@group(0) @binding(3) var<storage, read_write> collision_pairs: array<CollisionPair>;

@compute @workgroup_size(64)
fn physics_tick(
    @builtin(global_invocation_id) id: vec3<u32>,
) {
    let entity_idx = id.x;
    if entity_idx >= arrayLength(&physics_data) {
        return;
    }
    
    let body = &physics_data[entity_idx];
    let chunk_pos = world_to_chunk(body.position);
    
    // Check if this entity's chunk is being ticked
    if !chunk_in_tick_list(chunk_pos, &chunk_list) {
        return;
    }
    
    // Apply velocity (discrete integration)
    let new_position = body.position + body.velocity * params.tick_duration;
    
    // Voxel grid collision
    let collision = check_voxel_collision(new_position);
    if !collision.hit {
        physics_data[entity_idx].position = new_position;
        physics_data[entity_idx].velocity += GRAVITY * params.tick_duration;
    } else {
        // Handle collision response
        handle_collision(&physics_data[entity_idx], collision);
    }
}
```

### Phase 2: Spatial Tick Cascading (2 weeks)

**Goal**: Only tick chunks that need updates

#### 2.1 Activity-Based Ticking
```rust
impl SpatialTickScheduler {
    pub fn should_chunk_tick(&self, chunk_pos: ChunkPos, current_tick: u64) -> bool {
        let morton = chunk_pos.to_morton();
        
        if let Some(state) = self.chunk_ticks.get(&morton) {
            // Check tick divisor
            if current_tick % state.tick_divisor as u64 != 0 {
                return false;
            }
            
            // Check activity threshold
            if state.activity_level < ACTIVITY_THRESHOLD {
                return false;
            }
            
            true
        } else {
            false
        }
    }
    
    pub fn cascade_tick_to_neighbors(
        &mut self,
        chunk_pos: ChunkPos,
        current_tick: u64,
    ) {
        const NEIGHBOR_OFFSETS: [(i32, i32, i32); 6] = [
            (1, 0, 0), (-1, 0, 0),
            (0, 1, 0), (0, -1, 0),
            (0, 0, 1), (0, 0, -1),
        ];
        
        for offset in NEIGHBOR_OFFSETS {
            let neighbor = chunk_pos.offset(offset.0, offset.1, offset.2);
            let morton = neighbor.to_morton();
            
            if let Some(state) = self.chunk_ticks.get_mut(&morton) {
                // Increase activity if neighbor is active
                state.activity_level = state.activity_level.saturating_add(1);
                
                // Schedule for next appropriate tick
                let next_tick = ((current_tick / state.tick_divisor as u64) + 1) 
                    * state.tick_divisor as u64;
                self.tick_queue.push((Reverse(next_tick), neighbor));
            }
        }
    }
}
```

#### 2.2 Activity Detection
```rust
bitflags! {
    pub struct ChunkActivity: u16 {
        const PLAYERS_PRESENT = 0x0001;
        const BLOCKS_CHANGED = 0x0002;
        const ENTITIES_MOVING = 0x0004;
        const FLUIDS_FLOWING = 0x0008;
        const REDSTONE_ACTIVE = 0x0010;
        const EXPLOSIONS = 0x0020;
        const PROJECTILES = 0x0040;
        const FALLING_BLOCKS = 0x0080;
    }
}

pub fn calculate_chunk_activity(
    chunk: &ChunkData,
    entities: &[Entity],
    events: &[WorldEvent],
) -> u8 {
    let mut activity = ChunkActivity::empty();
    
    // Check for players
    if entities.iter().any(|e| e.is_player()) {
        activity.insert(ChunkActivity::PLAYERS_PRESENT);
    }
    
    // Check for block changes
    if events.iter().any(|e| matches!(e, WorldEvent::BlockChanged(_))) {
        activity.insert(ChunkActivity::BLOCKS_CHANGED);
    }
    
    // Convert to activity level (0-255)
    (activity.bits().count_ones() * 32).min(255) as u8
}
```

### Phase 3: Visual Smoothing Without Interpolation (1 week)

**Goal**: Smooth rendering without interpolating physics states

#### 3.1 Critically Damped Spring System
```rust
/// Visual smoothing for render positions
pub struct VisualSmoother {
    /// Spring strength (0.0 - 1.0)
    pub spring_strength: f32,
    
    /// Damping factor
    pub damping: f32,
    
    /// Visual positions (separate from physics)
    pub visual_positions: Vec<Vec3>,
    
    /// Visual velocities for smoothing
    pub visual_velocities: Vec<Vec3>,
}

impl VisualSmoother {
    pub fn update(
        &mut self,
        physics_positions: &[Vec3],
        render_dt: f32,
    ) {
        for (i, &physics_pos) in physics_positions.iter().enumerate() {
            let visual_pos = self.visual_positions[i];
            let visual_vel = self.visual_velocities[i];
            
            // Calculate spring force
            let displacement = physics_pos - visual_pos;
            let spring_force = displacement * self.spring_strength;
            
            // Apply critically damped spring
            let damping_force = visual_vel * self.damping;
            let acceleration = spring_force - damping_force;
            
            // Update visual state
            self.visual_velocities[i] = visual_vel + acceleration * render_dt;
            self.visual_positions[i] = visual_pos + self.visual_velocities[i] * render_dt;
        }
    }
    
    pub fn get_visual_position(&self, entity_id: usize) -> Vec3 {
        self.visual_positions[entity_id]
    }
}
```

#### 3.2 Render Integration
```rust
pub fn render_frame(
    renderer: &mut Renderer,
    physics: &TickPhysicsSystem,
    smoother: &mut VisualSmoother,
    render_dt: f32,
) {
    // Update visual positions
    let physics_positions = physics.get_current_positions();
    smoother.update(&physics_positions, render_dt);
    
    // Render at smoothed positions
    for (entity_id, entity) in entities.iter().enumerate() {
        let visual_pos = smoother.get_visual_position(entity_id);
        
        // Blocks and static objects use physics position directly
        // Moving entities use smoothed position
        let render_pos = if entity.is_static() {
            physics_positions[entity_id]
        } else {
            visual_pos
        };
        
        renderer.draw_entity(entity, render_pos);
    }
}
```

### Phase 4: Temporal LOD System (2 weeks)

**Goal**: Variable tick rates based on distance and importance

#### 4.1 LOD Zone Definition
```rust
pub struct TemporalLodZones {
    /// Player positions for LOD calculation
    pub player_positions: Vec<Vec3>,
    
    /// LOD configuration
    pub zones: [LodZone; 4],
}

#[derive(Clone, Copy)]
pub struct LodZone {
    /// Maximum distance for this LOD
    pub max_distance: f32,
    
    /// Tick divisor (1 = every tick, 2 = every other, etc.)
    pub tick_divisor: u8,
    
    /// Minimum activity level to force tick
    pub activity_override: u8,
}

impl Default for TemporalLodZones {
    fn default() -> Self {
        Self {
            player_positions: Vec::new(),
            zones: [
                LodZone { max_distance: 50.0, tick_divisor: 1, activity_override: 0 },    // 20 Hz
                LodZone { max_distance: 100.0, tick_divisor: 2, activity_override: 50 },  // 10 Hz
                LodZone { max_distance: 200.0, tick_divisor: 4, activity_override: 100 }, // 5 Hz
                LodZone { max_distance: f32::INFINITY, tick_divisor: 20, activity_override: 200 }, // 1 Hz
            ],
        }
    }
}
```

#### 4.2 Adaptive Tick Assignment
```rust
impl TemporalLodZones {
    pub fn calculate_tick_divisor(&self, chunk_pos: ChunkPos) -> u8 {
        let chunk_center = chunk_pos.to_world_center();
        
        // Find minimum distance to any player
        let min_distance = self.player_positions
            .iter()
            .map(|&player_pos| (chunk_center - player_pos).length())
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(f32::INFINITY);
        
        // Find appropriate LOD zone
        for zone in &self.zones {
            if min_distance <= zone.max_distance {
                return zone.tick_divisor;
            }
        }
        
        // Fallback to slowest tick rate
        20
    }
    
    pub fn should_override_tick(&self, chunk_pos: ChunkPos, activity: u8) -> bool {
        let divisor = self.calculate_tick_divisor(chunk_pos);
        
        // Find the zone for this divisor
        for zone in &self.zones {
            if zone.tick_divisor == divisor {
                return activity >= zone.activity_override;
            }
        }
        
        false
    }
}
```

### Phase 5: Network Synchronization (2 weeks)

**Goal**: Deterministic network play using ticks

#### 5.1 Tick-Based Networking
```rust
/// Network packet for tick synchronization
#[derive(Serialize, Deserialize)]
pub struct TickPacket {
    /// The tick this packet represents
    pub tick: u64,
    
    /// Player inputs for this tick
    pub inputs: Vec<PlayerInput>,
    
    /// Authoritative state updates
    pub state_updates: Vec<StateUpdate>,
    
    /// Tick checksum for validation
    pub checksum: u32,
}

pub struct NetworkedTickSystem {
    /// Local tick system
    pub local_ticks: TickPhysicsSystem,
    
    /// Network tick buffer
    pub tick_buffer: VecDeque<TickPacket>,
    
    /// Tick delay for network buffering
    pub network_delay: u8,
    
    /// Rollback state for prediction
    pub rollback_states: VecDeque<WorldSnapshot>,
}

impl NetworkedTickSystem {
    pub fn process_network_tick(&mut self, packet: TickPacket) -> Result<()> {
        // Validate tick is in acceptable range
        let current = self.local_ticks.current_tick;
        if packet.tick > current + MAX_TICK_AHEAD {
            return Err(TickError::TooFarAhead);
        }
        
        // If packet is for past tick, rollback and replay
        if packet.tick < current {
            self.rollback_to_tick(packet.tick)?;
            self.replay_ticks_from(packet.tick)?;
        }
        
        // Buffer future ticks
        self.tick_buffer.push_back(packet);
        Ok(())
    }
}
```

## Critical Coding Philosophies

### 1. **Discrete Time, Discrete Space**
```rust
// ✅ CORRECT - Embrace discreteness
pub fn tick_physics(world: &mut World, tick: u64) {
    // Physics happens at exact tick boundaries
}

// ❌ WRONG - Fighting against discrete nature
pub fn update_physics(world: &mut World, dt: f32) {
    // Variable timestep breaks determinism
}
```

### 2. **Data-Oriented Programming (DOP)**
```rust
// ✅ CORRECT - Data and functions separate
fn process_chunk_tick(
    tick_state: &ChunkTickState,
    chunk_data: &mut ChunkData,
    tick: u64,
) {
    // Transform data, no methods
}

// ❌ WRONG - OOP style
impl Chunk {
    fn tick(&mut self) { } // NO METHODS!
}
```

### 3. **GPU-First Architecture**
- Each tick is one GPU dispatch
- No CPU physics simulation
- Batch all tick operations
- Use compute shaders for parallel processing

### 4. **Zero Fighting Time**
- No time accumulation
- No interpolation between physics states
- Clean tick boundaries
- Predictable workload per frame

### 5. **Performance Over Complexity**
- Simple tick counters over complex time tracking
- Spatial optimization over global updates
- Fixed workloads over variable steps

## Success Metrics

### Performance Targets
1. **Tick Consistency**
   - 20Hz ± 0.1ms jitter
   - Zero dropped ticks under normal load
   - < 5ms physics compute per tick

2. **Scalability**
   - 10,000+ active chunks
   - 100,000+ physics entities
   - Linear scaling with player count

3. **Visual Quality**
   - No visible stutter
   - Smooth movement without interpolation lag
   - Consistent feel across framerates

4. **Network Performance**
   - < 50ms input latency
   - Deterministic across all clients
   - Efficient rollback/replay

### Correctness Metrics
1. **Determinism**
   - Same tick = same result, always
   - Cross-platform consistency
   - Reproducible replays

2. **Stability**
   - No spiral of death
   - Graceful degradation under load
   - Bounded memory usage

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_tick_timing() {
    let mut system = TickPhysicsSystem::new(20); // 20Hz
    let start = Instant::now();
    
    // Simulate 1 second
    let mut ticks = 0;
    loop {
        let now = start + Duration::from_millis(ticks * 50 + 10);
        if let Some(tick) = system.update(now) {
            ticks += 1;
            assert_eq!(tick, ticks as u64);
        }
        if ticks >= 20 { break; }
    }
    
    assert_eq!(ticks, 20); // Exactly 20 ticks in 1 second
}

#[test]
fn test_spatial_ticking() {
    let mut scheduler = SpatialTickScheduler::new();
    let chunk = ChunkPos::new(0, 0, 0);
    
    // Set LOD zone
    scheduler.set_tick_divisor(chunk, 2); // Every other tick
    
    assert!(scheduler.should_chunk_tick(chunk, 2));
    assert!(!scheduler.should_chunk_tick(chunk, 3));
    assert!(scheduler.should_chunk_tick(chunk, 4));
}
```

### Integration Tests
```rust
#[test]
fn test_deterministic_simulation() {
    let world1 = create_test_world();
    let world2 = world1.clone();
    
    let mut physics1 = TickPhysicsSystem::new(20);
    let mut physics2 = TickPhysicsSystem::new(20);
    
    // Run 100 ticks
    for tick in 1..=100 {
        physics1.execute_tick(&mut world1, tick);
        physics2.execute_tick(&mut world2, tick);
    }
    
    // Worlds must be identical
    assert_eq!(world1.checksum(), world2.checksum());
}
```

### Performance Benchmarks
```rust
#[bench]
fn bench_tick_performance(b: &mut Bencher) {
    let world = create_large_world(); // 1000 chunks, 10000 entities
    let mut physics = TickPhysicsSystem::new(20);
    
    b.iter(|| {
        physics.execute_tick(&mut world, 1);
    });
}
```

## Migration Plan

### Phase 0: Preparation
1. Add feature flag `tick-physics`
2. Create tick system alongside existing
3. Add metrics collection

### Phase 1: Parallel Implementation
1. Implement core tick loop
2. Port physics to tick-based
3. Add visual smoothing
4. Compare with existing system

### Phase 2: Gradual Rollout
1. Enable for new worlds
2. Migrate existing worlds
3. Remove old accumulator system

### Phase 3: Optimization
1. Profile real usage
2. Tune LOD zones
3. Optimize GPU kernels

## Risk Mitigation

### Technical Risks
1. **Visual Stuttering**
   - Mitigation: Critically damped springs
   - Fallback: Optional interpolation

2. **Network Complexity**
   - Mitigation: Start with lockstep
   - Gradual rollback/prediction

3. **GPU Scheduling**
   - Mitigation: Adaptive tick rates
   - Dynamic load balancing

### Performance Risks
1. **Fixed Cost Too High**
   - Mitigation: Spatial optimization
   - Temporal LOD system

2. **Memory Usage**
   - Mitigation: Chunk pooling
   - State compression

## Future Enhancements

### Advanced Tick Scheduling
- Predictive ticking based on player movement
- Speculative execution for low-latency feel
- Multi-resolution temporal grids

### GPU Optimizations
- Persistent threads for physics
- Async compute overlap
- Hardware scheduling integration

### Network Features
- Delta compression
- Tick interpolation for spectators
- Hierarchical state synchronization

## Conclusion

The tick-based physics system aligns perfectly with Hearth Engine's discrete voxel world and GPU-first architecture. By embracing ticks instead of fighting continuous time, we achieve:

- **Simplicity**: No complex time accumulation
- **Performance**: Predictable GPU workloads
- **Determinism**: Perfect for networked play
- **Scalability**: Spatial and temporal optimization

This system will support 10,000+ concurrent players with consistent 144+ FPS performance while maintaining the intuitive, responsive feel essential for engaging gameplay.

Remember: Time is discrete. Space is discrete. Embrace the tick.