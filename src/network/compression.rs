use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::world::VoxelPos;
use crate::inventory::InventorySlot;
use glam::{Vec3, Quat};

/// Delta compression for entity state updates
#[derive(Debug, Clone)]
pub struct EntityStateDelta {
    /// Entity ID
    pub entity_id: u32,
    /// Changed fields
    pub changes: EntityFieldChanges,
}

/// Bitmask for changed entity fields
#[derive(Debug, Clone, Copy, Default)]
pub struct EntityFieldMask {
    bits: u32,
}

impl EntityFieldMask {
    pub const POSITION: u32 = 1 << 0;
    pub const ROTATION: u32 = 1 << 1;
    pub const VELOCITY: u32 = 1 << 2;
    pub const HEALTH: u32 = 1 << 3;
    pub const ANIMATION: u32 = 1 << 4;
    pub const EQUIPMENT: u32 = 1 << 5;
    pub const METADATA: u32 = 1 << 6;
    pub const MOVEMENT_STATE: u32 = 1 << 7;
    
    pub fn new() -> Self {
        Self { bits: 0 }
    }
    
    pub fn set(&mut self, flag: u32) {
        self.bits |= flag;
    }
    
    pub fn has(&self, flag: u32) -> bool {
        self.bits & flag != 0
    }
    
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }
}

/// Changed entity fields
#[derive(Debug, Clone)]
pub struct EntityFieldChanges {
    pub mask: EntityFieldMask,
    pub position: Option<Vec3>,
    pub rotation: Option<Quat>,
    pub velocity: Option<Vec3>,
    pub health: Option<f32>,
    pub animation: Option<u8>,
    pub equipment: Option<Vec<InventorySlot>>,
    pub metadata: Option<HashMap<String, String>>,
    pub movement_state: Option<u8>,
}

impl EntityFieldChanges {
    pub fn new() -> Self {
        Self {
            mask: EntityFieldMask::new(),
            position: None,
            rotation: None,
            velocity: None,
            health: None,
            animation: None,
            equipment: None,
            metadata: None,
            movement_state: None,
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.mask.is_empty()
    }
}

/// Full entity state for comparison
#[derive(Debug, Clone)]
pub struct EntityState {
    pub entity_id: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub health: f32,
    pub animation: u8,
    pub equipment: Vec<InventorySlot>,
    pub metadata: HashMap<String, String>,
    pub movement_state: u8,
}

/// Delta encoder for entity states
pub struct DeltaEncoder {
    /// Previous states for each entity
    previous_states: HashMap<u32, EntityState>,
    /// Position quantization (1cm precision)
    position_precision: f32,
    /// Rotation quantization (degrees)
    rotation_precision: f32,
}

impl DeltaEncoder {
    pub fn new() -> Self {
        Self {
            previous_states: HashMap::new(),
            position_precision: 0.01, // 1cm
            rotation_precision: 1.0,   // 1 degree
        }
    }
    
