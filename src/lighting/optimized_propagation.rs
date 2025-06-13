/// Data-Oriented Light Propagation System
/// 
/// Sprint 37: Converted from OOP to pure functions operating on data buffers.
/// Zero-allocation light propagation using pre-allocated buffers.
/// Pure functions for light update transformations - no methods, just data operations.

use crate::world::{World, VoxelPos, BlockId};
use crate::lighting::{LightType, MAX_LIGHT_LEVEL, LIGHT_FALLOFF};

/// Pre-allocated buffers for light propagation (pure data)
pub struct PropagationBuffers {
    /// Primary queue for light propagation
    pub light_queue: Vec<(VoxelPos, LightType, u8)>,
    /// Secondary queue for swapping
    pub light_queue_swap: Vec<(VoxelPos, LightType, u8)>,
    /// Queue for light removal
    pub removal_queue: Vec<(VoxelPos, LightType, u8)>,
    /// Secondary removal queue for swapping
    pub removal_queue_swap: Vec<(VoxelPos, LightType, u8)>,
    /// Neighbor positions buffer
    pub neighbors: [(VoxelPos, bool); 6], // bool indicates if position is valid
}

/// Create new propagation buffers
/// Pure function - returns pre-allocated data structure
pub fn create_propagation_buffers(capacity: usize) -> PropagationBuffers {
    PropagationBuffers {
        light_queue: Vec::with_capacity(capacity),
        light_queue_swap: Vec::with_capacity(capacity),
        removal_queue: Vec::with_capacity(capacity),
        removal_queue_swap: Vec::with_capacity(capacity),
        neighbors: [(VoxelPos::new(0, 0, 0), false); 6],
    }
}

/// Clear all buffers
/// Function - transforms buffer data to empty state
pub fn clear_propagation_buffers(buffers: &mut PropagationBuffers) {
    buffers.light_queue.clear();
    buffers.light_queue_swap.clear();
    buffers.removal_queue.clear();
    buffers.removal_queue_swap.clear();
}

/// Fill neighbors buffer for a given position
/// Function - transforms neighbors array with position data
#[inline(always)]
pub fn fill_neighbors(buffers: &mut PropagationBuffers, pos: VoxelPos) {
    buffers.neighbors[0] = (VoxelPos::new(pos.x + 1, pos.y, pos.z), true);
    buffers.neighbors[1] = (VoxelPos::new(pos.x - 1, pos.y, pos.z), true);
    buffers.neighbors[2] = (VoxelPos::new(pos.x, pos.y + 1, pos.z), true);
    buffers.neighbors[3] = (VoxelPos::new(pos.x, pos.y - 1, pos.z), true);
    buffers.neighbors[4] = (VoxelPos::new(pos.x, pos.y, pos.z + 1), true);
    buffers.neighbors[5] = (VoxelPos::new(pos.x, pos.y, pos.z - 1), true);
}

/// Light propagation data (no methods)
/// Pure data - manipulated by free functions only
pub struct LightPropagatorData {
    pub buffers: PropagationBuffers,
    /// Maximum iterations per frame to prevent hanging
    pub max_iterations: usize,
}

/// Light propagation configuration
#[derive(Debug, Copy, Clone)]
pub struct LightPropagatorConfig {
    pub buffer_capacity: usize,
    pub max_iterations: usize,
}

impl Default for LightPropagatorConfig {
    fn default() -> Self {
        Self {
            buffer_capacity: 4096,
            max_iterations: 65536,
        }
    }
}

/// Create new light propagator data
/// Pure function - returns data structure with pre-allocated buffers
pub fn create_light_propagator_data(config: LightPropagatorConfig) -> LightPropagatorData {
    LightPropagatorData {
        buffers: create_propagation_buffers(config.buffer_capacity),
        max_iterations: config.max_iterations,
    }
}

/// Add a light source to queue
/// Function - transforms light queue by adding light update
pub fn add_light(data: &mut LightPropagatorData, pos: VoxelPos, light_type: LightType, level: u8) {
    data.buffers.light_queue.push((pos, light_type, level));
}

/// Remove a light source from queue
/// Function - transforms removal queue by adding light removal
pub fn remove_light(data: &mut LightPropagatorData, pos: VoxelPos, light_type: LightType, level: u8) {
    data.buffers.removal_queue.push((pos, light_type, level));
}

/// Process all pending light updates without allocating
/// Function - transforms world light data based on queued updates
pub fn propagate_light(data: &mut LightPropagatorData, world: &mut World) {
    // Process removals first
    process_light_removals(data, world);
    
    // Then process additions
    process_light_additions(data, world);
}

/// Process light removals
/// Function - transforms world light data by removing lights
fn process_light_removals(data: &mut LightPropagatorData, world: &mut World) {
    let mut iterations = 0;
    
    while !data.buffers.removal_queue.is_empty() && iterations < data.max_iterations {
        // Process current queue
        for i in 0..data.buffers.removal_queue.len() {
            let (pos, light_type, old_level) = data.buffers.removal_queue[i];
            remove_light_at_position(data, world, pos, light_type, old_level);
        }
        
        // Clear current queue and swap
        data.buffers.removal_queue.clear();
        std::mem::swap(&mut data.buffers.removal_queue, &mut data.buffers.removal_queue_swap);
        
        iterations += 1;
    }
    
    // Clear any remaining items
    data.buffers.removal_queue.clear();
    data.buffers.removal_queue_swap.clear();
}

