use super::{PhysicsData, EntityId, physics_tables};
use rayon::prelude::*;

// Import physics constants from main physics module
use crate::physics::{FIXED_TIMESTEP, GRAVITY, TERMINAL_VELOCITY};

/// Physics integrator for updating positions and velocities
pub struct PhysicsIntegrator {
    accumulator: f32,
    alpha: f32,
    previous_positions: Vec<[f32; 3]>,
    previous_velocities: Vec<[f32; 3]>,
}

impl PhysicsIntegrator {
    pub fn new(max_entities: usize) -> Self {
        Self {
            accumulator: 0.0,
            alpha: 0.0,
            previous_positions: Vec::with_capacity(max_entities),
            previous_velocities: Vec::with_capacity(max_entities),
        }
    }
    
    /// Update physics with fixed timestep and interpolation
    pub fn update<F>(&mut self, physics_data: &mut PhysicsData, frame_time: f32, mut step_fn: F)
    where
        F: FnMut(&mut PhysicsData, f32),
    {
        // Clamp frame time to prevent spiral of death
        let frame_time = frame_time.min(0.25);
        
        self.accumulator += frame_time;
        
        // Fixed timestep integration
        while self.accumulator >= FIXED_TIMESTEP {
            // Save previous state for interpolation
            self.save_previous_state(physics_data);
            
            // Run physics step
            step_fn(physics_data, FIXED_TIMESTEP);
            
            self.accumulator -= FIXED_TIMESTEP;
        }
        
        // Calculate interpolation factor
        self.alpha = self.accumulator / FIXED_TIMESTEP;
    }
    
    /// Save previous state for interpolation
    fn save_previous_state(&mut self, physics_data: &PhysicsData) {
        let count = physics_data.entity_count();
        
        // Resize if needed
        self.previous_positions.resize(count, [0.0; 3]);
        self.previous_velocities.resize(count, [0.0; 3]);
        
        // Copy current state
        self.previous_positions[..count].copy_from_slice(&physics_data.positions[..count]);
        self.previous_velocities[..count].copy_from_slice(&physics_data.velocities[..count]);
    }
    
    /// Get interpolated position for rendering
    pub fn get_interpolated_position(&self, entity: EntityId, physics_data: &PhysicsData) -> Option<[f32; 3]> {
        let idx = entity.index();
        
        // Get both previous and current positions safely
        let prev = self.previous_positions.get(idx)?;
        let curr = physics_data.positions.get(idx)?;
        
        Some([
            prev[0] + (curr[0] - prev[0]) * self.alpha,
            prev[1] + (curr[1] - prev[1]) * self.alpha,
            prev[2] + (curr[2] - prev[2]) * self.alpha,
        ])
    }
    
    /// Apply forces to entities
    pub fn apply_forces(physics_data: &mut PhysicsData, forces: &[[f32; 3]], dt: f32) {
        let count = physics_data.entity_count().min(forces.len());
        
        // Get mutable slices for parallel iteration
        let velocities = &mut physics_data.velocities[..count];
        let flags = &physics_data.flags[..count];
        let inverse_masses = &physics_data.inverse_masses[..count];
        let forces = &forces[..count];
        
        // Use zip to iterate over multiple slices in parallel
        velocities.par_iter_mut()
            .zip(flags.par_iter())
            .zip(inverse_masses.par_iter())
            .zip(forces.par_iter())
            .for_each(|(((vel, flag), &inv_mass), force)| {
                if flag.is_dynamic() {
                    // F = ma, so a = F/m = F * inv_mass
                    vel[0] += force[0] * inv_mass * dt;
                    vel[1] += force[1] * inv_mass * dt;
                    vel[2] += force[2] * inv_mass * dt;
                }
            });
    }
    
