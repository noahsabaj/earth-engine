use std::collections::{HashMap, HashSet};
use glam::Vec3;
use crate::world::VoxelPos;
use super::error::{NetworkResult, NetworkErrorContext};

/// Maximum view distance for entity updates (meters)
const MAX_ENTITY_VIEW_DISTANCE: f32 = 128.0;

/// Maximum view distance for block updates (chunks)
const MAX_CHUNK_VIEW_DISTANCE: i32 = 8;

/// Update frequency for interest management (Hz)
const INTEREST_UPDATE_RATE: f32 = 2.0;

/// A region in the world for spatial partitioning
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct RegionCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl RegionCoord {
    /// Size of each region in world units
    pub const REGION_SIZE: f32 = 64.0;
    
    /// Create region coord from world position
    pub fn from_position(pos: Vec3) -> Self {
        Self {
            x: (pos.x / Self::REGION_SIZE).floor() as i32,
            y: (pos.y / Self::REGION_SIZE).floor() as i32,
            z: (pos.z / Self::REGION_SIZE).floor() as i32,
        }
    }
    
    /// Get neighboring regions (including diagonals)
    pub fn get_neighbors(&self) -> Vec<RegionCoord> {
        let mut neighbors = Vec::with_capacity(26);
        
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    neighbors.push(RegionCoord {
                        x: self.x + dx,
                        y: self.y + dy,
                        z: self.z + dz,
                    });
                }
            }
        }
        
        neighbors
    }
}

/// Interest management for a single player
#[derive(Debug)]
pub struct PlayerInterest {
    /// Player ID
    player_id: u32,
    /// Current player position
    position: Vec3,
    /// Current region
    current_region: RegionCoord,
    /// Entities this player is interested in
    interested_entities: HashSet<u32>,
    /// Chunks this player is interested in
    interested_chunks: HashSet<VoxelPos>,
    /// Custom view distance override
    view_distance_override: Option<f32>,
}

impl PlayerInterest {
    pub fn new(player_id: u32, position: Vec3) -> Self {
        Self {
            player_id,
            position,
            current_region: RegionCoord::from_position(position),
            interested_entities: HashSet::new(),
            interested_chunks: HashSet::new(),
            view_distance_override: None,
        }
    }
    
    /// Update player position
    pub fn update_position(&mut self, position: Vec3) {
        self.position = position;
        self.current_region = RegionCoord::from_position(position);
    }
    
    /// Set custom view distance
    pub fn set_view_distance(&mut self, distance: Option<f32>) {
        self.view_distance_override = distance;
    }
    
    /// Get effective view distance
    pub fn get_view_distance(&self) -> f32 {
        self.view_distance_override.unwrap_or(MAX_ENTITY_VIEW_DISTANCE)
    }
}

/// Regional interest management system
pub struct InterestManager {
    /// Players and their interests
    players: HashMap<u32, PlayerInterest>,
    /// Entities organized by region
    entity_regions: HashMap<RegionCoord, HashSet<u32>>,
    /// Entity positions
    entity_positions: HashMap<u32, (Vec3, RegionCoord)>,
    /// Chunk subscribers
    chunk_subscribers: HashMap<VoxelPos, HashSet<u32>>,
    /// Last interest update time
    last_update: std::time::Instant,
    /// Update interval
    update_interval: std::time::Duration,
}

