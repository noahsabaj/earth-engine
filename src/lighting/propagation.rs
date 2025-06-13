/// Data-Oriented Light Propagation Functions
/// 
/// Pure functions for light propagation using flood-fill algorithm.
/// No methods, no self, just data transformations.
/// Follows DOP principles from Sprint 37.

use std::collections::VecDeque;
use crate::world::{WorldInterface, VoxelPos, BlockId};
use crate::lighting::{LightType, MAX_LIGHT_LEVEL, LIGHT_FALLOFF};

/// Light propagation data (DOP - no methods)
/// Pure data structure for light propagation state
pub struct LightPropagatorData {
    /// Queue of positions to propagate light from
    pub light_queue: VecDeque<(VoxelPos, LightType, u8)>,
    /// Queue of positions to remove light from
    pub removal_queue: VecDeque<(VoxelPos, LightType, u8)>,
}

/// Create new light propagator data
/// Pure function - returns data structure, no behavior
pub fn create_light_propagator_data() -> LightPropagatorData {
    LightPropagatorData {
        light_queue: VecDeque::new(),
        removal_queue: VecDeque::new(),
    }
}

/// Add a light source
/// Function - transforms propagator data by queuing light addition
pub fn add_light_to_queue(data: &mut LightPropagatorData, pos: VoxelPos, light_type: LightType, level: u8) {
    data.light_queue.push_back((pos, light_type, level));
}

/// Remove a light source
/// Function - transforms propagator data by queuing light removal
pub fn remove_light_from_queue(data: &mut LightPropagatorData, pos: VoxelPos, light_type: LightType, level: u8) {
    data.removal_queue.push_back((pos, light_type, level));
}

/// Process all pending light updates
/// Function - transforms world data by processing light propagation queue
pub fn propagate_queued_lights(data: &mut LightPropagatorData, world: &mut dyn WorldInterface) {
    // First, process removals
    while let Some((pos, light_type, old_level)) = data.removal_queue.pop_front() {
        remove_light_recursive(data, world, pos, light_type, old_level);
    }
    
    // Then, process additions
    while let Some((pos, light_type, level)) = data.light_queue.pop_front() {
        propagate_light_recursive(data, world, pos, light_type, level);
    }
}

/// Propagate light recursively
/// Function - transforms world data by recursive light spreading
fn propagate_light_recursive(data: &mut LightPropagatorData, world: &mut dyn WorldInterface, pos: VoxelPos, light_type: LightType, level: u8) {
        // Skip if position is solid
        if world.get_block(pos) != BlockId::AIR && !world.is_block_transparent(pos) {
            return;
        }
        
        // Get current light level at this position
        let current_level = match light_type {
            LightType::Sky => world.get_sky_light(pos),
            LightType::Block => world.get_block_light(pos),
        };
        
        // Only update if new level is higher
        if level <= current_level {
            return;
        }
        
        // Set the new light level
        match light_type {
            LightType::Sky => world.set_sky_light(pos, level),
            LightType::Block => world.set_block_light(pos, level),
        }
        
        // Propagate to neighbors if there's light left to spread
        if level > LIGHT_FALLOFF {
            let next_level = level - LIGHT_FALLOFF;
            
            // Check all 6 neighbors
            let neighbors = [
                VoxelPos::new(pos.x + 1, pos.y, pos.z),
                VoxelPos::new(pos.x - 1, pos.y, pos.z),
                VoxelPos::new(pos.x, pos.y + 1, pos.z),
                VoxelPos::new(pos.x, pos.y - 1, pos.z),
                VoxelPos::new(pos.x, pos.y, pos.z + 1),
                VoxelPos::new(pos.x, pos.y, pos.z - 1),
            ];
            
            for neighbor in neighbors {
                // For skylight, only propagate downward at full strength
                if light_type == LightType::Sky && neighbor.y < pos.y && level == MAX_LIGHT_LEVEL {
                    data.light_queue.push_back((neighbor, light_type, MAX_LIGHT_LEVEL));
                } else {
                    data.light_queue.push_back((neighbor, light_type, next_level));
                }
            }
        }
    }

