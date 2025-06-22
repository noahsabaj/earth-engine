//! Data-Oriented Player Data Structures for Cache Efficiency
//!
//! This module implements cache-optimized player data structures following DOP principles.
//! The design separates hot and cold data paths, uses Structure of Arrays (SOA) layouts,
//! and ensures optimal cache line utilization for high-performance player operations.

use crate::persistence::{PersistenceError, PersistenceResult};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum number of concurrent players for memory allocation
pub const MAX_PLAYERS: usize = 10_000;

/// Cache line size for memory alignment optimizations
pub const CACHE_LINE_SIZE: usize = 64;

/// Number of cache lines to reserve for hot player data
pub const HOT_DATA_CACHE_LINES: usize = 4;

/// Player hot data - frequently accessed together for cache efficiency
/// Layout fits in two cache lines (128 bytes total with alignment)
/// First cache line: position + velocity + rotation + health/hunger
/// Second cache line: experience/level + flags with room for future expansion
#[repr(C, align(64))] // Cache line aligned
#[derive(Debug, Clone, Copy)]
pub struct PlayerHotData {
    /// 3D position in world space (12 bytes)
    pub position: Vec3,
    /// 3D velocity vector (12 bytes)
    pub velocity: Vec3,
    /// Rotation quaternion (16 bytes)
    pub rotation: Quat,
    /// Player health (4 bytes)
    pub health: f32,
    /// Player hunger/food level (4 bytes)
    pub hunger: f32,
    /// Experience points (4 bytes)
    pub experience: u32,
    /// Experience level (4 bytes)
    pub level: u32,
    /// Game mode (1 byte)
    pub game_mode: u8,
    /// Movement state flags (1 byte)
    pub movement_state: u8,
    /// Dirty flags for networking (1 byte)
    pub dirty_flags: u8,
    /// Reserved padding to fill cache line alignment (61 bytes)
    pub _padding: [u8; 61],
}

/// Player cold data - infrequently accessed metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerColdData {
    /// Player UUID (heap allocated)
    pub uuid: String,
    /// Player username (heap allocated)
    pub username: String,
    /// Spawn position
    pub spawn_position: Option<Vec3>,
    /// Last login timestamp
    pub last_login: u64,
    /// Total play time in seconds
    pub play_time: u64,
    /// Player statistics (cold data)
    pub stats: PlayerStatsData,
    /// Active potion effects
    pub effects: Vec<PotionEffectData>,
    /// Unlocked achievements
    pub achievements: Vec<String>,
    /// Custom player tags
    pub tags: Vec<String>,
}

/// Player statistics stored separately from hot data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerStatsData {
    pub blocks_broken: u64,
    pub blocks_placed: u64,
    pub distance_walked: f64,
    pub distance_sprinted: f64,
    pub distance_fallen: f64,
    pub distance_climbed: f64,
    pub distance_flown: f64,
    pub jumps: u64,
    pub deaths: u32,
    pub mob_kills: u32,
    pub player_kills: u32,
    pub damage_dealt: f64,
    pub damage_taken: f64,
    pub play_time: u64,
}

/// Potion effect data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotionEffectData {
    pub effect_type: String,
    pub amplifier: u8,
    pub duration: f32,
}

/// Data-Oriented Player Buffer using Structure of Arrays layout
pub struct PlayerDataBuffer {
    /// Current number of active players
    pub count: usize,
    /// Maximum capacity
    pub capacity: usize,

    /// Hot data buffers - cache-aligned and co-located
    /// Position components (SIMD-friendly)
    pub position_x: Vec<f32>,
    pub position_y: Vec<f32>,
    pub position_z: Vec<f32>,

    /// Velocity components (SIMD-friendly)
    pub velocity_x: Vec<f32>,
    pub velocity_y: Vec<f32>,
    pub velocity_z: Vec<f32>,

    /// Rotation components (quaternion)
    pub rotation_x: Vec<f32>,
    pub rotation_y: Vec<f32>,
    pub rotation_z: Vec<f32>,
    pub rotation_w: Vec<f32>,