    /// Apply impulses to entities
    pub fn apply_impulses(physics_data: &mut PhysicsData, impulses: &[(EntityId, [f32; 3])]) {
        // For impulses, we need to handle potentially non-contiguous updates
        // We'll use a sequential approach for simplicity and correctness
        // This is typically fine since impulse application is less frequent than force integration
        
        for (entity, impulse) in impulses {
            let idx = entity.index();
            if idx < physics_data.entity_count() {
                // Safely check flags
                if let Some(flag) = physics_data.flags.get(idx) {
                    if flag.is_dynamic() {
                        // Safely get inverse mass and velocity
                        if let (Some(inv_mass), Some(velocity)) = (
                            physics_data.inverse_masses.get(idx),
                            physics_data.velocities.get_mut(idx)
                        ) {
                            // Impulse = change in momentum = m * Δv
                            // So Δv = impulse / m = impulse * inv_mass
                            velocity[0] += impulse[0] * inv_mass;
                            velocity[1] += impulse[1] * inv_mass;
                            velocity[2] += impulse[2] * inv_mass;
                        }
                    }
                }
            }
        }
    }
    
    /// Apply damping to reduce energy over time
    pub fn apply_damping(physics_data: &mut PhysicsData, linear_damping: f32, dt: f32) {
        let damping_factor = (1.0 - linear_damping).powf(dt);
        
        physics_data.velocities.par_iter_mut().for_each(|vel| {
            vel[0] *= damping_factor;
            vel[1] *= damping_factor;
            vel[2] *= damping_factor;
        });
    }
    
    /// Teleport an entity to a new position
    pub fn teleport(physics_data: &mut PhysicsData, entity: EntityId, position: [f32; 3]) {
        let idx = entity.index();
        if idx < physics_data.entity_count() {
            // Safely update position
            if let Some(pos) = physics_data.positions.get_mut(idx) {
                *pos = position;
            }
            
            // Clear velocity to prevent overshooting
            if let Some(vel) = physics_data.velocities.get_mut(idx) {
                *vel = [0.0, 0.0, 0.0];
            }
            
            // Update bounding box
            if let Some(bbox) = physics_data.bounding_boxes.get_mut(idx) {
                let half_extents = [0.5, 0.5, 0.5]; // Simplified
                *bbox = physics_tables::AABB::from_center_half_extents(
                    position,
                    half_extents,
                );
            }
        }
    }
    
    /// Set velocity directly
    pub fn set_velocity(physics_data: &mut PhysicsData, entity: EntityId, velocity: [f32; 3]) {
        let idx = entity.index();
        if idx < physics_data.entity_count() {
            // Safely set velocity
            if let Some(vel) = physics_data.velocities.get_mut(idx) {
                *vel = velocity;
            }
            
            // Wake up entity if it was sleeping
            if let Some(flag) = physics_data.flags.get_mut(idx) {
                flag.set_flag(physics_tables::PhysicsFlags::SLEEPING, false);
            }
        }
    }
    
    /// Get current interpolation alpha for rendering
    pub fn get_alpha(&self) -> f32 {
        self.alpha
    }
    
    /// Integrate physics with world collision detection
    pub fn integrate_with_world<W: WorldInterface>(&mut self, physics_data: &mut PhysicsData, world: &W, dt: f32) {
        // Clamp frame time to prevent spiral of death
        let frame_time = dt.min(0.25);
        
        self.accumulator += frame_time;
        
        // Fixed timestep integration
        while self.accumulator >= FIXED_TIMESTEP {
            // Save previous state for interpolation
            self.save_previous_state(physics_data);
            
            // Run physics step with collision detection
            self.physics_step_with_collision(physics_data, world, FIXED_TIMESTEP);
            
            self.accumulator -= FIXED_TIMESTEP;
        }
        
        // Calculate interpolation factor
        self.alpha = self.accumulator / FIXED_TIMESTEP;
    }
    
