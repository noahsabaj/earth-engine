use std::collections::HashMap;
use crate::ecs::{Entity, EcsWorld};
use crate::network::{Packet, ServerPacket, EntityType, EntityMetadata};
use glam::{Vec3, Quat};

/// Network entity ID (different from ECS entity)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkEntityId(pub u32);

/// Entity that can be replicated over network
#[derive(Debug, Clone)]
pub struct NetworkEntity {
    pub network_id: NetworkEntityId,
    pub entity: Entity,
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
    pub fn new(network_id: NetworkEntityId, entity: Entity, entity_type: EntityType) -> Self {
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

/// Manages entity replication between server and clients
pub struct ReplicationManager {
    /// All network entities
    entities: HashMap<NetworkEntityId, NetworkEntity>,
    /// Mapping from ECS entity to network entity
    entity_to_network: HashMap<Entity, NetworkEntityId>,
    /// Next network entity ID
    next_network_id: u32,
    /// Entities that need spawn packets sent
    spawn_queue: Vec<NetworkEntityId>,
    /// Entities that need despawn packets sent
    despawn_queue: Vec<NetworkEntityId>,
}

impl ReplicationManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            entity_to_network: HashMap::new(),
            next_network_id: 1000, // Start at 1000 to avoid conflicts with player IDs
            spawn_queue: Vec::new(),
            despawn_queue: Vec::new(),
        }
    }
    
    /// Register an entity for replication
    pub fn register_entity(&mut self, entity: Entity, entity_type: EntityType) -> NetworkEntityId {
        let network_id = NetworkEntityId(self.next_network_id);
        self.next_network_id += 1;
        
        let network_entity = NetworkEntity::new(network_id, entity, entity_type);
        self.entities.insert(network_id, network_entity);
        self.entity_to_network.insert(entity, network_id);
        self.spawn_queue.push(network_id);
        
        network_id
    }
    
    /// Unregister an entity
    pub fn unregister_entity(&mut self, entity: Entity) {
        if let Some(network_id) = self.entity_to_network.remove(&entity) {
            self.entities.remove(&network_id);
            self.despawn_queue.push(network_id);
        }
    }
    
    /// Get network entity by ECS entity
    pub fn get_network_entity(&self, entity: Entity) -> Option<&NetworkEntity> {
        self.entity_to_network.get(&entity)
            .and_then(|id| self.entities.get(id))
    }
    
    /// Get mutable network entity by ECS entity
    pub fn get_network_entity_mut(&mut self, entity: Entity) -> Option<&mut NetworkEntity> {
        if let Some(id) = self.entity_to_network.get(&entity).copied() {
            self.entities.get_mut(&id)
        } else {
            None
        }
    }
    
    /// Process replication for all entities
    pub fn process_replication(&mut self, ecs_world: &EcsWorld) -> Vec<Packet> {
        let mut packets = Vec::new();
        
        // Process spawn queue
        while let Some(network_id) = self.spawn_queue.pop() {
            if let Some(network_entity) = self.entities.get(&network_id) {
                // Get entity position and rotation from ECS
                let (position, rotation, velocity) = if let Some(transform) = ecs_world.get_component::<crate::ecs::components::Transform>(network_entity.entity) {
                    let velocity = ecs_world.get_component::<crate::ecs::components::Physics>(network_entity.entity)
                        .map(|p| Vec3::new(p.velocity.x, p.velocity.y, p.velocity.z))
                        .unwrap_or(Vec3::ZERO);
                    let pos = Vec3::new(transform.position.x, transform.position.y, transform.position.z);
                    let rot = Quat::from_euler(glam::EulerRot::YXZ, transform.rotation.y, transform.rotation.x, transform.rotation.z);
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
            if let Some(transform) = ecs_world.get_component::<crate::ecs::components::Transform>(network_entity.entity) {
                let velocity = ecs_world.get_component::<crate::ecs::components::Physics>(network_entity.entity)
                    .map(|p| Vec3::new(p.velocity.x, p.velocity.y, p.velocity.z))
                    .unwrap_or(Vec3::ZERO);
                
                let pos = Vec3::new(transform.position.x, transform.position.y, transform.position.z);
                let rot = Quat::from_euler(glam::EulerRot::YXZ, transform.rotation.y, transform.rotation.x, transform.rotation.z);
                
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
    network_to_entity: HashMap<NetworkEntityId, Entity>,
}

impl ReplicationReceiver {
    pub fn new() -> Self {
        Self {
            network_to_entity: HashMap::new(),
        }
    }
    
    /// Handle entity spawn packet
    pub fn handle_entity_spawn(&mut self, ecs_world: &mut EcsWorld, 
                              network_id: u32, entity_type: EntityType,
                              position: Vec3, rotation: Quat, velocity: Vec3) {
        let network_id = NetworkEntityId(network_id);
        
        // Create entity
        let entity = ecs_world.create_entity();
        
        // Add transform component
        ecs_world.add_component(entity, crate::ecs::components::Transform {
            position: cgmath::Point3::new(position.x, position.y, position.z),
            rotation: {
                let euler = rotation.to_euler(glam::EulerRot::YXZ);
                cgmath::Vector3::new(euler.1, euler.0, euler.2)
            },
            scale: cgmath::Vector3::new(1.0, 1.0, 1.0),
        });
        
        // Add physics component if has velocity
        if velocity != Vec3::ZERO {
            ecs_world.add_component(entity, crate::ecs::components::Physics {
                velocity: cgmath::Vector3::new(velocity.x, velocity.y, velocity.z),
                acceleration: cgmath::Vector3::new(0.0, 0.0, 0.0),
                mass: 1.0,
                gravity_scale: 1.0,
                drag: 0.1,
                angular_velocity: cgmath::Vector3::new(0.0, 0.0, 0.0),
                bounding_box: crate::physics::AABB {
                    min: cgmath::Point3::new(-0.5, -0.5, -0.5),
                    max: cgmath::Point3::new(0.5, 0.5, 0.5),
                },
                grounded: false,
            });
        }
        
        // Add type-specific components
        match entity_type {
            EntityType::Item { item_id, count } => {
                ecs_world.add_component(entity, crate::ecs::components::ItemComponent {
                    item_id: crate::item::ItemId(item_id),
                    stack_size: count,
                    pickup_delay: 0.0,
                    lifetime: 300.0,
                });
            }
            _ => {}
        }
        
        self.network_to_entity.insert(network_id, entity);
    }
    
    /// Handle entity despawn packet
    pub fn handle_entity_despawn(&mut self, ecs_world: &mut EcsWorld, network_id: u32) {
        let network_id = NetworkEntityId(network_id);
        
        if let Some(entity) = self.network_to_entity.remove(&network_id) {
            ecs_world.remove_entity(entity);
        }
    }
    
    /// Handle entity update packet
    pub fn handle_entity_update(&mut self, ecs_world: &mut EcsWorld,
                               network_id: u32, position: Vec3, rotation: Quat, velocity: Vec3) {
        let network_id = NetworkEntityId(network_id);
        
        if let Some(&entity) = self.network_to_entity.get(&network_id) {
            // Update transform
            if let Some(transform) = ecs_world.get_component_mut::<crate::ecs::components::Transform>(entity) {
                transform.position = cgmath::Point3::new(position.x, position.y, position.z);
                // Convert quaternion to euler angles
                let euler = rotation.to_euler(glam::EulerRot::YXZ);
                transform.rotation = cgmath::Vector3::new(euler.1, euler.0, euler.2);
            }
            
            // Update physics
            if let Some(physics) = ecs_world.get_component_mut::<crate::ecs::components::Physics>(entity) {
                physics.velocity = cgmath::Vector3::new(velocity.x, velocity.y, velocity.z);
            }
        }
    }
}