    /// Health and hunger (frequently checked together)
    pub health: Vec<f32>,
    pub hunger: Vec<f32>,

    /// Experience data (moderately hot)
    pub experience: Vec<u32>,
    pub level: Vec<u32>,

    /// Game state flags (packed into single bytes)
    pub game_mode: Vec<u8>,
    pub movement_state: Vec<u8>,
    pub dirty_flags: Vec<u8>,

    /// Player ID mapping (hot path for lookups)
    pub player_ids: Vec<u32>,

    /// Cold data storage (separate allocation pattern)
    pub cold_data: HashMap<u32, PlayerColdData>,

    /// Free slot indices for efficient allocation
    pub free_slots: Vec<usize>,
}

impl Default for PlayerHotData {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 100.0, 0.0),
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            health: 20.0,
            hunger: 20.0,
            experience: 0,
            level: 0,
            game_mode: 0,      // Survival
            movement_state: 0, // Normal
            dirty_flags: 0,
            _padding: [0; 61],
        }
    }
}

impl PlayerDataBuffer {
    /// Create a new player data buffer with specified capacity
    pub fn new(capacity: usize) -> Self {
        let safe_capacity = capacity.min(MAX_PLAYERS);

        Self {
            count: 0,
            capacity: safe_capacity,

            // Pre-allocate all hot data arrays
            position_x: Vec::with_capacity(safe_capacity),
            position_y: Vec::with_capacity(safe_capacity),
            position_z: Vec::with_capacity(safe_capacity),

            velocity_x: Vec::with_capacity(safe_capacity),
            velocity_y: Vec::with_capacity(safe_capacity),
            velocity_z: Vec::with_capacity(safe_capacity),

            rotation_x: Vec::with_capacity(safe_capacity),
            rotation_y: Vec::with_capacity(safe_capacity),
            rotation_z: Vec::with_capacity(safe_capacity),
            rotation_w: Vec::with_capacity(safe_capacity),

            health: Vec::with_capacity(safe_capacity),
            hunger: Vec::with_capacity(safe_capacity),

            experience: Vec::with_capacity(safe_capacity),
            level: Vec::with_capacity(safe_capacity),

            game_mode: Vec::with_capacity(safe_capacity),
            movement_state: Vec::with_capacity(safe_capacity),
            dirty_flags: Vec::with_capacity(safe_capacity),

            player_ids: Vec::with_capacity(safe_capacity),

            cold_data: HashMap::with_capacity(safe_capacity),
            free_slots: Vec::new(),
        }
    }

    /// Add a new player to the buffer, returns index
    pub fn add_player(
        &mut self,
        player_id: u32,
        hot_data: PlayerHotData,
        cold_data: PlayerColdData,
    ) -> Option<usize> {
        // Use free slot if available, otherwise append
        let index = if let Some(slot) = self.free_slots.pop() {
            slot
        } else if self.count < self.capacity {
            let idx = self.count;
            self.count += 1;
            idx
        } else {
            return None; // Buffer full
        };

        // Ensure vectors are large enough
        self.ensure_capacity(index + 1);

        // Insert hot data into SOA buffers
        self.player_ids[index] = player_id;
        self.position_x[index] = hot_data.position.x;
        self.position_y[index] = hot_data.position.y;
        self.position_z[index] = hot_data.position.z;

        self.velocity_x[index] = hot_data.velocity.x;
        self.velocity_y[index] = hot_data.velocity.y;
        self.velocity_z[index] = hot_data.velocity.z;

        self.rotation_x[index] = hot_data.rotation.x;
        self.rotation_y[index] = hot_data.rotation.y;
        self.rotation_z[index] = hot_data.rotation.z;
        self.rotation_w[index] = hot_data.rotation.w;

        self.health[index] = hot_data.health;
        self.hunger[index] = hot_data.hunger;

        self.experience[index] = hot_data.experience;
        self.level[index] = hot_data.level;

        self.game_mode[index] = hot_data.game_mode;
        self.movement_state[index] = hot_data.movement_state;
        self.dirty_flags[index] = hot_data.dirty_flags;

        // Store cold data separately
        self.cold_data.insert(player_id, cold_data);

        Some(index)
    }