    /// Encode entity state as delta
    pub fn encode_delta(&mut self, current: &EntityState) -> EntityStateDelta {
        let mut changes = EntityFieldChanges::new();
        
        if let Some(previous) = self.previous_states.get(&current.entity_id) {
            // Position delta
            if self.position_changed(&previous.position, &current.position) {
                changes.mask.set(EntityFieldMask::POSITION);
                changes.position = Some(current.position);
            }
            
            // Rotation delta
            if self.rotation_changed(&previous.rotation, &current.rotation) {
                changes.mask.set(EntityFieldMask::ROTATION);
                changes.rotation = Some(current.rotation);
            }
            
            // Velocity delta
            if self.velocity_changed(&previous.velocity, &current.velocity) {
                changes.mask.set(EntityFieldMask::VELOCITY);
                changes.velocity = Some(current.velocity);
            }
            
            // Health delta
            if (previous.health - current.health).abs() > 0.01 {
                changes.mask.set(EntityFieldMask::HEALTH);
                changes.health = Some(current.health);
            }
            
            // Animation delta
            if previous.animation != current.animation {
                changes.mask.set(EntityFieldMask::ANIMATION);
                changes.animation = Some(current.animation);
            }
            
            // Movement state delta
            if previous.movement_state != current.movement_state {
                changes.mask.set(EntityFieldMask::MOVEMENT_STATE);
                changes.movement_state = Some(current.movement_state);
            }
            
            // Equipment delta (simplified - just check if changed)
            if previous.equipment.len() != current.equipment.len() {
                changes.mask.set(EntityFieldMask::EQUIPMENT);
                changes.equipment = Some(current.equipment.clone());
            }
            
            // Metadata delta
            if previous.metadata != current.metadata {
                changes.mask.set(EntityFieldMask::METADATA);
                changes.metadata = Some(current.metadata.clone());
            }
        } else {
            // First time seeing this entity - send full state
            changes.mask.set(EntityFieldMask::POSITION);
            changes.mask.set(EntityFieldMask::ROTATION);
            changes.mask.set(EntityFieldMask::VELOCITY);
            changes.mask.set(EntityFieldMask::HEALTH);
            changes.mask.set(EntityFieldMask::ANIMATION);
            changes.mask.set(EntityFieldMask::EQUIPMENT);
            changes.mask.set(EntityFieldMask::MOVEMENT_STATE);
            
            changes.position = Some(current.position);
            changes.rotation = Some(current.rotation);
            changes.velocity = Some(current.velocity);
            changes.health = Some(current.health);
            changes.animation = Some(current.animation);
            changes.equipment = Some(current.equipment.clone());
            changes.movement_state = Some(current.movement_state);
            
            if !current.metadata.is_empty() {
                changes.mask.set(EntityFieldMask::METADATA);
                changes.metadata = Some(current.metadata.clone());
            }
        }
        
        // Store current state for next delta
        self.previous_states.insert(current.entity_id, current.clone());
        
        EntityStateDelta {
            entity_id: current.entity_id,
            changes,
        }
    }
    
    /// Remove entity from tracking
    pub fn remove_entity(&mut self, entity_id: u32) {
        self.previous_states.remove(&entity_id);
    }
    
    /// Clear all tracked states
    pub fn clear(&mut self) {
        self.previous_states.clear();
    }
    
    fn position_changed(&self, a: &Vec3, b: &Vec3) -> bool {
        (a.x - b.x).abs() > self.position_precision ||
        (a.y - b.y).abs() > self.position_precision ||
        (a.z - b.z).abs() > self.position_precision
    }
    
    fn rotation_changed(&self, a: &Quat, b: &Quat) -> bool {
        // Compare quaternions by angle difference
        let angle = a.angle_between(*b).to_degrees();
        angle > self.rotation_precision
    }
    
    fn velocity_changed(&self, a: &Vec3, b: &Vec3) -> bool {
        (a.x - b.x).abs() > 0.1 ||
        (a.y - b.y).abs() > 0.1 ||
        (a.z - b.z).abs() > 0.1
    }
}

/// Delta decoder for entity states
pub struct DeltaDecoder {
    /// Current states for each entity
    current_states: HashMap<u32, EntityState>,
}

impl DeltaDecoder {
    pub fn new() -> Self {
        Self {
            current_states: HashMap::new(),
        }
    }
    
    /// Apply delta to get full state
    pub fn apply_delta(&mut self, delta: &EntityStateDelta) -> Option<EntityState> {
        let state = self.current_states.entry(delta.entity_id)
            .or_insert_with(|| EntityState {
                entity_id: delta.entity_id,
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                velocity: Vec3::ZERO,
                health: 100.0,
                animation: 0,
                equipment: vec![],
                metadata: HashMap::new(),
                movement_state: 0,
            });
        
        // Apply changes
        if let Some(pos) = &delta.changes.position {
            state.position = *pos;
        }
        if let Some(rot) = &delta.changes.rotation {
            state.rotation = *rot;
        }
        if let Some(vel) = &delta.changes.velocity {
            state.velocity = *vel;
        }
        if let Some(health) = delta.changes.health {
            state.health = health;
        }
        if let Some(anim) = delta.changes.animation {
            state.animation = anim;
        }
        if let Some(equip) = &delta.changes.equipment {
            state.equipment = equip.clone();
        }
        if let Some(meta) = &delta.changes.metadata {
            state.metadata = meta.clone();
        }
        if let Some(movement) = delta.changes.movement_state {
            state.movement_state = movement;
        }
        
        Some(state.clone())
    }
    
