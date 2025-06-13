use std::collections::HashMap;
use std::time::Instant;
use glam::Vec3;
use crate::world::{World, VoxelPos};
use crate::network::{
    InterestManager, DeltaEncoder, AntiCheat, LagCompensation,
    EntityState, InterpolationManager, PositionSnapshot,
    PlayerStateSnapshot, MovementState, ServerPacket,
    PacketOptimizer,
};

/// Network synchronization manager
pub struct NetworkSync {
    /// Interest management
    interest_manager: InterestManager,
    /// Delta compression for entities
    delta_encoder: DeltaEncoder,
    /// Anti-cheat system
    anti_cheat: AntiCheat,
    /// Lag compensation
    lag_compensation: LagCompensation,
    /// Client interpolation managers
    client_interpolators: HashMap<u32, InterpolationManager>,
    /// Packet optimizer
    packet_optimizer: PacketOptimizer,
    /// Sync state for each player
    player_sync_states: HashMap<u32, PlayerSyncState>,
    /// Server tick rate
    tick_rate: u32,
    /// Last sync time
    last_sync: Instant,
}

/// Per-player synchronization state
struct PlayerSyncState {
    /// Last acknowledged packet
    last_ack_sequence: u32,
    /// Pending reliable packets
    pending_reliable: Vec<(u32, ServerPacket)>,
    /// Last entity snapshot sent
    last_entity_snapshot: HashMap<u32, EntityState>,
    /// Average ping
    average_ping: f32,
    /// Ping jitter
    ping_jitter: f32,
    /// Connection quality (0-1)
    connection_quality: f32,
}

impl NetworkSync {
    pub fn new(tick_rate: u32) -> Self {
        Self {
            interest_manager: InterestManager::new(),
            delta_encoder: DeltaEncoder::new(),
            anti_cheat: AntiCheat::new(),
            lag_compensation: LagCompensation::new(),
            client_interpolators: HashMap::new(),
            packet_optimizer: PacketOptimizer::new(1400), // Typical MTU
            player_sync_states: HashMap::new(),
            tick_rate,
            last_sync: Instant::now(),
        }
    }
    
    /// Add a new player to sync
    pub fn add_player(&mut self, player_id: u32, position: Vec3) {
        self.interest_manager.add_player(player_id, position);
        self.player_sync_states.insert(player_id, PlayerSyncState {
            last_ack_sequence: 0,
            pending_reliable: Vec::new(),
            last_entity_snapshot: HashMap::new(),
            average_ping: 50.0,
            ping_jitter: 5.0,
            connection_quality: 1.0,
        });
        self.client_interpolators.insert(player_id, InterpolationManager::new());
    }
    
    /// Remove a player
    pub fn remove_player(&mut self, player_id: u32) {
        self.interest_manager.remove_player(player_id);
        self.player_sync_states.remove(&player_id);
        self.client_interpolators.remove(&player_id);
        self.anti_cheat.reset_player(player_id);
    }
    
    /// Process incoming player update
    pub fn process_player_update(
        &mut self,
        player_id: u32,
        old_pos: Vec3,
        new_pos: Vec3,
        rotation: glam::Quat,
        velocity: Vec3,
        movement_state: MovementState,
        on_ground: bool,
        delta_time: f32,
        sequence: u32,
    ) -> Result<Vec3, &'static str> {
        // Validate movement
        let validation = self.anti_cheat.validate_movement(
            player_id,
            old_pos,
            new_pos,
            delta_time,
            movement_state,
            on_ground,
        );
        
        if !validation.is_valid {
            // Check if we should kick/ban
            if self.anti_cheat.should_ban(player_id) {
                return Err("Player banned for cheating");
            } else if self.anti_cheat.should_kick(player_id) {
                return Err("Player kicked for violations");
            }
            
            // Return corrected position if available
            if let Some(corrected) = validation.corrected_position {
                return Ok(corrected);
            }
            
            return Ok(old_pos); // Reject the move
        }
        