    /// Remove player from buffer
    pub fn remove_player(&mut self, index: usize) {
        if index >= self.count {
            return;
        }

        let player_id = self.player_ids[index];

        // Remove cold data
        self.cold_data.remove(&player_id);

        // Add slot to free list
        self.free_slots.push(index);

        // Mark slot as invalid
        self.player_ids[index] = u32::MAX;
    }

    /// Get hot data for a player at index
    pub fn get_hot_data(&self, index: usize) -> Option<PlayerHotData> {
        if index >= self.count || self.player_ids[index] == u32::MAX {
            return None;
        }

        Some(PlayerHotData {
            position: Vec3::new(
                self.position_x[index],
                self.position_y[index],
                self.position_z[index],
            ),
            velocity: Vec3::new(
                self.velocity_x[index],
                self.velocity_y[index],
                self.velocity_z[index],
            ),
            rotation: Quat::from_xyzw(
                self.rotation_x[index],
                self.rotation_y[index],
                self.rotation_z[index],
                self.rotation_w[index],
            ),
            health: self.health[index],
            hunger: self.hunger[index],
            experience: self.experience[index],
            level: self.level[index],
            game_mode: self.game_mode[index],
            movement_state: self.movement_state[index],
            dirty_flags: self.dirty_flags[index],
            _padding: [0; 61],
        })
    }

    /// Update position for a player (cache-efficient batch operation)
    pub fn update_position(&mut self, index: usize, position: Vec3) {
        if index < self.count && self.player_ids[index] != u32::MAX {
            self.position_x[index] = position.x;
            self.position_y[index] = position.y;
            self.position_z[index] = position.z;
            self.dirty_flags[index] |= DIRTY_POSITION;
        }
    }

    /// Update velocity for a player (cache-efficient batch operation)
    pub fn update_velocity(&mut self, index: usize, velocity: Vec3) {
        if index < self.count && self.player_ids[index] != u32::MAX {
            self.velocity_x[index] = velocity.x;
            self.velocity_y[index] = velocity.y;
            self.velocity_z[index] = velocity.z;
            self.dirty_flags[index] |= DIRTY_VELOCITY;
        }
    }

    /// Update health for a player
    pub fn update_health(&mut self, index: usize, health: f32) {
        if index < self.count && self.player_ids[index] != u32::MAX {
            self.health[index] = health;
            self.dirty_flags[index] |= DIRTY_HEALTH;
        }
    }

    /// Batch physics update - highly cache-efficient due to SOA layout
    pub fn update_physics(&mut self, dt: f32) {
        // Process all positions in a tight loop (excellent cache behavior)
        for i in 0..self.count {
            if self.player_ids[i] == u32::MAX {
                continue;
            }

            // Update position based on velocity (vectorizable)
            self.position_x[i] += self.velocity_x[i] * dt;
            self.position_y[i] += self.velocity_y[i] * dt;
            self.position_z[i] += self.velocity_z[i] * dt;
        }
    }

    /// Get players with dirty flags (networking optimization)
    pub fn get_dirty_players(&self, dirty_mask: u8) -> Vec<usize> {
        let mut dirty_players = Vec::new();

        // Single pass through dirty flags array (cache-friendly)
        for i in 0..self.count {
            if self.player_ids[i] != u32::MAX && (self.dirty_flags[i] & dirty_mask) != 0 {
                dirty_players.push(i);
            }
        }

        dirty_players
    }

    /// Clear dirty flags for a player
    pub fn clear_dirty_flags(&mut self, index: usize, flags: u8) {
        if index < self.count && self.player_ids[index] != u32::MAX {
            self.dirty_flags[index] &= !flags;
        }
    }

