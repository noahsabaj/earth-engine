/// Zero-allocation light propagation engine
/// Uses pre-allocated buffers to eliminate allocations during light updates

use crate::world::{World, VoxelPos, BlockId};
use crate::lighting::{LightType, MAX_LIGHT_LEVEL, LIGHT_FALLOFF};

/// Pre-allocated buffers for light propagation
struct PropagationBuffers {
    /// Primary queue for light propagation
    light_queue: Vec<(VoxelPos, LightType, u8)>,
    /// Secondary queue for swapping
    light_queue_swap: Vec<(VoxelPos, LightType, u8)>,
    /// Queue for light removal
    removal_queue: Vec<(VoxelPos, LightType, u8)>,
    /// Secondary removal queue for swapping
    removal_queue_swap: Vec<(VoxelPos, LightType, u8)>,
    /// Neighbor positions buffer
    neighbors: [(VoxelPos, bool); 6], // bool indicates if position is valid
}

impl PropagationBuffers {
    fn new(capacity: usize) -> Self {
        Self {
            light_queue: Vec::with_capacity(capacity),
            light_queue_swap: Vec::with_capacity(capacity),
            removal_queue: Vec::with_capacity(capacity),
            removal_queue_swap: Vec::with_capacity(capacity),
            neighbors: [(VoxelPos::new(0, 0, 0), false); 6],
        }
    }
    
    fn clear(&mut self) {
        self.light_queue.clear();
        self.light_queue_swap.clear();
        self.removal_queue.clear();
        self.removal_queue_swap.clear();
    }
    
    /// Fill neighbors buffer for a given position
    #[inline(always)]
    fn fill_neighbors(&mut self, pos: VoxelPos) {
        self.neighbors[0] = (VoxelPos::new(pos.x + 1, pos.y, pos.z), true);
        self.neighbors[1] = (VoxelPos::new(pos.x - 1, pos.y, pos.z), true);
        self.neighbors[2] = (VoxelPos::new(pos.x, pos.y + 1, pos.z), true);
        self.neighbors[3] = (VoxelPos::new(pos.x, pos.y - 1, pos.z), true);
        self.neighbors[4] = (VoxelPos::new(pos.x, pos.y, pos.z + 1), true);
        self.neighbors[5] = (VoxelPos::new(pos.x, pos.y, pos.z - 1), true);
    }
}

/// Optimized light propagator with zero allocations
pub struct OptimizedLightPropagator {
    buffers: PropagationBuffers,
    /// Maximum iterations per frame to prevent hanging
    max_iterations: usize,
}

impl OptimizedLightPropagator {
    pub fn new() -> Self {
        Self {
            buffers: PropagationBuffers::new(4096), // Pre-allocate for typical workload
            max_iterations: 65536, // Limit iterations to prevent hanging
        }
    }
    
    /// Add a light source
    pub fn add_light(&mut self, pos: VoxelPos, light_type: LightType, level: u8) {
        self.buffers.light_queue.push((pos, light_type, level));
    }
    
    /// Remove a light source
    pub fn remove_light(&mut self, pos: VoxelPos, light_type: LightType, level: u8) {
        self.buffers.removal_queue.push((pos, light_type, level));
    }
    
    /// Process all pending light updates without allocating
    pub fn propagate(&mut self, world: &mut World) {
        // Process removals first
        self.process_removals(world);
        
        // Then process additions
        self.process_additions(world);
    }
    
    fn process_removals(&mut self, world: &mut World) {
        let mut iterations = 0;
        
        while !self.buffers.removal_queue.is_empty() && iterations < self.max_iterations {
            // Process current queue
            for i in 0..self.buffers.removal_queue.len() {
                let (pos, light_type, old_level) = self.buffers.removal_queue[i];
                self.remove_light_at_position(world, pos, light_type, old_level);
            }
            
            // Clear current queue and swap
            self.buffers.removal_queue.clear();
            std::mem::swap(&mut self.buffers.removal_queue, &mut self.buffers.removal_queue_swap);
            
            iterations += 1;
        }
        
        // Clear any remaining items
        self.buffers.removal_queue.clear();
        self.buffers.removal_queue_swap.clear();
    }
    
    fn process_additions(&mut self, world: &mut World) {
        let mut iterations = 0;
        
        while !self.buffers.light_queue.is_empty() && iterations < self.max_iterations {
            // Process current queue
            for i in 0..self.buffers.light_queue.len() {
                let (pos, light_type, level) = self.buffers.light_queue[i];
                self.propagate_light_at_position(world, pos, light_type, level);
            }
            
            // Clear current queue and swap
            self.buffers.light_queue.clear();
            std::mem::swap(&mut self.buffers.light_queue, &mut self.buffers.light_queue_swap);
            
            iterations += 1;
        }
        
        // Clear any remaining items
        self.buffers.light_queue.clear();
        self.buffers.light_queue_swap.clear();
    }
    
    #[inline]
    fn propagate_light_at_position(&mut self, world: &mut World, pos: VoxelPos, light_type: LightType, level: u8) {
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
            self.buffers.fill_neighbors(pos);
            
            for i in 0..6 {
                let (neighbor, valid) = self.buffers.neighbors[i];
                if !valid {
                    continue;
                }
                
                // For skylight, only propagate downward at full strength
                if light_type == LightType::Sky && neighbor.y < pos.y && level == MAX_LIGHT_LEVEL {
                    self.buffers.light_queue_swap.push((neighbor, light_type, MAX_LIGHT_LEVEL));
                } else {
                    self.buffers.light_queue_swap.push((neighbor, light_type, next_level));
                }
            }
        }
    }
    
    #[inline]
    fn remove_light_at_position(&mut self, world: &mut World, pos: VoxelPos, light_type: LightType, old_level: u8) {
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
        self.buffers.fill_neighbors(pos);
        
        // Check neighbors and queue for removal or re-lighting
        for i in 0..6 {
            let (neighbor, valid) = self.buffers.neighbors[i];
            if !valid {
                continue;
            }
            
            let neighbor_level = match light_type {
                LightType::Sky => world.get_sky_light(neighbor),
                LightType::Block => world.get_block_light(neighbor),
            };
            
            if neighbor_level > 0 && neighbor_level < old_level {
                // This neighbor was lit by us, remove it
                self.buffers.removal_queue_swap.push((neighbor, light_type, neighbor_level));
            } else if neighbor_level >= old_level {
                // This neighbor has its own light source, re-propagate
                self.buffers.light_queue_swap.push((neighbor, light_type, neighbor_level));
            }
        }
    }
    
    /// Calculate initial skylight for a chunk without allocations
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
    
    /// Clear all buffers - useful for cleanup
    pub fn clear(&mut self) {
        self.buffers.clear();
    }
    
    /// Get current queue sizes for debugging
    pub fn queue_sizes(&self) -> (usize, usize) {
        (self.buffers.light_queue.len(), self.buffers.removal_queue.len())
    }
}

// Thread-local optimized propagator for parallel chunk processing
thread_local! {
    static LOCAL_PROPAGATOR: std::cell::RefCell<OptimizedLightPropagator> = 
        std::cell::RefCell::new(OptimizedLightPropagator::new());
}

/// Process light propagation using thread-local propagator
pub fn propagate_light_thread_local<F>(f: F)
where
    F: FnOnce(&mut OptimizedLightPropagator),
{
    LOCAL_PROPAGATOR.with(|propagator| {
        f(&mut propagator.borrow_mut());
    });
}