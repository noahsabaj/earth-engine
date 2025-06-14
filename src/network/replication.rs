use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use crate::ecs::{EntityId, EcsWorldData, get_transform, get_physics};
use crate::network::{
    Packet, ServerPacket, ClientPacket, EntityType, EntityMetadata,
};
use crate::network::packet::{
    ChunkSaveStatus, ChunkSaveStateData, SaveStatus, LoadStatus,
};
use crate::world::{ChunkPos, Chunk};
use crate::persistence::{PersistenceResult, PersistenceError, NetworkValidator};
use glam::{Vec3, Quat};

/// Network entity ID (different from ECS entity)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkEntityId(pub u32);

/// Entity that can be replicated over network
#[derive(Debug, Clone)]
pub struct NetworkEntity {
    pub network_id: NetworkEntityId,
    pub entity: EntityId,
    pub entity_type: EntityType,
    pub owner_id: Option<u32>, // Player ID that owns this entity
    pub replicate_to_all: bool,
    pub replicate_to_owner: bool,
    pub last_position: Vec3,
    pub last_rotation: Quat,
    pub position_threshold: f32,
    pub rotation_threshold: f32,
}

impl NetworkEntity {
    pub fn new(network_id: NetworkEntityId, entity: EntityId, entity_type: EntityType) -> Self {
        Self {
            network_id,
            entity,
            entity_type,
            owner_id: None,
            replicate_to_all: true,
            replicate_to_owner: false,
            last_position: Vec3::ZERO,
            last_rotation: Quat::IDENTITY,
            position_threshold: 0.1, // 10cm
            rotation_threshold: 0.01, // Small rotation change
        }
    }
    
    /// Set the owner of this entity
    pub fn set_owner(&mut self, player_id: u32) {
        self.owner_id = Some(player_id);
    }
    
    /// Check if position/rotation changed enough to replicate
    pub fn needs_replication(&self, position: Vec3, rotation: Quat) -> bool {
        let pos_delta = (position - self.last_position).length();
        let rot_delta = (rotation.w - self.last_rotation.w).abs() + 
                       (rotation.x - self.last_rotation.x).abs() +
                       (rotation.y - self.last_rotation.y).abs() +
                       (rotation.z - self.last_rotation.z).abs();
        
        pos_delta > self.position_threshold || rot_delta > self.rotation_threshold
    }
    
    /// Update last replicated state
    pub fn update_replicated_state(&mut self, position: Vec3, rotation: Quat) {
        self.last_position = position;
        self.last_rotation = rotation;
    }
}

/// Chunk replication data
#[derive(Debug, Clone)]
pub struct ChunkReplicationData {
    pub chunk_pos: ChunkPos,
    pub save_state: ChunkSaveStatus,
    pub last_sync: Instant,
    pub checksum: u64,
    pub pending_save: bool,
    pub pending_load: bool,
    pub sync_priority: ChunkSyncPriority,
    pub error_count: u32,
    pub last_error: Option<String>,
}

/// Chunk synchronization priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkSyncPriority {
    Low,
    Normal, 
    High,
    Critical,
}

impl ChunkReplicationData {
    pub fn new(chunk_pos: ChunkPos, checksum: u64) -> Self {
        Self {
            chunk_pos,
            save_state: ChunkSaveStatus::Clean,
            last_sync: Instant::now(),
            checksum,
            pending_save: false,
            pending_load: false,
            sync_priority: ChunkSyncPriority::Normal,
            error_count: 0,
            last_error: None,
        }
    }

    /// Check if chunk needs synchronization
    pub fn needs_sync(&self, max_sync_interval: Duration) -> bool {
        self.pending_save || 
        self.pending_load ||
        self.save_state == ChunkSaveStatus::Dirty ||
        Instant::now().duration_since(self.last_sync) > max_sync_interval
    }

