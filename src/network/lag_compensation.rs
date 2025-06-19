use std::collections::VecDeque;
use std::time::{Duration, Instant};
use crate::{VoxelPos, BlockId};
use glam::{Vec3, Quat};

/// Maximum history to keep for lag compensation (milliseconds)
const MAX_HISTORY_MS: u64 = 1000;

/// Number of history snapshots to keep
const MAX_HISTORY_SNAPSHOTS: usize = 50;

/// Player state at a specific time
#[derive(Debug, Clone)]
pub struct PlayerStateSnapshot {
    pub timestamp: Instant,
    pub server_tick: u32,
    pub player_id: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub hitbox_min: Vec3,
    pub hitbox_max: Vec3,
}

/// World state at a specific time
#[derive(Debug, Clone)]
pub struct WorldStateSnapshot {
    pub timestamp: Instant,
    pub server_tick: u32,
    pub block_changes: Vec<BlockChange>,
}

/// A single block change
#[derive(Debug, Clone)]
pub struct BlockChange {
    pub position: VoxelPos,
    pub old_block: BlockId,
    pub new_block: BlockId,
}

/// Lag compensation system for server-side hit validation
pub struct LagCompensation {
    /// History of player states
    player_history: std::collections::HashMap<u32, VecDeque<PlayerStateSnapshot>>,
    /// History of world changes
    world_history: VecDeque<WorldStateSnapshot>,
    /// Current server time
    server_time: Instant,
    /// Maximum rewind time
    max_rewind_time: Duration,
}

impl LagCompensation {
    pub fn new() -> Self {
        Self {
            player_history: std::collections::HashMap::new(),
            world_history: VecDeque::with_capacity(MAX_HISTORY_SNAPSHOTS),
            server_time: Instant::now(),
            max_rewind_time: Duration::from_millis(MAX_HISTORY_MS),
        }
    }
    
    
    
    
    /// Rewind and validate a player action
    pub fn validate_action<F, R>(
        &self,
        player_id: u32,
        action_timestamp: Instant,
        client_ping_ms: u32,
        validation_fn: F,
    ) -> Option<R>
    where
        F: FnOnce(&PlayerStateSnapshot, &[PlayerStateSnapshot]) -> Option<R>,
    {
        // Calculate the actual time when the action happened on the server
        let rewind_time = self.server_time.checked_sub(Duration::from_millis(client_ping_ms as u64 / 2))
            .unwrap_or(self.server_time);
        
        // Don't rewind too far
        if self.server_time.duration_since(rewind_time) > self.max_rewind_time {
            return None;
        }
        
        // Get player state at rewind time
        let player_state = self.get_player_state_at_time(player_id, rewind_time)?;
        
        // Get all other player states at that time
        let mut other_players = Vec::new();
        for (&other_id, history) in &self.player_history {
            if other_id != player_id {
                if let Some(state) = self.get_state_from_history(history, rewind_time) {
                    other_players.push(state);
                }
            }
        }
        
        // Validate the action
        validation_fn(&player_state, &other_players)
    }
    
    /// Get player state at a specific time
    fn get_player_state_at_time(&self, player_id: u32, time: Instant) -> Option<PlayerStateSnapshot> {
        let history = self.player_history.get(&player_id)?;
        self.get_state_from_history(history, time)
    }
    
    /// Get state from history at a specific time
    fn get_state_from_history(&self, history: &VecDeque<PlayerStateSnapshot>, time: Instant) -> Option<PlayerStateSnapshot> {
        // Find the two states to interpolate between
        let mut before = None;
        let mut after = None;
        
        for snapshot in history {
            if snapshot.timestamp <= time {
                before = Some(snapshot);
            } else {
                after = Some(snapshot);
                break;
            }
        }
        
        match (before, after) {
            (Some(before_state), Some(after_state)) => {
                // Interpolate between states
                let total_time = after_state.timestamp.duration_since(before_state.timestamp).as_secs_f32();
                let elapsed = time.duration_since(before_state.timestamp).as_secs_f32();
                let t = if total_time > 0.0 { elapsed / total_time } else { 0.0 };
                
                Some(PlayerStateSnapshot {
                    timestamp: time,
                    server_tick: before_state.server_tick,
                    player_id: before_state.player_id,
                    position: before_state.position.lerp(after_state.position, t),
                    rotation: before_state.rotation.slerp(after_state.rotation, t),
                    velocity: before_state.velocity.lerp(after_state.velocity, t),
                    hitbox_min: before_state.hitbox_min,
                    hitbox_max: before_state.hitbox_max,
                })
            }
            (Some(state), None) => {
                // Use the last known state
                Some(state.clone())
            }
            _ => None,
        }
    }
    