impl InterestManager {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            entity_regions: HashMap::new(),
            entity_positions: HashMap::new(),
            chunk_subscribers: HashMap::new(),
            last_update: std::time::Instant::now(),
            update_interval: std::time::Duration::from_secs_f32(1.0 / INTEREST_UPDATE_RATE),
        }
    }
    
    /// Add a player to the interest system
    pub fn add_player(&mut self, player_id: u32, position: Vec3) {
        let interest = PlayerInterest::new(player_id, position);
        self.players.insert(player_id, interest);
        self.update_player_interests(player_id);
    }
    
    /// Remove a player
    pub fn remove_player(&mut self, player_id: u32) {
        if let Some(interest) = self.players.remove(&player_id) {
            // Remove from chunk subscriptions
            for chunk in &interest.interested_chunks {
                if let Some(subscribers) = self.chunk_subscribers.get_mut(chunk) {
                    subscribers.remove(&player_id);
                }
            }
        }
    }
    
    /// Update player position
    pub fn update_player_position(&mut self, player_id: u32, position: Vec3) {
        if let Some(interest) = self.players.get_mut(&player_id) {
            let old_region = interest.current_region;
            interest.update_position(position);
            
            // If region changed, update interests immediately
            if old_region != interest.current_region {
                self.update_player_interests(player_id);
            }
        }
    }
    
    /// Add or update an entity position
    pub fn update_entity_position(&mut self, entity_id: u32, position: Vec3) {
        let new_region = RegionCoord::from_position(position);
        
        // Remove from old region
        if let Some((_, old_region)) = self.entity_positions.get(&entity_id) {
            if old_region != &new_region {
                if let Some(entities) = self.entity_regions.get_mut(old_region) {
                    entities.remove(&entity_id);
                }
            }
        }
        
        // Add to new region
        self.entity_regions
            .entry(new_region)
            .or_insert_with(HashSet::new)
            .insert(entity_id);
        
        self.entity_positions.insert(entity_id, (position, new_region));
    }
    
    /// Remove an entity
    pub fn remove_entity(&mut self, entity_id: u32) {
        if let Some((_, region)) = self.entity_positions.remove(&entity_id) {
            if let Some(entities) = self.entity_regions.get_mut(&region) {
                entities.remove(&entity_id);
            }
        }
        
        // Remove from all player interests
        for interest in self.players.values_mut() {
            interest.interested_entities.remove(&entity_id);
        }
    }
    
    /// Update interests for all players (call periodically)
    pub fn update_all_interests(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_update) < self.update_interval {
            return;
        }
        
        self.last_update = now;
        
        let player_ids: Vec<u32> = self.players.keys().copied().collect();
        for player_id in player_ids {
            self.update_player_interests(player_id);
        }
    }
    
    /// Update interests for a specific player
    fn update_player_interests(&mut self, player_id: u32) {
        let Some(interest) = self.players.get(&player_id) else { return };
        
        let view_distance = interest.get_view_distance();
        let view_distance_sq = view_distance * view_distance;
        let player_pos = interest.position;
        let player_region = interest.current_region;
        
        // Find entities in range
        let mut new_entities = HashSet::new();
        
        // Check current region and neighbors
        let mut regions_to_check = vec![player_region];
        regions_to_check.extend(player_region.get_neighbors());
        
        for region in regions_to_check {
            if let Some(entities) = self.entity_regions.get(&region) {
                for &entity_id in entities {
                    if entity_id == player_id {
                        continue; // Don't include self
                    }
                    
                    if let Some((entity_pos, _)) = self.entity_positions.get(&entity_id) {
                        let dist_sq = (*entity_pos - player_pos).length_squared();
                        if dist_sq <= view_distance_sq {
                            new_entities.insert(entity_id);
                        }
                    }
                }
            }
        }
        
        // Find chunks in range
        let chunk_radius = (view_distance / 32.0).ceil() as i32;
        let player_chunk = VoxelPos::from_world_pos(player_pos);
        let mut new_chunks = HashSet::new();
        
        for dx in -chunk_radius..=chunk_radius {
            for dy in -2..=2 { // Limit vertical chunk loading
                for dz in -chunk_radius..=chunk_radius {
                    let chunk_pos = VoxelPos {
                        x: player_chunk.x + dx,
                        y: player_chunk.y + dy,
                        z: player_chunk.z + dz,
                    };
                    
                    // Simple distance check
                    let chunk_center = Vec3::new(
                        (chunk_pos.x as f32 + 0.5) * 32.0,
                        (chunk_pos.y as f32 + 0.5) * 32.0,
                        (chunk_pos.z as f32 + 0.5) * 32.0,
                    );
                    
                    let dist = (chunk_center - player_pos).length();
                    if dist <= view_distance + 32.0 { // Add chunk size for margin
                        new_chunks.insert(chunk_pos);
                    }
                }
            }
        }
        
        // Update the player's interests
        if let Some(interest) = self.players.get_mut(&player_id) {
            // Update chunk subscriptions
            let old_chunks = std::mem::replace(&mut interest.interested_chunks, new_chunks.clone());
            
            // Remove from old chunks
            for chunk in old_chunks.difference(&new_chunks) {
                if let Some(subscribers) = self.chunk_subscribers.get_mut(chunk) {
                    subscribers.remove(&player_id);
                }
            }
            
            // Add to new chunks
            for chunk in new_chunks.difference(&old_chunks) {
                self.chunk_subscribers
                    .entry(*chunk)
                    .or_insert_with(HashSet::new)
                    .insert(player_id);
            }
            
            interest.interested_entities = new_entities;
        }
    }
    
    /// Get entities a player is interested in
    pub fn get_interested_entities(&self, player_id: u32) -> Option<&HashSet<u32>> {
        self.players.get(&player_id).map(|i| &i.interested_entities)
    }
    
    /// Get chunks a player is interested in
    pub fn get_interested_chunks(&self, player_id: u32) -> Option<&HashSet<VoxelPos>> {
        self.players.get(&player_id).map(|i| &i.interested_chunks)
    }
    
    /// Get players interested in a chunk
    pub fn get_chunk_subscribers(&self, chunk: &VoxelPos) -> Option<&HashSet<u32>> {
        self.chunk_subscribers.get(chunk)
    }
    
    /// Check if player is interested in an entity
    pub fn is_interested_in_entity(&self, player_id: u32, entity_id: u32) -> bool {
        self.players
            .get(&player_id)
            .map(|i| i.interested_entities.contains(&entity_id))
            .unwrap_or(false)
    }
    
    /// Get interest changes for a player
    pub fn get_interest_changes(&self, player_id: u32, old_entities: &HashSet<u32>) 
        -> (Vec<u32>, Vec<u32>) {
        let Some(interest) = self.players.get(&player_id) else {
            return (vec![], old_entities.iter().copied().collect());
        };
        
        let current = &interest.interested_entities;
        
        // New entities (in current but not in old)
        let added: Vec<u32> = current.difference(old_entities).copied().collect();
        
        // Removed entities (in old but not in current)
        let removed: Vec<u32> = old_entities.difference(current).copied().collect();
        
        (added, removed)
    }
    
    /// Get all players in a region and neighboring regions
    pub fn get_players_near_region(&self, region: RegionCoord) -> Vec<u32> {
        let mut players = Vec::new();
        let mut regions_to_check = vec![region];
        regions_to_check.extend(region.get_neighbors());
        
        for player_id in self.players.keys() {
            if let Some(interest) = self.players.get(player_id) {
                if regions_to_check.contains(&interest.current_region) {
                    players.push(*player_id);
                }
            }
        }
        
        players
    }
    
    /// Debug: Get statistics
    pub fn get_stats(&self) -> InterestStats {
        let total_subscriptions: usize = self.players
            .values()
            .map(|i| i.interested_entities.len() + i.interested_chunks.len())
            .sum();
        
        InterestStats {
            player_count: self.players.len(),
            entity_count: self.entity_positions.len(),
            region_count: self.entity_regions.len(),
            total_subscriptions,
            avg_entities_per_player: if self.players.is_empty() { 0.0 } else {
                self.players.values().map(|i| i.interested_entities.len()).sum::<usize>() as f32 
                    / self.players.len() as f32
            },
            avg_chunks_per_player: if self.players.is_empty() { 0.0 } else {
                self.players.values().map(|i| i.interested_chunks.len()).sum::<usize>() as f32 
                    / self.players.len() as f32
            },
        }
    }
}