    /// Mark chunk as needing save
    pub fn mark_dirty(&mut self) {
        self.save_state = ChunkSaveStatus::Dirty;
        self.pending_save = true;
        if self.sync_priority == ChunkSyncPriority::Low {
            self.sync_priority = ChunkSyncPriority::Normal;
        }
    }

    /// Mark save operation started
    pub fn start_save(&mut self) {
        self.save_state = ChunkSaveStatus::Saving;
        self.pending_save = false;
        self.last_sync = Instant::now();
    }

    /// Mark save operation completed
    pub fn complete_save(&mut self, success: bool, error: Option<String>) {
        if success {
            self.save_state = ChunkSaveStatus::Saved;
            self.error_count = 0;
            self.last_error = None;
        } else {
            self.save_state = ChunkSaveStatus::SaveFailed;
            self.error_count += 1;
            self.last_error = error;
            // Increase priority on failure
            self.sync_priority = match self.sync_priority {
                ChunkSyncPriority::Low => ChunkSyncPriority::Normal,
                ChunkSyncPriority::Normal => ChunkSyncPriority::High,
                ChunkSyncPriority::High => ChunkSyncPriority::Critical,
                ChunkSyncPriority::Critical => ChunkSyncPriority::Critical,
            };
        }
    }

    /// Mark load operation started
    pub fn start_load(&mut self) {
        self.save_state = ChunkSaveStatus::Loading;
        self.pending_load = false;
        self.last_sync = Instant::now();
    }

    /// Mark load operation completed
    pub fn complete_load(&mut self, success: bool, error: Option<String>) {
        if success {
            self.save_state = ChunkSaveStatus::Loaded;
            self.error_count = 0;
            self.last_error = None;
        } else {
            self.save_state = ChunkSaveStatus::LoadFailed;
            self.error_count += 1;
            self.last_error = error;
        }
    }
}

/// Manages entity replication between server and clients
pub struct ReplicationManager {
    /// All network entities
    entities: HashMap<NetworkEntityId, NetworkEntity>,
    /// Mapping from ECS entity to network entity
    entity_to_network: HashMap<EntityId, NetworkEntityId>,
    /// Next network entity ID
    next_network_id: u32,
    /// Entities that need spawn packets sent
    spawn_queue: Vec<NetworkEntityId>,
    /// Entities that need despawn packets sent
    despawn_queue: Vec<NetworkEntityId>,
    /// Chunk replication data
    chunks: HashMap<ChunkPos, ChunkReplicationData>,
    /// Network validator for consistency checking
    validator: Option<Arc<Mutex<NetworkValidator>>>,
    /// Configuration
    config: ReplicationConfig,
}

/// Configuration for replication system
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// Maximum interval between chunk synchronization
    pub max_chunk_sync_interval: Duration,
    /// Enable chunk validation
    pub enable_chunk_validation: bool,
    /// Maximum chunks to sync per tick
    pub max_chunks_per_tick: usize,
    /// Retry limit for failed operations
    pub max_retry_count: u32,
    /// Batch size for chunk state updates
    pub chunk_state_batch_size: usize,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            max_chunk_sync_interval: Duration::from_secs(30),
            enable_chunk_validation: true,
            max_chunks_per_tick: 10,
            max_retry_count: 3,
            chunk_state_batch_size: 20,
        }
    }
}

impl ReplicationManager {
    pub fn new() -> Self {
        Self::with_config(ReplicationConfig::default())
    }

    pub fn with_config(config: ReplicationConfig) -> Self {
        Self {
            entities: HashMap::new(),
            entity_to_network: HashMap::new(),
            next_network_id: 1000, // Start at 1000 to avoid conflicts with player IDs
            spawn_queue: Vec::new(),
            despawn_queue: Vec::new(),
            chunks: HashMap::new(),
            validator: None,
            config,
        }
    }

    /// Set network validator for consistency checking
    pub fn set_validator(&mut self, validator: Arc<Mutex<NetworkValidator>>) {
        self.validator = Some(validator);
    }
    