        // Update interest management
        self.interest_manager.update_player_position(player_id, new_pos);
        
        // Add to lag compensation history
        let snapshot = PlayerStateSnapshot {
            timestamp: Instant::now(),
            server_tick: sequence,
            player_id,
            position: new_pos,
            rotation,
            velocity,
            hitbox_min: Vec3::new(-0.3, 0.0, -0.3),
            hitbox_max: Vec3::new(0.3, 1.8, 0.3),
        };
        self.lag_compensation.add_player_snapshot(snapshot);
        
        Ok(new_pos)
    }
    
    /// Generate sync packets for all players
    pub fn generate_sync_packets(
        &mut self,
        world: &World,
        entities: &HashMap<u32, EntityState>,
    ) -> HashMap<u32, Vec<ServerPacket>> {
        let mut packets_by_player = HashMap::new();
        
        // Update all interests
        self.interest_manager.update_all_interests();
        
        for (player_id, sync_state) in &mut self.player_sync_states {
            let mut packets = Vec::new();
            
            // Get player's interested entities
            if let Some(interested) = self.interest_manager.get_interested_entities(*player_id) {
                let mut entity_updates = Vec::new();
                
                // Generate entity deltas
                for &entity_id in interested {
                    if let Some(entity_state) = entities.get(&entity_id) {
                        let delta = self.delta_encoder.encode_delta(entity_state);
                        if !delta.changes.is_empty() {
                            entity_updates.push(delta);
                        }
                    }
                }
                
                // Check for removed entities
                let old_entities: std::collections::HashSet<u32> = 
                    sync_state.last_entity_snapshot.keys().copied().collect();
                let (_, removed) = self.interest_manager.get_interest_changes(*player_id, &old_entities);
                
                for entity_id in removed {
                    packets.push(ServerPacket::EntityDespawn { entity_id });
                }
                
                // Split entity updates into packets
                let update_packets = self.packet_optimizer.split_entity_updates(entity_updates);
                for updates in update_packets {
                    // Send individual entity updates
                    for delta in updates {
                        if let (Some(pos), Some(rot), Some(vel)) = (
                            delta.changes.position,
                            delta.changes.rotation,
                            delta.changes.velocity,
                        ) {
                            packets.push(ServerPacket::EntityUpdate {
                                entity_id: delta.entity_id,
                                position: pos,
                                rotation: rot,
                                velocity: vel,
                            });
                        }
                    }
                }
                
                // Update last snapshot
                sync_state.last_entity_snapshot = interested.iter()
                    .filter_map(|&id| entities.get(&id).map(|e| (id, e.clone())))
                    .collect();
            }
            
            // Handle chunk updates
            if let Some(chunks) = self.interest_manager.get_interested_chunks(*player_id) {
                // TODO: Generate chunk update packets
            }
            
            // Auto-adjust interpolation delay based on connection quality
            if let Some(interpolator) = self.client_interpolators.get_mut(player_id) {
                interpolator.auto_adjust_delay(
                    sync_state.average_ping as u32,
                    sync_state.ping_jitter as u32,
                );
            }
            
            packets_by_player.insert(*player_id, packets);
        }
        
        packets_by_player
    }
    
    /// Process entity position for interpolation
    pub fn add_entity_position(
        &mut self,
        entity_id: u32,
        position: Vec3,
        rotation: glam::Quat,
        velocity: Vec3,
        server_tick: u32,
    ) {
        let snapshot = PositionSnapshot {
            timestamp: Instant::now(),
            server_tick,
            position,
            rotation,
            velocity,
        };
        
        // Update all client interpolators
        for interpolator in self.client_interpolators.values_mut() {
            interpolator.add_snapshot(entity_id, snapshot.clone());
        }
        
        // Update interest manager
        self.interest_manager.update_entity_position(entity_id, position);
    }
    
    /// Get interpolated position for rendering
    pub fn get_interpolated_position(
        &mut self,
        player_id: u32,
        entity_id: u32,
        render_time: Instant,
    ) -> Option<(Vec3, glam::Quat)> {
        self.client_interpolators.get_mut(&player_id)?
            .get_interpolated(entity_id, render_time)
    }
    
    /// Update player connection stats
    pub fn update_player_stats(
        &mut self,
        player_id: u32,
        ping_ms: u32,
        packet_loss: f32,
    ) {
        if let Some(state) = self.player_sync_states.get_mut(&player_id) {
            // Update average ping (exponential moving average)
            state.average_ping = state.average_ping * 0.9 + ping_ms as f32 * 0.1;
            
            // Calculate jitter
            let ping_diff = (ping_ms as f32 - state.average_ping).abs();
            state.ping_jitter = state.ping_jitter * 0.9 + ping_diff * 0.1;
            
            // Calculate connection quality
            let ping_quality = 1.0 - (state.average_ping / 300.0).min(1.0);
            let jitter_quality = 1.0 - (state.ping_jitter / 50.0).min(1.0);
            let loss_quality = 1.0 - packet_loss;
            
            state.connection_quality = (ping_quality + jitter_quality + loss_quality) / 3.0;
        }
    }
    
    /// Validate block interaction with lag compensation
    pub fn validate_block_interaction(
        &mut self,
        player_id: u32,
        player_pos: Vec3,
        block_pos: VoxelPos,
        interaction_type: crate::network::InteractionType,
        ping_ms: u32,
    ) -> bool {
        // Basic validation
        if !self.anti_cheat.validate_block_interaction(
            player_id,
            player_pos,
            block_pos,
            interaction_type,
        ) {
            return false;
        }
        
        // Lag compensated validation
        self.lag_compensation.validate_action(
            player_id,
            Instant::now(),
            ping_ms,
            |player_state, _| {
                // Re-validate with historical position
                let block_center = Vec3::new(
                    block_pos.x as f32 + 0.5,
                    block_pos.y as f32 + 0.5,
                    block_pos.z as f32 + 0.5,
                );
                let distance = (block_center - player_state.position).length();
                
                Some(distance <= 5.5) // Slightly more lenient for lag
            },
        ).unwrap_or(false)
    }
    
    /// Get sync statistics
    pub fn get_stats(&self) -> SyncStats {
        let interest_stats = self.interest_manager.get_stats();
        
        let avg_ping: f32 = if self.player_sync_states.is_empty() {
            0.0
        } else {
            self.player_sync_states.values()
                .map(|s| s.average_ping)
                .sum::<f32>() / self.player_sync_states.len() as f32
        };
        
        let avg_quality: f32 = if self.player_sync_states.is_empty() {
            1.0
        } else {
            self.player_sync_states.values()
                .map(|s| s.connection_quality)
                .sum::<f32>() / self.player_sync_states.len() as f32
        };
        
        SyncStats {
            player_count: self.player_sync_states.len(),
            entity_count: interest_stats.entity_count,
            avg_entities_per_player: interest_stats.avg_entities_per_player,
            avg_chunks_per_player: interest_stats.avg_chunks_per_player,
            avg_ping_ms: avg_ping,
            avg_connection_quality: avg_quality,
        }
    }
    
    /// Periodic maintenance
    pub fn tick(&mut self) {
        // Clean up old violations
        self.anti_cheat.cleanup_old_violations();
        
        // Update lag compensation time
        self.lag_compensation.update_time(Instant::now());
        self.lag_compensation.cleanup_old_history();
        
        self.last_sync = Instant::now();
    }
}

/// Synchronization statistics
#[derive(Debug)]
pub struct SyncStats {
    pub player_count: usize,
    pub entity_count: usize,
    pub avg_entities_per_player: f32,
    pub avg_chunks_per_player: f32,
    pub avg_ping_ms: f32,
    pub avg_connection_quality: f32,
}