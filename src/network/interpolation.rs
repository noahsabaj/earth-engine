use std::collections::VecDeque;
use std::time::{Duration, Instant};
use glam::{Vec3, Quat};

/// Maximum number of position snapshots to keep
const MAX_SNAPSHOTS: usize = 20;

/// How far behind to render entities (in milliseconds)
const INTERPOLATION_DELAY_MS: u64 = 100;

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
    
    /// Add a new position snapshot
    pub fn add_snapshot(&mut self, snapshot: PositionSnapshot) {
        // Remove old snapshots
        while self.snapshots.len() >= MAX_SNAPSHOTS {
            self.snapshots.pop_front();
        }
        
        // Add new snapshot
        self.snapshots.push_back(snapshot);
    }
    
    /// Get interpolated position and rotation
    pub fn get_interpolated(&mut self, now: Instant) -> (Vec3, Quat) {
        self.current_time = now;
        
        // Calculate render time (current time - interpolation delay)
        let render_time = now - self.interpolation_delay;
        
        // Find the two snapshots to interpolate between
        let mut from_snapshot = None;
        let mut to_snapshot = None;
        
        for i in 0..self.snapshots.len() {
            if self.snapshots[i].timestamp <= render_time {
                from_snapshot = Some(i);
            } else if from_snapshot.is_some() && to_snapshot.is_none() {
                to_snapshot = Some(i);
                break;
            }
        }
        
        match (from_snapshot, to_snapshot) {
            (Some(from_idx), Some(to_idx)) => {
                // Interpolate between two snapshots
                let from = &self.snapshots[from_idx];
                let to = &self.snapshots[to_idx];
                
                let total_time = to.timestamp.duration_since(from.timestamp).as_secs_f32();
                let elapsed_time = render_time.duration_since(from.timestamp).as_secs_f32();
                let t = if total_time > 0.0 { elapsed_time / total_time } else { 0.0 };
                let t = t.clamp(0.0, 1.0);
                
                let position = from.position.lerp(to.position, t);
                let rotation = from.rotation.slerp(to.rotation, t);
                
                self.last_position = position;
                self.last_rotation = rotation;
                
                (position, rotation)
            }
            (Some(from_idx), None) if self.allow_extrapolation => {
                // Extrapolate from the last snapshot
                let from = &self.snapshots[from_idx];
                let elapsed = render_time.duration_since(from.timestamp).as_secs_f32();
                
                // Extrapolate position using velocity
                let position = from.position + from.velocity * elapsed;
                let rotation = from.rotation; // Don't extrapolate rotation
                
                self.last_position = position;
                self.last_rotation = rotation;
                
                (position, rotation)
            }
            _ => {
                // No valid snapshots, return last known position
                (self.last_position, self.last_rotation)
            }
        }
    }
    
    /// Set interpolation delay
    pub fn set_interpolation_delay(&mut self, delay_ms: u64) {
        self.interpolation_delay = Duration::from_millis(delay_ms);
    }
    
    /// Enable or disable extrapolation
    pub fn set_extrapolation(&mut self, enabled: bool) {
        self.allow_extrapolation = enabled;
    }
    
    /// Clear all snapshots
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
    
    /// Get the current interpolation delay in milliseconds
    pub fn get_delay_ms(&self) -> u64 {
        self.interpolation_delay.as_millis() as u64
    }
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
    
    /// Add or update an entity snapshot
    pub fn add_snapshot(&mut self, entity_id: u32, snapshot: PositionSnapshot) {
        let interpolator = self.interpolators
            .entry(entity_id)
            .or_insert_with(|| {
                let mut interp = EntityInterpolator::new();
                interp.set_interpolation_delay(self.global_delay_ms);
                interp.set_extrapolation(self.global_extrapolation);
                interp
            });
        
        interpolator.add_snapshot(snapshot);
    }
    
    /// Get interpolated position for an entity
    pub fn get_interpolated(&mut self, entity_id: u32, now: Instant) -> Option<(Vec3, Quat)> {
        self.interpolators.get_mut(&entity_id)
            .map(|interp| interp.get_interpolated(now))
    }
    
    /// Remove an entity's interpolator
    pub fn remove_entity(&mut self, entity_id: u32) {
        self.interpolators.remove(&entity_id);
    }
    
    /// Set global interpolation delay
    pub fn set_global_delay(&mut self, delay_ms: u64) {
        self.global_delay_ms = delay_ms;
        for interpolator in self.interpolators.values_mut() {
            interpolator.set_interpolation_delay(delay_ms);
        }
    }
    
    /// Set global extrapolation setting
    pub fn set_global_extrapolation(&mut self, enabled: bool) {
        self.global_extrapolation = enabled;
        for interpolator in self.interpolators.values_mut() {
            interpolator.set_extrapolation(enabled);
        }
    }
    
    /// Auto-adjust delay based on network conditions
    pub fn auto_adjust_delay(&mut self, average_ping_ms: u32, jitter_ms: u32) {
        // Calculate optimal delay based on ping and jitter
        // Higher jitter requires more delay to prevent stuttering
        let optimal_delay = average_ping_ms / 2 + jitter_ms * 2;
        let optimal_delay = optimal_delay.clamp(50, 200); // Keep between 50-200ms
        
        self.set_global_delay(optimal_delay as u64);
    }
}