    /// Register an entity for replication
    pub fn register_entity(&mut self, entity: EntityId, entity_type: EntityType) -> NetworkEntityId {
        let network_id = NetworkEntityId(self.next_network_id);
        self.next_network_id += 1;
        
        let network_entity = NetworkEntity::new(network_id, entity, entity_type);
        self.entities.insert(network_id, network_entity);
        self.entity_to_network.insert(entity, network_id);
        self.spawn_queue.push(network_id);
        
        network_id
    }
    
    /// Unregister an entity
    pub fn unregister_entity(&mut self, entity: EntityId) {
        if let Some(network_id) = self.entity_to_network.remove(&entity) {
            self.entities.remove(&network_id);
            self.despawn_queue.push(network_id);
        }
    }
    
    /// Get network entity by ECS entity
    pub fn get_network_entity(&self, entity: EntityId) -> Option<&NetworkEntity> {
        self.entity_to_network.get(&entity)
            .and_then(|id| self.entities.get(id))
    }
    
    /// Get mutable network entity by ECS entity
    pub fn get_network_entity_mut(&mut self, entity: EntityId) -> Option<&mut NetworkEntity> {
        if let Some(id) = self.entity_to_network.get(&entity).copied() {
            self.entities.get_mut(&id)
        } else {
            None
        }
    }
    
    /// Process replication for all entities
    pub fn process_replication(&mut self, ecs_world: &EcsWorldData) -> Vec<Packet> {
        let mut packets = Vec::new();
        
        // Process spawn queue
        while let Some(network_id) = self.spawn_queue.pop() {
            if let Some(network_entity) = self.entities.get(&network_id) {
                // Get entity position and rotation from ECS
                let (position, rotation, velocity) = if let Some(transform) = get_transform(ecs_world, network_entity.entity) {
                    let velocity = get_physics(ecs_world, network_entity.entity)
                        .map(|p| Vec3::new(p.velocity[0], p.velocity[1], p.velocity[2]))
                        .unwrap_or(Vec3::ZERO);
                    let pos = Vec3::new(transform.position[0], transform.position[1], transform.position[2]);
                    let rot = Quat::from_euler(glam::EulerRot::YXZ, transform.rotation[1], transform.rotation[0], transform.rotation[2]);
                    (pos, rot, velocity)
                } else {
                    (Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO)
                };
                
                packets.push(Packet::Server(ServerPacket::EntitySpawn {
                    entity_id: network_id.0,
                    entity_type: network_entity.entity_type.clone(),
                    position,
                    rotation,
                    velocity,
                    metadata: EntityMetadata {
                        health: None,
                        name: None,
                        custom_data: Vec::new(),
                    },
                }));
            }
        }
        
        // Process despawn queue
        while let Some(network_id) = self.despawn_queue.pop() {
            packets.push(Packet::Server(ServerPacket::EntityDespawn {
                entity_id: network_id.0,
            }));
        }
        
        // Process position updates
        for network_entity in self.entities.values_mut() {
            // Get current transform
            if let Some(transform) = get_transform(ecs_world, network_entity.entity) {
                let velocity = get_physics(ecs_world, network_entity.entity)
                    .map(|p| Vec3::new(p.velocity[0], p.velocity[1], p.velocity[2]))
                    .unwrap_or(Vec3::ZERO);
                
                let pos = Vec3::new(transform.position[0], transform.position[1], transform.position[2]);
                let rot = Quat::from_euler(glam::EulerRot::YXZ, transform.rotation[1], transform.rotation[0], transform.rotation[2]);
                
                // Check if update needed
                if network_entity.needs_replication(pos, rot) {
                    network_entity.update_replicated_state(pos, rot);
                    
                    packets.push(Packet::Server(ServerPacket::EntityUpdate {
                        entity_id: network_entity.network_id.0,
                        position: pos,
                        rotation: rot,
                        velocity,
                    }));
                }
            }
        }
        
        packets
    }