/// Process light additions
/// Function - transforms world light data by adding lights
fn process_light_additions(data: &mut LightPropagatorData, world: &mut World) {
    let mut iterations = 0;
    
    while !data.buffers.light_queue.is_empty() && iterations < data.max_iterations {
        // Process current queue
        for i in 0..data.buffers.light_queue.len() {
            let (pos, light_type, level) = data.buffers.light_queue[i];
            propagate_light_at_position(data, world, pos, light_type, level);
        }
        
        // Clear current queue and swap
        data.buffers.light_queue.clear();
        std::mem::swap(&mut data.buffers.light_queue, &mut data.buffers.light_queue_swap);
        
        iterations += 1;
    }
    
    // Clear any remaining items
    data.buffers.light_queue.clear();
    data.buffers.light_queue_swap.clear();
}

/// Propagate light at specific position
/// Function - transforms world light data by adding light and propagating to neighbors
#[inline]
fn propagate_light_at_position(data: &mut LightPropagatorData, world: &mut World, pos: VoxelPos, light_type: LightType, level: u8) {
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
        
        // Fill neighbors buffer
        fill_neighbors(&mut data.buffers, pos);
        
        for i in 0..6 {
            let (neighbor, valid) = data.buffers.neighbors[i];
            if !valid {
                continue;
            }
            
            // For skylight, only propagate downward at full strength
            if light_type == LightType::Sky && neighbor.y < pos.y && level == MAX_LIGHT_LEVEL {
                data.buffers.light_queue_swap.push((neighbor, light_type, MAX_LIGHT_LEVEL));
            } else {
                data.buffers.light_queue_swap.push((neighbor, light_type, next_level));
            }
        }
    }
}

/// Remove light at specific position
/// Function - transforms world light data by removing light and handling neighbors
#[inline]
fn remove_light_at_position(data: &mut LightPropagatorData, world: &mut World, pos: VoxelPos, light_type: LightType, old_level: u8) {
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
    
    // Fill neighbors buffer
    fill_neighbors(&mut data.buffers, pos);
    
    // Check neighbors and queue for removal or re-lighting
    for i in 0..6 {
        let (neighbor, valid) = data.buffers.neighbors[i];
        if !valid {
            continue;
        }
        
        let neighbor_level = match light_type {
            LightType::Sky => world.get_sky_light(neighbor),
            LightType::Block => world.get_block_light(neighbor),
        };
        
        if neighbor_level > 0 && neighbor_level < old_level {
            // This neighbor was lit by us, remove it
            data.buffers.removal_queue_swap.push((neighbor, light_type, neighbor_level));
        } else if neighbor_level >= old_level {
            // This neighbor has its own light source, re-propagate
            data.buffers.light_queue_swap.push((neighbor, light_type, neighbor_level));
        }
    }
}

/// Calculate initial skylight for a chunk without allocations
/// Pure function - transforms world skylight data for entire chunk
pub fn calculate_chunk_skylight(world: &mut World, chunk_x: i32, chunk_y: i32, chunk_z: i32, chunk_size: u32) {
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
                
                // Check if current block blocks light
                if world.get_block(pos) != BlockId::AIR && !world.is_block_transparent(pos) {
                    light_level = 0;
                }
                
                // Set skylight level
                world.set_sky_light(pos, light_level);
            }
        }
    }
}

/// Clear all queues
/// Function - transforms propagator data to empty state
pub fn clear_light_propagator(data: &mut LightPropagatorData) {
    clear_propagation_buffers(&mut data.buffers);
}

/// Get current queue sizes for debugging
/// Pure function - reads queue lengths
pub fn get_queue_sizes(data: &LightPropagatorData) -> (usize, usize) {
    (data.buffers.light_queue.len(), data.buffers.removal_queue.len())
}

/// Get the number of pending light updates
/// Pure function - reads total pending updates
pub fn get_pending_updates(data: &LightPropagatorData) -> usize {
    data.buffers.light_queue.len() + data.buffers.removal_queue.len()
}

// ===== THREAD-LOCAL SUPPORT =====
// Using thread-local data for performance in parallel scenarios

thread_local! {
    static LOCAL_PROPAGATOR: std::cell::RefCell<LightPropagatorData> = 
        std::cell::RefCell::new(create_light_propagator_data(LightPropagatorConfig::default()));
}

/// Process light propagation using thread-local propagator
/// Function - transforms world using thread-local light propagator data
pub fn propagate_light_thread_local<F>(f: F)
where
    F: FnOnce(&mut LightPropagatorData),
{
    LOCAL_PROPAGATOR.with(|propagator| {
        f(&mut propagator.borrow_mut());
    });
}

// ===== COMPATIBILITY LAYER =====
// Temporary aliases for code that hasn't been converted yet

/// Compatibility alias - will be removed in future sprints
#[deprecated(note = "Use LightPropagatorData and pure functions instead")]
pub type OptimizedLightPropagator = LightPropagatorData;