    /// Single physics step with improved collision detection
    fn physics_step_with_collision<W: WorldInterface>(&mut self, physics_data: &mut PhysicsData, world: &W, dt: f32) {
        let count = physics_data.entity_count();
        
        // Apply gravity and forces
        parallel::apply_gravity(
            &mut physics_data.velocities[..count],
            &physics_data.flags[..count],
            GRAVITY,
            dt,
        );
        
        // Update positions with collision detection
        for i in 0..count {
            if let (Some(pos), Some(vel), Some(flag), Some(bbox)) = (
                physics_data.positions.get_mut(i),
                physics_data.velocities.get_mut(i),
                physics_data.flags.get(i),
                physics_data.bounding_boxes.get_mut(i),
            ) {
                if flag.is_active() && flag.is_dynamic() {
                    // Calculate intended movement
                    let delta = [vel[0] * dt, vel[1] * dt, vel[2] * dt];
                    
                    // Perform collision detection and resolution
                    let (new_pos, new_vel, collision_flags) = self.resolve_world_collision(
                        world, *pos, delta, *vel, [0.4, 0.9, 0.4] // Player-like AABB half extents
                    );
                    
                    // Update position and velocity
                    *pos = new_pos;
                    *vel = new_vel;
                    
                    // Update bounding box
                    *bbox = physics_tables::AABB::from_center_half_extents(new_pos, [0.4, 0.9, 0.4]);
                    
                    // Update flags based on collision
                    if let Some(physics_flag) = physics_data.flags.get_mut(i) {
                        physics_flag.set_flag(physics_tables::PhysicsFlags::SLEEPING, collision_flags.grounded);
                    }
                }
            }
        }
    }
    
    /// Improved collision resolution with sliding and multi-axis support
    fn resolve_world_collision<W: WorldInterface>(
        &self,
        world: &W,
        position: [f32; 3],
        delta: [f32; 3],
        velocity: [f32; 3],
        half_extents: [f32; 3],
    ) -> ([f32; 3], [f32; 3], CollisionFlags) {
        let mut resolved_pos = position;
        let mut resolved_vel = velocity;
        let mut collision_flags = CollisionFlags::default();
        
        // Resolve collision for each axis separately to enable sliding
        for axis in 0..3 {
            let mut axis_delta = [0.0; 3];
            axis_delta[axis] = delta[axis];
            
            let (new_pos, new_vel, hit) = self.resolve_axis_collision(
                world,
                resolved_pos,
                axis_delta,
                resolved_vel,
                half_extents,
                axis,
            );
            
            resolved_pos = new_pos;
            resolved_vel = new_vel;
            
            if hit {
                match axis {
                    0 => collision_flags.wall_collision_x = true,
                    1 => {
                        collision_flags.grounded = axis_delta[1] < 0.0;
                        collision_flags.ceiling_collision = axis_delta[1] > 0.0;
                    }
                    2 => collision_flags.wall_collision_z = true,
                    _ => {}
                }
            }
        }
        
        (resolved_pos, resolved_vel, collision_flags)
    }
    
