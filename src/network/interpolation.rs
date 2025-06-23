use crate::constants::network_constants::{INTERPOLATION_DELAY_MS, MAX_SNAPSHOTS};
use glam::{Quat, Vec3};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Position snapshot for interpolation
#[derive(Debug, Clone)]
pub struct PositionSnapshot {
    pub timestamp: Instant,
    pub server_tick: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
}

/// Interpolates entity positions between network updates
pub struct EntityInterpolator {
    /// History of position snapshots
    snapshots: VecDeque<PositionSnapshot>,
    /// Current interpolation time
    current_time: Instant,
    /// Interpolation delay
    interpolation_delay: Duration,
    /// Last interpolated position
    last_position: Vec3,
    /// Last interpolated rotation
    last_rotation: Quat,
    /// Whether to use extrapolation
    allow_extrapolation: bool,
}

impl EntityInterpolator {
    pub fn new() -> Self {
        Self {
            snapshots: VecDeque::with_capacity(MAX_SNAPSHOTS),
            current_time: Instant::now(),
            interpolation_delay: Duration::from_millis(INTERPOLATION_DELAY_MS),
            last_position: Vec3::ZERO,
            last_rotation: Quat::IDENTITY,
            allow_extrapolation: true,
        }
    }

    /// Get the current interpolation delay in milliseconds
    pub fn get_delay_ms(&self) -> u64 {
        self.interpolation_delay.as_millis() as u64
    }
}

// DOP functions for EntityInterpolator
/// Add a new position snapshot (DOP)
pub fn entity_interpolator_add_snapshot(
    interpolator: &mut EntityInterpolator,
    snapshot: PositionSnapshot,
) {
    // Remove old snapshots
    while interpolator.snapshots.len() >= MAX_SNAPSHOTS {
        interpolator.snapshots.pop_front();
    }

    // Add new snapshot
    interpolator.snapshots.push_back(snapshot);
}

/// Get interpolated position and rotation (DOP)
pub fn entity_interpolator_get_interpolated(
    interpolator: &mut EntityInterpolator,
    now: Instant,
) -> (Vec3, Quat) {
    interpolator.current_time = now;

    // Calculate render time (current time - interpolation delay)
    let render_time = now - interpolator.interpolation_delay;

    // Find the two snapshots to interpolate between
    let mut from_snapshot = None;
    let mut to_snapshot = None;

    for i in 0..interpolator.snapshots.len() {
        if interpolator.snapshots[i].timestamp <= render_time {
            from_snapshot = Some(i);
        } else if from_snapshot.is_some() && to_snapshot.is_none() {
            to_snapshot = Some(i);
            break;
        }
    }

    match (from_snapshot, to_snapshot) {
        (Some(from_idx), Some(to_idx)) => {
            // Interpolate between two snapshots
            let from = &interpolator.snapshots[from_idx];
            let to = &interpolator.snapshots[to_idx];

            let total_time = to.timestamp.duration_since(from.timestamp).as_secs_f32();
            let elapsed_time = render_time.duration_since(from.timestamp).as_secs_f32();
            let t = if total_time > 0.0 {
                elapsed_time / total_time
            } else {
                0.0
            };
            let t = t.clamp(0.0, 1.0);

            let position = from.position.lerp(to.position, t);
            let rotation = from.rotation.slerp(to.rotation, t);

            interpolator.last_position = position;
            interpolator.last_rotation = rotation;

            (position, rotation)
        }
        (Some(from_idx), None) if interpolator.allow_extrapolation => {
            // Extrapolate from the last snapshot
            let from = &interpolator.snapshots[from_idx];
            let elapsed = render_time.duration_since(from.timestamp).as_secs_f32();

            // Extrapolate position using velocity
            let position = from.position + from.velocity * elapsed;
            let rotation = from.rotation; // Don't extrapolate rotation

            interpolator.last_position = position;
            interpolator.last_rotation = rotation;

            (position, rotation)
        }
        _ => {
            // No valid snapshots, return last known position
            (interpolator.last_position, interpolator.last_rotation)
        }
    }
}