/// Statistics for interest management
#[derive(Debug)]
pub struct InterestStats {
    pub player_count: usize,
    pub entity_count: usize,
    pub region_count: usize,
    pub total_subscriptions: usize,
    pub avg_entities_per_player: f32,
    pub avg_chunks_per_player: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_region_coord() {
        let pos = Vec3::new(100.0, 50.0, -30.0);
        let region = RegionCoord::from_position(pos);
        assert_eq!(region.x, 1);
        assert_eq!(region.y, 0);
        assert_eq!(region.z, -1);
    }
    
    #[test]
    fn test_interest_management() {
        let mut manager = InterestManager::new();
        
        // Add players
        manager.add_player(1, Vec3::new(0.0, 0.0, 0.0));
        manager.add_player(2, Vec3::new(50.0, 0.0, 0.0));
        manager.add_player(3, Vec3::new(200.0, 0.0, 0.0)); // Far away
        
        // Add entities
        manager.update_entity_position(10, Vec3::new(10.0, 0.0, 0.0));
        manager.update_entity_position(11, Vec3::new(60.0, 0.0, 0.0));
        manager.update_entity_position(12, Vec3::new(250.0, 0.0, 0.0));
        
        // Force update by setting last_update to a past time
        manager.last_update = std::time::Instant::now() - std::time::Duration::from_secs(10);
        manager.update_all_interests();
        
        // Check interests
        let p1_entities = manager.get_interested_entities(1).unwrap();
        assert!(p1_entities.contains(&10)); // Nearby
        assert!(p1_entities.contains(&11)); // Within range
        assert!(!p1_entities.contains(&12)); // Too far
        
        let p3_entities = manager.get_interested_entities(3).unwrap();
        assert!(!p3_entities.contains(&10)); // Too far
        assert!(!p3_entities.contains(&11)); // Too far
        assert!(p3_entities.contains(&12)); // Nearby
    }
}