    /// Resolve collision for a single axis
    fn resolve_axis_collision<W: WorldInterface>(
        &self,
        world: &W,
        position: [f32; 3],
        delta: [f32; 3],
        velocity: [f32; 3],
        half_extents: [f32; 3],
        axis: usize,
    ) -> ([f32; 3], [f32; 3], bool) {
        let mut resolved_pos = [
            position[0] + delta[0],
            position[1] + delta[1],
            position[2] + delta[2],
        ];
        let mut resolved_vel = velocity;
        let mut collision_detected = false;
        
        // Get overlapping blocks for this movement
        let blocks = self.get_overlapping_blocks(world, position, resolved_pos, half_extents);
        
        for block_pos in blocks {
            if self.is_solid_block(world, block_pos) {
                // Check if AABB intersects with block
                if self.aabb_intersects_block(resolved_pos, half_extents, block_pos) {
                    // Resolve collision by moving back to edge
                    let block_center = [block_pos[0] as f32 + 0.5, block_pos[1] as f32 + 0.5, block_pos[2] as f32 + 0.5];
                    let block_half_extents = [0.5; 3];
                    
                    match axis {
                        0 => { // X axis
                            if delta[0] > 0.0 {
                                // Moving right, place at left edge of block
                                resolved_pos[0] = block_center[0] - block_half_extents[0] - half_extents[0] - 0.001;
                            } else if delta[0] < 0.0 {
                                // Moving left, place at right edge of block
                                resolved_pos[0] = block_center[0] + block_half_extents[0] + half_extents[0] + 0.001;
                            }
                            resolved_vel[0] = 0.0;
                        }
                        1 => { // Y axis
                            if delta[1] > 0.0 {
                                // Moving up, place at bottom edge of block
                                resolved_pos[1] = block_center[1] - block_half_extents[1] - half_extents[1] - 0.001;
                            } else if delta[1] < 0.0 {
                                // Moving down, place at top edge of block
                                resolved_pos[1] = block_center[1] + block_half_extents[1] + half_extents[1] + 0.001;
                            }
                            resolved_vel[1] = 0.0;
                        }
                        2 => { // Z axis
                            if delta[2] > 0.0 {
                                // Moving forward, place at back edge of block
                                resolved_pos[2] = block_center[2] - block_half_extents[2] - half_extents[2] - 0.001;
                            } else if delta[2] < 0.0 {
                                // Moving backward, place at front edge of block
                                resolved_pos[2] = block_center[2] + block_half_extents[2] + half_extents[2] + 0.001;
                            }
                            resolved_vel[2] = 0.0;
                        }
                        _ => {}
                    }
                    
                    collision_detected = true;
                }
            }
        }
        
        (resolved_pos, resolved_vel, collision_detected)
    }
    
    /// Get blocks that might overlap with the movement
    fn get_overlapping_blocks<W: WorldInterface>(
        &self,
        world: &W,
        start_pos: [f32; 3],
        end_pos: [f32; 3],
        half_extents: [f32; 3],
    ) -> Vec<[i32; 3]> {
        let mut blocks = Vec::new();
        
        // Calculate expanded AABB that covers the entire movement
        let min_x = (start_pos[0] - half_extents[0]).min(end_pos[0] - half_extents[0]);
        let max_x = (start_pos[0] + half_extents[0]).max(end_pos[0] + half_extents[0]);
        let min_y = (start_pos[1] - half_extents[1]).min(end_pos[1] - half_extents[1]);
        let max_y = (start_pos[1] + half_extents[1]).max(end_pos[1] + half_extents[1]);
        let min_z = (start_pos[2] - half_extents[2]).min(end_pos[2] - half_extents[2]);
        let max_z = (start_pos[2] + half_extents[2]).max(end_pos[2] + half_extents[2]);
        
        // Get block coordinates
        let block_min_x = min_x.floor() as i32;
        let block_max_x = max_x.ceil() as i32;
        let block_min_y = min_y.floor() as i32;
        let block_max_y = max_y.ceil() as i32;
        let block_min_z = min_z.floor() as i32;
        let block_max_z = max_z.ceil() as i32;
        
        for x in block_min_x..=block_max_x {
            for y in block_min_y..=block_max_y {
                for z in block_min_z..=block_max_z {
                    blocks.push([x, y, z]);
                }
            }
        }
        
        blocks
    }
    
    /// Check if a block is solid
    fn is_solid_block<W: WorldInterface>(&self, world: &W, block_pos: [i32; 3]) -> bool {
        if let Some(block_id) = world.get_block_at(block_pos) {
            world.is_solid_block(block_id)
        } else {
            false // No block found, not solid
        }
    }
    