/// Remove light recursively
/// Function - transforms world data by recursive light removal
fn remove_light_recursive(data: &mut LightPropagatorData, world: &mut dyn WorldInterface, pos: VoxelPos, light_type: LightType, old_level: u8) {
        // Get current light level
        let current_level = match light_type {
            LightType::Sky => world.get_sky_light(pos),
            LightType::Block => world.get_block_light(pos),
        };
        
        // If light level has changed, skip (already processed or re-lit)
        if current_level != old_level {
            return;
        }
        
        // Clear the light at this position
        match light_type {
            LightType::Sky => world.set_sky_light(pos, 0),
            LightType::Block => world.set_block_light(pos, 0),
        }
        
        // Check neighbors and queue for removal or re-lighting
        let neighbors = [
            VoxelPos::new(pos.x + 1, pos.y, pos.z),
            VoxelPos::new(pos.x - 1, pos.y, pos.z),
            VoxelPos::new(pos.x, pos.y + 1, pos.z),
            VoxelPos::new(pos.x, pos.y - 1, pos.z),
            VoxelPos::new(pos.x, pos.y, pos.z + 1),
            VoxelPos::new(pos.x, pos.y, pos.z - 1),
        ];
        
        for neighbor in neighbors {
            let neighbor_level = match light_type {
                LightType::Sky => world.get_sky_light(neighbor),
                LightType::Block => world.get_block_light(neighbor),
            };
            
            if neighbor_level > 0 && neighbor_level < old_level {
                // This neighbor was lit by us, remove it
                data.removal_queue.push_back((neighbor, light_type, neighbor_level));
            } else if neighbor_level >= old_level {
                // This neighbor has its own light source, re-propagate
                data.light_queue.push_back((neighbor, light_type, neighbor_level));
            }
        }
    }

/// Calculate initial skylight for a chunk
/// Function - transforms world data by calculating skylight for chunk area
pub fn calculate_chunk_skylight(world: &mut dyn WorldInterface, chunk_x: i32, chunk_y: i32, chunk_z: i32, chunk_size: u32) {
        let world_x_start = chunk_x * chunk_size as i32;
        let world_y_start = chunk_y * chunk_size as i32;
        let world_z_start = chunk_z * chunk_size as i32;
        
        // For each column in the chunk
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                let world_x = world_x_start + x as i32;
                let world_z = world_z_start + z as i32;
                
                // Start from the top of the chunk
                let mut light_level = MAX_LIGHT_LEVEL;
                
                for y in (0..chunk_size).rev() {
                    let world_y = world_y_start + y as i32;
                    let pos = VoxelPos::new(world_x, world_y, world_z);
                    
                    // Check if this block blocks light
                    if world.get_block(pos) != BlockId::AIR && !world.is_block_transparent(pos) {
                        light_level = 0;
                    }
                    
                    // Set skylight level
                    world.set_sky_light(pos, light_level);
                }
            }
        }
    }

// ===== COMPATIBILITY LAYER =====
// Temporary aliases for code that hasn't been converted yet

/// Compatibility alias - will be removed in future sprints
#[deprecated(note = "Use LightPropagatorData and pure functions instead")]
pub type LightPropagator = LightPropagatorData;

/// Compatibility implementation for gradual migration
#[allow(deprecated)]
impl LightPropagator {
    /// Compatibility constructor
    #[deprecated(note = "Use create_light_propagator_data instead")]
    pub fn new() -> Self {
        create_light_propagator_data()
    }
    
    /// Compatibility method
    #[deprecated(note = "Use add_light_to_queue instead")]
    pub fn add_light(&mut self, pos: VoxelPos, light_type: LightType, level: u8) {
        add_light_to_queue(self, pos, light_type, level);
    }
    
    /// Compatibility method
    #[deprecated(note = "Use remove_light_from_queue instead")]
    pub fn remove_light(&mut self, pos: VoxelPos, light_type: LightType, level: u8) {
        remove_light_from_queue(self, pos, light_type, level);
    }
    
    /// Compatibility method
    #[deprecated(note = "Use propagate_queued_lights instead")]
    pub fn propagate(&mut self, world: &mut dyn WorldInterface) {
        propagate_queued_lights(self, world);
    }
}