    /// Validate a hit with lag compensation
    pub fn validate_hit(
        &self,
        shooter_id: u32,
        target_id: u32,
        shot_origin: Vec3,
        shot_direction: Vec3,
        max_distance: f32,
        ping_ms: u32,
    ) -> Option<HitValidation> {
        self.validate_action(shooter_id, self.server_time, ping_ms, |shooter_state, other_players| {
            // Find the target player
            let target_state = other_players.iter()
                .find(|p| p.player_id == target_id)?;
            
            // Perform raycast from shooter position
            let hit_point = self.raycast_vs_hitbox(
                shot_origin,
                shot_direction,
                max_distance,
                target_state.position,
                target_state.hitbox_min,
                target_state.hitbox_max,
            )?;
            
            Some(HitValidation {
                hit: true,
                hit_point,
                distance: (hit_point - shot_origin).length(),
            })
        })
    }
    
    /// Simple raycast vs AABB
    fn raycast_vs_hitbox(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        hitbox_center: Vec3,
        hitbox_min: Vec3,
        hitbox_max: Vec3,
    ) -> Option<Vec3> {
        // Transform to hitbox space
        let ray_origin = origin - hitbox_center;
        
        // Calculate intersection with AABB
        let inv_dir = Vec3::new(
            if direction.x != 0.0 { 1.0 / direction.x } else { f32::INFINITY },
            if direction.y != 0.0 { 1.0 / direction.y } else { f32::INFINITY },
            if direction.z != 0.0 { 1.0 / direction.z } else { f32::INFINITY },
        );
        
        let t1 = (hitbox_min - ray_origin) * inv_dir;
        let t2 = (hitbox_max - ray_origin) * inv_dir;
        
        let tmin = t1.min(t2);
        let tmax = t1.max(t2);
        
        let tmin = tmin.x.max(tmin.y).max(tmin.z).max(0.0);
        let tmax = tmax.x.min(tmax.y).min(tmax.z);
        
        if tmin <= tmax && tmin <= max_distance {
            Some(origin + direction * tmin)
        } else {
            None
        }
    }
    
}

/// Add a player state snapshot
pub fn lag_compensation_add_player_snapshot(lag_comp: &mut LagCompensation, snapshot: PlayerStateSnapshot) {
    let history = lag_comp.player_history
        .entry(snapshot.player_id)
        .or_insert_with(|| VecDeque::with_capacity(MAX_HISTORY_SNAPSHOTS));
    
    // Remove old snapshots
    while history.len() >= MAX_HISTORY_SNAPSHOTS {
        history.pop_front();
    }
    
    // Remove snapshots older than max history
    let cutoff_time = lag_comp.server_time.checked_sub(lag_comp.max_rewind_time)
        .unwrap_or(lag_comp.server_time);
    while let Some(front) = history.front() {
        if front.timestamp < cutoff_time {
            history.pop_front();
        } else {
            break;
        }
    }
    
    history.push_back(snapshot);
}

/// Add a world state snapshot
pub fn lag_compensation_add_world_snapshot(lag_comp: &mut LagCompensation, snapshot: WorldStateSnapshot) {
    // Remove old snapshots
    while lag_comp.world_history.len() >= MAX_HISTORY_SNAPSHOTS {
        lag_comp.world_history.pop_front();
    }
    
    lag_comp.world_history.push_back(snapshot);
}

/// Update server time
pub fn lag_compensation_update_time(lag_comp: &mut LagCompensation, now: Instant) {
    lag_comp.server_time = now;
}

/// Clean up old history
pub fn lag_compensation_cleanup_old_history(lag_comp: &mut LagCompensation) {
    let cutoff_time = lag_comp.server_time.checked_sub(lag_comp.max_rewind_time)
        .unwrap_or(lag_comp.server_time);
    
    // Clean player history
    for history in lag_comp.player_history.values_mut() {
        while let Some(front) = history.front() {
            if front.timestamp < cutoff_time {
                history.pop_front();
            } else {
                break;
            }
        }
    }
    
    // Clean world history
    while let Some(front) = lag_comp.world_history.front() {
        if front.timestamp < cutoff_time {
            lag_comp.world_history.pop_front();
        } else {
            break;
        }
    }
}

/// Result of hit validation
#[derive(Debug, Clone)]
pub struct HitValidation {
    pub hit: bool,
    pub hit_point: Vec3,
    pub distance: f32,
}