    /// Find player by ID (returns buffer index)
    pub fn find_player(&self, player_id: u32) -> Option<usize> {
        // Linear search through player_ids array (cache-friendly for small player counts)
        for i in 0..self.count {
            if self.player_ids[i] == player_id {
                return Some(i);
            }
        }
        None
    }

    /// Get cold data for a player
    pub fn get_cold_data(&self, player_id: u32) -> Option<&PlayerColdData> {
        self.cold_data.get(&player_id)
    }

    /// Get mutable cold data for a player
    pub fn get_cold_data_mut(&mut self, player_id: u32) -> Option<&mut PlayerColdData> {
        self.cold_data.get_mut(&player_id)
    }

    /// Get memory usage statistics
    pub fn memory_usage(&self) -> PlayerBufferMemoryStats {
        let hot_data_size = std::mem::size_of::<f32>() * self.capacity * 12  // position + velocity + rotation
            + std::mem::size_of::<f32>() * self.capacity * 2  // health + hunger
            + std::mem::size_of::<u32>() * self.capacity * 3  // experience + level + player_id
            + std::mem::size_of::<u8>() * self.capacity * 3; // game_mode + movement_state + dirty_flags

        let cold_data_size = self
            .cold_data
            .iter()
            .map(|(_, data)| estimate_cold_data_size(data))
            .sum::<usize>();

        PlayerBufferMemoryStats {
            hot_data_bytes: hot_data_size,
            cold_data_bytes: cold_data_size,
            total_bytes: hot_data_size + cold_data_size,
            active_players: self.count - self.free_slots.len(),
            capacity: self.capacity,
            cache_lines_used: (hot_data_size + CACHE_LINE_SIZE - 1) / CACHE_LINE_SIZE,
        }
    }

    /// Ensure vector capacity for given size
    fn ensure_capacity(&mut self, size: usize) {
        if self.position_x.len() < size {
            self.position_x.resize(size, 0.0);
            self.position_y.resize(size, 0.0);
            self.position_z.resize(size, 0.0);

            self.velocity_x.resize(size, 0.0);
            self.velocity_y.resize(size, 0.0);
            self.velocity_z.resize(size, 0.0);

            self.rotation_x.resize(size, 0.0);
            self.rotation_y.resize(size, 0.0);
            self.rotation_z.resize(size, 0.0);
            self.rotation_w.resize(size, 1.0);

            self.health.resize(size, 20.0);
            self.hunger.resize(size, 20.0);

            self.experience.resize(size, 0);
            self.level.resize(size, 0);

            self.game_mode.resize(size, 0);
            self.movement_state.resize(size, 0);
            self.dirty_flags.resize(size, 0);

            self.player_ids.resize(size, u32::MAX);
        }
    }
}

/// Dirty flags for tracking changes
pub const DIRTY_POSITION: u8 = 1 << 0;
pub const DIRTY_VELOCITY: u8 = 1 << 1;
pub const DIRTY_ROTATION: u8 = 1 << 2;
pub const DIRTY_HEALTH: u8 = 1 << 3;
pub const DIRTY_HUNGER: u8 = 1 << 4;
pub const DIRTY_EXPERIENCE: u8 = 1 << 5;
pub const DIRTY_LEVEL: u8 = 1 << 6;
pub const DIRTY_ALL: u8 = 0xFF;

/// Memory usage statistics for the player buffer
#[derive(Debug, Clone)]
pub struct PlayerBufferMemoryStats {
    pub hot_data_bytes: usize,
    pub cold_data_bytes: usize,
    pub total_bytes: usize,
    pub active_players: usize,
    pub capacity: usize,
    pub cache_lines_used: usize,
}