    /// Check if AABB intersects with a block
    fn aabb_intersects_block(&self, position: [f32; 3], half_extents: [f32; 3], block_pos: [i32; 3]) -> bool {
        let entity_min = [
            position[0] - half_extents[0],
            position[1] - half_extents[1],
            position[2] - half_extents[2],
        ];
        let entity_max = [
            position[0] + half_extents[0],
            position[1] + half_extents[1],
            position[2] + half_extents[2],
        ];
        
        let block_min = [block_pos[0] as f32, block_pos[1] as f32, block_pos[2] as f32];
        let block_max = [block_pos[0] as f32 + 1.0, block_pos[1] as f32 + 1.0, block_pos[2] as f32 + 1.0];
        
        // AABB intersection test
        entity_min[0] < block_max[0] && entity_max[0] > block_min[0] &&
        entity_min[1] < block_max[1] && entity_max[1] > block_min[1] &&
        entity_min[2] < block_max[2] && entity_max[2] > block_min[2]
    }
}

/// Collision flags for tracking different types of collisions
#[derive(Default, Debug, Clone)]
struct CollisionFlags {
    pub grounded: bool,
    pub ceiling_collision: bool,
    pub wall_collision_x: bool,
    pub wall_collision_z: bool,
}

/// World interface trait for collision detection
pub trait WorldInterface {
    fn get_block_at(&self, pos: [i32; 3]) -> Option<u32>;
    fn is_solid_block(&self, block_id: u32) -> bool;
}

/// Adapter to implement physics WorldInterface for existing WorldInterface
pub struct WorldAdapter<'a, W> {
    world: &'a W,
}

impl<'a, W> WorldAdapter<'a, W> {
    pub fn new(world: &'a W) -> Self {
        Self { world }
    }
}

impl<'a, W: crate::world::WorldInterface> WorldInterface for WorldAdapter<'a, W> {
    fn get_block_at(&self, pos: [i32; 3]) -> Option<u32> {
        let voxel_pos = crate::VoxelPos::new(pos[0], pos[1], pos[2]);
        let block_id = self.world.get_block(voxel_pos);
        Some(block_id.0.into()) // Convert BlockId u16 to u32
    }
    
    fn is_solid_block(&self, block_id: u32) -> bool {
        // Convert u32 back to BlockId and check if it's solid
        let block = crate::BlockId(block_id as u16);
        block != crate::BlockId::AIR
    }
}

/// Parallel integration utilities
pub mod parallel {
    use super::*;
    
    /// Integrate positions in parallel
    pub fn integrate_positions(
        positions: &mut [[f32; 3]],
        velocities: &[[f32; 3]],
        flags: &[physics_tables::PhysicsFlags],
        dt: f32,
    ) {
        positions.par_iter_mut()
            .zip(velocities.par_iter())
            .zip(flags.par_iter())
            .for_each(|((pos, vel), flag)| {
                if flag.is_active() && flag.is_dynamic() {
                    pos[0] += vel[0] * dt;
                    pos[1] += vel[1] * dt;
                    pos[2] += vel[2] * dt;
                }
            });
    }
    
    /// Apply gravity in parallel
    pub fn apply_gravity(
        velocities: &mut [[f32; 3]],
        flags: &[physics_tables::PhysicsFlags],
        gravity: f32,
        dt: f32,
    ) {
        velocities.par_iter_mut()
            .zip(flags.par_iter())
            .for_each(|(vel, flag)| {
                if flag.is_active() && flag.is_dynamic() && flag.has_gravity() {
                    vel[1] += gravity * dt;
                    
                    // Clamp to terminal velocity
                    if vel[1] < TERMINAL_VELOCITY {
                        vel[1] = TERMINAL_VELOCITY;
                    }
                }
            });
    }
    
    /// Update bounding boxes in parallel
    pub fn update_bounding_boxes(
        bounding_boxes: &mut [physics_tables::AABB],
        positions: &[[f32; 3]],
        half_extents: &[[f32; 3]],
    ) {
        bounding_boxes.par_iter_mut()
            .zip(positions.par_iter())
            .zip(half_extents.par_iter())
            .for_each(|((aabb, pos), extents)| {
                *aabb = physics_tables::AABB::from_center_half_extents(*pos, *extents);
            });
    }
}