/// Set interpolation delay (DOP)
pub fn entity_interpolator_set_interpolation_delay(
    interpolator: &mut EntityInterpolator,
    delay_ms: u64,
) {
    interpolator.interpolation_delay = Duration::from_millis(delay_ms);
}

/// Enable or disable extrapolation (DOP)
pub fn entity_interpolator_set_extrapolation(interpolator: &mut EntityInterpolator, enabled: bool) {
    interpolator.allow_extrapolation = enabled;
}

/// Clear all snapshots (DOP)
pub fn entity_interpolator_clear(interpolator: &mut EntityInterpolator) {
    interpolator.snapshots.clear();
}

/// Manages interpolation for multiple entities
pub struct InterpolationManager {
    /// Interpolators for each entity
    interpolators: std::collections::HashMap<u32, EntityInterpolator>,
    /// Global interpolation settings
    global_delay_ms: u64,
    global_extrapolation: bool,
}

impl InterpolationManager {
    pub fn new() -> Self {
        Self {
            interpolators: std::collections::HashMap::new(),
            global_delay_ms: INTERPOLATION_DELAY_MS,
            global_extrapolation: true,
        }
    }
}

// DOP functions for InterpolationManager
/// Add or update an entity snapshot (DOP)
pub fn interpolation_manager_add_snapshot(
    manager: &mut InterpolationManager,
    entity_id: u32,
    snapshot: PositionSnapshot,
) {
    let interpolator = manager.interpolators.entry(entity_id).or_insert_with(|| {
        let mut interp = EntityInterpolator::new();
        entity_interpolator_set_interpolation_delay(&mut interp, manager.global_delay_ms);
        entity_interpolator_set_extrapolation(&mut interp, manager.global_extrapolation);
        interp
    });

    entity_interpolator_add_snapshot(interpolator, snapshot);
}

/// Get interpolated position for an entity (DOP)
pub fn interpolation_manager_get_interpolated(
    manager: &mut InterpolationManager,
    entity_id: u32,
    now: Instant,
) -> Option<(Vec3, Quat)> {
    manager
        .interpolators
        .get_mut(&entity_id)
        .map(|interp| entity_interpolator_get_interpolated(interp, now))
}

/// Remove an entity's interpolator (DOP)
pub fn interpolation_manager_remove_entity(manager: &mut InterpolationManager, entity_id: u32) {
    manager.interpolators.remove(&entity_id);
}

/// Set global interpolation delay (DOP)
pub fn interpolation_manager_set_global_delay(manager: &mut InterpolationManager, delay_ms: u64) {
    manager.global_delay_ms = delay_ms;
    for interpolator in manager.interpolators.values_mut() {
        entity_interpolator_set_interpolation_delay(interpolator, delay_ms);
    }
}

/// Set global extrapolation setting (DOP)
pub fn interpolation_manager_set_global_extrapolation(
    manager: &mut InterpolationManager,
    enabled: bool,
) {
    manager.global_extrapolation = enabled;
    for interpolator in manager.interpolators.values_mut() {
        entity_interpolator_set_extrapolation(interpolator, enabled);
    }
}

/// Auto-adjust delay based on network conditions (DOP)
pub fn interpolation_manager_auto_adjust_delay(
    manager: &mut InterpolationManager,
    average_ping_ms: u32,
    jitter_ms: u32,
) {
    // Calculate optimal delay based on ping and jitter
    // Higher jitter requires more delay to prevent stuttering
    let optimal_delay = average_ping_ms / 2 + jitter_ms * 2;
    let optimal_delay = optimal_delay.clamp(50, 200); // Keep between 50-200ms

    interpolation_manager_set_global_delay(manager, optimal_delay as u64);
}