impl PlayerBufferMemoryStats {
    /// Get memory efficiency ratio (hot data / total data)
    pub fn hot_data_ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.hot_data_bytes as f64 / self.total_bytes as f64
        }
    }

    /// Get cache line utilization percentage
    pub fn cache_utilization(&self) -> f64 {
        if self.active_players == 0 {
            0.0
        } else {
            let ideal_cache_lines =
                (self.active_players * std::mem::size_of::<PlayerHotData>() + CACHE_LINE_SIZE - 1)
                    / CACHE_LINE_SIZE;
            ideal_cache_lines as f64 / self.cache_lines_used as f64
        }
    }
}

/// Estimate the size of cold data in memory
fn estimate_cold_data_size(data: &PlayerColdData) -> usize {
    data.uuid.len()
        + data.username.len()
        + std::mem::size_of_val(&data.spawn_position)
        + std::mem::size_of_val(&data.last_login)
        + std::mem::size_of_val(&data.play_time)
        + std::mem::size_of_val(&data.stats)
        + data.effects.len() * std::mem::size_of::<PotionEffectData>()
        + data.achievements.iter().map(|s| s.len()).sum::<usize>()
        + data.tags.iter().map(|s| s.len()).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_buffer_creation() {
        let buffer = PlayerDataBuffer::new(100);
        assert_eq!(buffer.capacity, 100);
        assert_eq!(buffer.count, 0);
    }

    #[test]
    fn test_player_hot_data_size() {
        // PlayerHotData is cache-line aligned (128 bytes = 2 cache lines)
        assert_eq!(std::mem::size_of::<PlayerHotData>(), CACHE_LINE_SIZE * 2);

        // Verify alignment
        assert_eq!(std::mem::align_of::<PlayerHotData>(), CACHE_LINE_SIZE);

        // Ensure it's still reasonably sized
        assert!(std::mem::size_of::<PlayerHotData>() <= 256);
    }

    #[test]
    fn test_add_remove_player() {
        let mut buffer = PlayerDataBuffer::new(10);

        let hot_data = PlayerHotData::default();
        let cold_data = PlayerColdData {
            uuid: "test-uuid".to_string(),
            username: "TestPlayer".to_string(),
            spawn_position: None,
            last_login: 0,
            play_time: 0,
            stats: PlayerStatsData::default(),
            effects: Vec::new(),
            achievements: Vec::new(),
            tags: Vec::new(),
        };

        // Add player
        let index = buffer
            .add_player(1, hot_data, cold_data)
            .expect("Failed to add player");
        assert_eq!(buffer.count, 1);
        assert_eq!(buffer.player_ids[index], 1);

        // Remove player
        buffer.remove_player(index);
        assert_eq!(buffer.free_slots.len(), 1);
        assert_eq!(buffer.player_ids[index], u32::MAX);
    }

    #[test]
    fn test_physics_update() {
        let mut buffer = PlayerDataBuffer::new(10);

        let hot_data = PlayerHotData {
            position: Vec3::new(0.0, 0.0, 0.0),
            velocity: Vec3::new(1.0, 2.0, 3.0),
            ..Default::default()
        };
        let cold_data = PlayerColdData {
            uuid: "test-uuid".to_string(),
            username: "TestPlayer".to_string(),
            spawn_position: None,
            last_login: 0,
            play_time: 0,
            stats: PlayerStatsData::default(),
            effects: Vec::new(),
            achievements: Vec::new(),
            tags: Vec::new(),
        };

        let index = buffer
            .add_player(1, hot_data, cold_data)
            .expect("[Test] Failed to add player to buffer");

        // Update physics
        buffer.update_physics(0.1);

        // Check updated position
        let updated_data = buffer
            .get_hot_data(index)
            .expect("[Test] Failed to get player hot data");
        assert_eq!(updated_data.position, Vec3::new(0.1, 0.2, 0.3));
    }

    #[test]
    fn test_memory_stats() {
        let buffer = PlayerDataBuffer::new(100);
        let stats = buffer.memory_usage();

        assert!(stats.hot_data_bytes > 0);
        assert_eq!(stats.active_players, 0);
        assert_eq!(stats.capacity, 100);
    }
}