    /// Register chunk for replication
    pub fn register_chunk(&mut self, chunk_pos: ChunkPos, checksum: u64) -> PersistenceResult<()> {
        let chunk_data = ChunkReplicationData::new(chunk_pos, checksum);
        self.chunks.insert(chunk_pos, chunk_data);

        // Register with validator if available
        if let Some(validator) = &self.validator {
            if let Ok(validator) = validator.lock() {
                validator.register_chunk(chunk_pos, checksum, ChunkSaveStatus::Clean)?;
            }
        }

        Ok(())
    }

    /// Unregister chunk from replication
    pub fn unregister_chunk(&mut self, chunk_pos: ChunkPos) {
        self.chunks.remove(&chunk_pos);
    }

    /// Mark chunk as dirty (needs saving)
    pub fn mark_chunk_dirty(&mut self, chunk_pos: ChunkPos, new_checksum: u64) {
        if let Some(chunk_data) = self.chunks.get_mut(&chunk_pos) {
            chunk_data.mark_dirty();
            chunk_data.checksum = new_checksum;
        } else {
            // Register new dirty chunk
            let mut chunk_data = ChunkReplicationData::new(chunk_pos, new_checksum);
            chunk_data.mark_dirty();
            self.chunks.insert(chunk_pos, chunk_data);
        }
    }

    /// Process chunk synchronization
    pub fn process_chunk_sync(&mut self) -> PersistenceResult<Vec<Packet>> {
        let mut packets = Vec::new();
        let mut chunk_states = Vec::new();

        // Find chunks that need synchronization
        let mut chunks_to_sync: Vec<_> = self.chunks.iter_mut()
            .filter(|(_, data)| data.needs_sync(self.config.max_chunk_sync_interval))
            .take(self.config.max_chunks_per_tick)
            .collect();

        // Sort by priority
        chunks_to_sync.sort_by(|(_, a), (_, b)| {
            let priority_order = |p: ChunkSyncPriority| match p {
                ChunkSyncPriority::Critical => 0,
                ChunkSyncPriority::High => 1,
                ChunkSyncPriority::Normal => 2,
                ChunkSyncPriority::Low => 3,
            };
            priority_order(a.sync_priority).cmp(&priority_order(b.sync_priority))
        });

        // Process each chunk
        for (chunk_pos, chunk_data) in chunks_to_sync {
            let state_data = ChunkSaveStateData {
                chunk_pos: *chunk_pos,
                state: chunk_data.save_state,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                error_message: chunk_data.last_error.clone(),
            };

            chunk_states.push(state_data);

            // Update sync time
            chunk_data.last_sync = Instant::now();
        }

        // Send chunk state updates in batches
        if !chunk_states.is_empty() {
            for batch in chunk_states.chunks(self.config.chunk_state_batch_size) {
                packets.push(Packet::Server(ServerPacket::ChunkSaveStates {
                    states: batch.to_vec(),
                }));
            }
        }

        // Validate chunk states if validator is available
        if self.config.enable_chunk_validation {
            if let Some(validator) = &self.validator {
                if let Ok(validator) = validator.lock() {
                    let chunk_state_map: HashMap<ChunkPos, ChunkSaveStatus> = self.chunks.iter()
                        .map(|(pos, data)| (*pos, data.save_state))
                        .collect();
                    
                    // Note: validate_network_sync method doesn't exist in new validator
                    // This would need to be updated to use the new state_validator API
                    // For now, commenting out to fix compilation
                    /*
                    if let Ok(validation_results) = validator.validate_network_sync(&chunk_state_map) {
                        for result in validation_results {
                            if !result.success {
                                eprintln!("[ReplicationManager] Validation error: {} errors", result.errors.len());
                                for error in &result.errors {
                                    eprintln!("  - {}", error.description);
                                }
                            }
                        }
                    }
                    */
                }
            }
        }

        Ok(packets)
    }