    /// Remove entity
    pub fn remove_entity(&mut self, entity_id: u32) {
        self.current_states.remove(&entity_id);
    }
    
    /// Get current state
    pub fn get_state(&self, entity_id: u32) -> Option<&EntityState> {
        self.current_states.get(&entity_id)
    }
}

/// Chunk delta compression
#[derive(Debug, Clone)]
pub struct ChunkDelta {
    pub chunk_pos: VoxelPos,
    pub changes: Vec<CompressedBlockChange>,
}

#[derive(Debug, Clone)]
pub struct CompressedBlockChange {
    pub offset: u16, // Packed offset within chunk (5 bits each for x,y,z)
    pub block_id: u16,
}

impl CompressedBlockChange {
    pub fn new(x: u8, y: u8, z: u8, block_id: u16) -> Self {
        debug_assert!(x < 32 && y < 32 && z < 32);
        let offset = ((x as u16) << 10) | ((y as u16) << 5) | (z as u16);
        Self { offset, block_id }
    }
    
    pub fn unpack(&self) -> (u8, u8, u8) {
        let x = ((self.offset >> 10) & 0x1F) as u8;
        let y = ((self.offset >> 5) & 0x1F) as u8;
        let z = (self.offset & 0x1F) as u8;
        (x, y, z)
    }
}

/// Run-length encoding for chunk data
pub struct ChunkCompressor;

impl ChunkCompressor {
    /// Compress chunk data using RLE
    pub fn compress(data: &[u16]) -> Vec<u8> {
        let mut compressed = Vec::new();
        let mut i = 0;
        
        while i < data.len() {
            let block = data[i];
            let mut count = 1;
            
            // Count consecutive same blocks
            while i + count < data.len() && data[i + count] == block && count < 255 {
                count += 1;
            }
            
            // Write count and block ID
            compressed.push(count as u8);
            compressed.extend_from_slice(&block.to_le_bytes());
            
            i += count;
        }
        
        compressed
    }
    
    /// Decompress chunk data
    pub fn decompress(compressed: &[u8]) -> Result<Vec<u16>, &'static str> {
        let mut data = Vec::new();
        let mut i = 0;
        
        while i < compressed.len() {
            if i + 2 >= compressed.len() {
                return Err("Invalid compressed data");
            }
            
            let count = compressed[i];
            let block_bytes = [compressed[i + 1], compressed[i + 2]];
            let block = u16::from_le_bytes(block_bytes);
            
            for _ in 0..count {
                data.push(block);
            }
            
            i += 3;
        }
        
        Ok(data)
    }
}

/// Network packet size optimizer
pub struct PacketOptimizer {
    /// Maximum packet size (bytes)
    max_packet_size: usize,
}

impl PacketOptimizer {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_packet_size: max_size,
        }
    }
    
    /// Split entity updates into multiple packets if needed
    pub fn split_entity_updates(&self, updates: Vec<EntityStateDelta>) -> Vec<Vec<EntityStateDelta>> {
        let mut packets = vec![];
        let mut current_packet = vec![];
        let mut current_size = 0;
        
        for update in updates {
            // Estimate size (rough approximation)
            let update_size = self.estimate_delta_size(&update);
            
            if current_size + update_size > self.max_packet_size && !current_packet.is_empty() {
                packets.push(current_packet);
                current_packet = vec![];
                current_size = 0;
            }
            
            current_size += update_size;
            current_packet.push(update);
        }
        
        if !current_packet.is_empty() {
            packets.push(current_packet);
        }
        
        packets
    }
    
    fn estimate_delta_size(&self, delta: &EntityStateDelta) -> usize {
        let mut size = 8; // Entity ID + field mask
        
        if delta.changes.position.is_some() { size += 12; }
        if delta.changes.rotation.is_some() { size += 16; }
        if delta.changes.velocity.is_some() { size += 12; }
        if delta.changes.health.is_some() { size += 4; }
        if delta.changes.animation.is_some() { size += 1; }
        if delta.changes.movement_state.is_some() { size += 1; }
        if let Some(equip) = &delta.changes.equipment {
            size += 2 + equip.len() * 8;
        }
        if let Some(meta) = &delta.changes.metadata {
            size += 2 + meta.len() * 32; // Rough estimate
        }
        
        size
    }
}