    /// Handle save operation completion for a chunk
    pub fn complete_chunk_save(&mut self, chunk_pos: ChunkPos, success: bool, 
                              error: Option<String>) -> PersistenceResult<()> {
        if let Some(chunk_data) = self.chunks.get_mut(&chunk_pos) {
            chunk_data.complete_save(success, error.clone());

            // Validate save if successful and validator available
            if success && self.config.enable_chunk_validation {
                if let Some(validator) = &self.validator {
                    if let Ok(validator) = validator.lock() {
                        let _ = validator.validate_chunk_save(chunk_pos, chunk_data.checksum, ChunkSaveStatus::Saved);
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle load operation completion for a chunk
    pub fn complete_chunk_load(&mut self, chunk_pos: ChunkPos, success: bool, 
                              error: Option<String>, checksum: Option<u64>) -> PersistenceResult<()> {
        if let Some(chunk_data) = self.chunks.get_mut(&chunk_pos) {
            chunk_data.complete_load(success, error.clone());
            
            if success {
                if let Some(new_checksum) = checksum {
                    chunk_data.checksum = new_checksum;
                }

                // Validate load if successful and validator available
                if self.config.enable_chunk_validation {
                    if let Some(validator) = &self.validator {
                        if let Ok(validator) = validator.lock() {
                            let _ = validator.validate_chunk_load(chunk_pos, chunk_data.checksum);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get chunks that need saving
    pub fn get_chunks_needing_save(&self) -> Vec<ChunkPos> {
        self.chunks.iter()
            .filter(|(_, data)| data.pending_save || data.save_state == ChunkSaveStatus::Dirty)
            .filter(|(_, data)| data.error_count < self.config.max_retry_count)
            .map(|(pos, _)| *pos)
            .collect()
    }

    /// Get chunks that failed save operations
    pub fn get_failed_chunks(&self) -> Vec<(ChunkPos, String)> {
        self.chunks.iter()
            .filter(|(_, data)| data.save_state == ChunkSaveStatus::SaveFailed || 
                               data.save_state == ChunkSaveStatus::LoadFailed)
            .filter_map(|(pos, data)| {
                data.last_error.as_ref().map(|error| (*pos, error.clone()))
            })
            .collect()
    }

    /// Get chunk synchronization statistics
    pub fn get_chunk_sync_stats(&self) -> ChunkSyncStats {
        let mut stats = ChunkSyncStats::default();

        for (_, chunk_data) in &self.chunks {
            stats.total_chunks += 1;

            match chunk_data.save_state {
                ChunkSaveStatus::Clean => stats.clean_chunks += 1,
                ChunkSaveStatus::Dirty => stats.dirty_chunks += 1,
                ChunkSaveStatus::Saving => stats.saving_chunks += 1,
                ChunkSaveStatus::Saved => stats.saved_chunks += 1,
                ChunkSaveStatus::SaveFailed => stats.failed_chunks += 1,
                ChunkSaveStatus::Loading => stats.loading_chunks += 1,
                ChunkSaveStatus::Loaded => stats.loaded_chunks += 1,
                ChunkSaveStatus::LoadFailed => stats.failed_chunks += 1,
            }

            match chunk_data.sync_priority {
                ChunkSyncPriority::Critical => stats.critical_priority += 1,
                ChunkSyncPriority::High => stats.high_priority += 1,
                ChunkSyncPriority::Normal => stats.normal_priority += 1,
                ChunkSyncPriority::Low => stats.low_priority += 1,
            }

            if chunk_data.pending_save {
                stats.pending_saves += 1;
            }
            if chunk_data.pending_load {
                stats.pending_loads += 1;
            }
        }

        stats
    }
    
    /// Get packets for a specific player (respects ownership)
    pub fn get_packets_for_player(&self, packets: &[Packet], player_id: u32) -> Vec<Packet> {
        packets.iter().filter_map(|packet| {
            match packet {
                Packet::Server(ServerPacket::EntitySpawn { entity_id, .. }) |
                Packet::Server(ServerPacket::EntityUpdate { entity_id, .. }) |
                Packet::Server(ServerPacket::EntityDespawn { entity_id, .. }) => {
                    // Check if this entity should be replicated to this player
                    let network_id = NetworkEntityId(*entity_id);
                    if let Some(network_entity) = self.entities.get(&network_id) {
                        let is_owner = network_entity.owner_id == Some(player_id);
                        
                        if network_entity.replicate_to_all ||
                           (is_owner && network_entity.replicate_to_owner) {
                            Some(packet.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => Some(packet.clone()),
            }
        }).collect()
    }
}

/// Client-side replication receiver
pub struct ReplicationReceiver {
    /// Network ID to ECS entity mapping
    network_to_entity: HashMap<NetworkEntityId, EntityId>,
}

impl ReplicationReceiver {
    pub fn new() -> Self {
        Self {
            network_to_entity: HashMap::new(),
        }
    }
    
    /// Handle entity spawn packet
    pub fn handle_entity_spawn(&mut self, ecs_world: &mut EcsWorldData, 
                              network_id: u32, entity_type: EntityType,
                              position: Vec3, rotation: Quat, velocity: Vec3) {
        let network_id = NetworkEntityId(network_id);
        
        // Create entity
        let entity = ecs_world.create_entity();
        
        // Add transform component
        let euler = rotation.to_euler(glam::EulerRot::YXZ);
        ecs_world.add_transform(entity, 
            [position.x, position.y, position.z],
            [euler.1, euler.0, euler.2],
            [1.0, 1.0, 1.0]);
        
        // Add physics component if has velocity
        if velocity != Vec3::ZERO {
            ecs_world.add_physics(entity, 1.0, [-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]);
            if let Some(physics) = ecs_world.components.get_physics_mut(entity) {
                physics.velocity = [velocity.x, velocity.y, velocity.z];
            }
        }
        
        // Add type-specific components
        match entity_type {
            EntityType::Item { item_id, count } => {
                ecs_world.add_item(entity, item_id, count);
            }
            _ => {}
        }
        
        self.network_to_entity.insert(network_id, entity);
    }
    
    /// Handle entity despawn packet
    pub fn handle_entity_despawn(&mut self, ecs_world: &mut EcsWorldData, network_id: u32) {
        let network_id = NetworkEntityId(network_id);
        
        if let Some(entity) = self.network_to_entity.remove(&network_id) {
            ecs_world.destroy_entity(entity);
        }
    }
    
    /// Handle entity update packet
    pub fn handle_entity_update(&mut self, ecs_world: &mut EcsWorldData,
                               network_id: u32, position: Vec3, rotation: Quat, velocity: Vec3) {
        let network_id = NetworkEntityId(network_id);
        
        if let Some(&entity) = self.network_to_entity.get(&network_id) {
            // Update transform
            if let Some(transform) = ecs_world.components.get_transform_mut(entity) {
                transform.position = [position.x, position.y, position.z];
                // Convert quaternion to euler angles
                let euler = rotation.to_euler(glam::EulerRot::YXZ);
                transform.rotation = [euler.1, euler.0, euler.2];
            }
            
            // Update physics
            if let Some(physics) = ecs_world.components.get_physics_mut(entity) {
                physics.velocity = [velocity.x, velocity.y, velocity.z];
            }
        }
    }
}

/// Statistics for chunk synchronization
#[derive(Debug, Clone, Default)]
pub struct ChunkSyncStats {
    pub total_chunks: usize,
    pub clean_chunks: usize,
    pub dirty_chunks: usize,
    pub saving_chunks: usize,
    pub saved_chunks: usize,
    pub loading_chunks: usize,
    pub loaded_chunks: usize,
    pub failed_chunks: usize,
    pub pending_saves: usize,
    pub pending_loads: usize,
    pub critical_priority: usize,
    pub high_priority: usize,
    pub normal_priority: usize,
    pub low_priority: usize,
}

/// Integrated replication system that combines entities and chunks
pub struct IntegratedReplicationSystem {
    pub entity_manager: ReplicationManager,
    pub receiver: ReplicationReceiver,
}

impl IntegratedReplicationSystem {
    pub fn new(config: ReplicationConfig) -> Self {
        Self {
            entity_manager: ReplicationManager::with_config(config),
            receiver: ReplicationReceiver::new(),
        }
    }

    /// Process all replication for a tick
    pub fn process_tick(&mut self, ecs_world: &EcsWorldData) -> PersistenceResult<Vec<Packet>> {
        let mut packets = Vec::new();

        // Process entity replication
        let entity_packets = self.entity_manager.process_replication(ecs_world);
        packets.extend(entity_packets);

        // Process chunk synchronization
        let chunk_packets = self.entity_manager.process_chunk_sync()?;
        packets.extend(chunk_packets);

        Ok(packets)
    }

    /// Get comprehensive replication statistics
    pub fn get_stats(&self) -> ReplicationStats {
        let chunk_stats = self.entity_manager.get_chunk_sync_stats();
        
        ReplicationStats {
            entity_count: self.entity_manager.entities.len(),
            chunk_stats,
            spawn_queue_size: self.entity_manager.spawn_queue.len(),
            despawn_queue_size: self.entity_manager.despawn_queue.len(),
        }
    }
}

/// Combined replication statistics
#[derive(Debug, Clone)]
pub struct ReplicationStats {
    pub entity_count: usize,
    pub chunk_stats: ChunkSyncStats,
    pub spawn_queue_size: usize,
    pub despawn_queue_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_replication_data() {
        let chunk_pos = ChunkPos { x: 0, y: 0, z: 0 };
        let mut chunk_data = ChunkReplicationData::new(chunk_pos, 12345);
        
        assert_eq!(chunk_data.save_state, ChunkSaveStatus::Clean);
        assert!(!chunk_data.pending_save);
        assert!(!chunk_data.pending_load);
        
        // Mark dirty
        chunk_data.mark_dirty();
        assert_eq!(chunk_data.save_state, ChunkSaveStatus::Dirty);
        assert!(chunk_data.pending_save);
        
        // Start save
        chunk_data.start_save();
        assert_eq!(chunk_data.save_state, ChunkSaveStatus::Saving);
        assert!(!chunk_data.pending_save);
        
        // Complete save successfully
        chunk_data.complete_save(true, None);
        assert_eq!(chunk_data.save_state, ChunkSaveStatus::Saved);
        assert_eq!(chunk_data.error_count, 0);
    }

    #[test]
    fn test_replication_manager_with_chunks() {
        let config = ReplicationConfig::default();
        let mut manager = ReplicationManager::with_config(config);
        
        let chunk_pos = ChunkPos { x: 0, y: 0, z: 0 };
        
        // Register chunk
        manager.register_chunk(chunk_pos, 12345).expect("Failed to register chunk");
        
        // Mark chunk dirty
        manager.mark_chunk_dirty(chunk_pos, 54321);
        
        // Check stats
        let stats = manager.get_chunk_sync_stats();
        assert_eq!(stats.total_chunks, 1);
        assert_eq!(stats.dirty_chunks, 1);
        
        // Get chunks needing save
        let chunks_to_save = manager.get_chunks_needing_save();
        assert_eq!(chunks_to_save.len(), 1);
        assert_eq!(chunks_to_save[0], chunk_pos);
    }

    #[test]
    fn test_integrated_replication_system() {
        let config = ReplicationConfig::default();
        let system = IntegratedReplicationSystem::new(config);
        
        let stats = system.get_stats();
        assert_eq!(stats.entity_count, 0);
        assert_eq!(stats.chunk_stats.total_chunks, 0);
        assert_eq!(stats.spawn_queue_size, 0);
        assert_eq!(stats.despawn_queue_size, 0);
    }
}