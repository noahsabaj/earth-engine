/// Zero-allocation physics world implementation
/// Uses pre-allocated buffers and object pools to eliminate allocations in the physics update loop

use super::{AABB, Vec3, PhysicsBody, FIXED_TIMESTEP, MovementState};
use crate::{World, VoxelPos, BlockId};
use cgmath::{Point3, Vector3, Zero, InnerSpace};
use std::collections::HashMap;

pub type EntityId = u32;

/// Pre-allocated buffers for physics calculations
struct PhysicsBuffers {
    /// Buffer for physics updates
    updates: Vec<(EntityId, AABB, Point3<f32>, Vec3, Vec3)>,
    /// Buffer for overlapping blocks
    overlapping_blocks: Vec<VoxelPos>,
    /// Buffer for solid block positions
    solid_blocks: Vec<VoxelPos>,
}

impl PhysicsBuffers {
    fn new(capacity: usize) -> Self {
        Self {
            updates: Vec::with_capacity(capacity),
            overlapping_blocks: Vec::with_capacity(64), // Max blocks to check
            solid_blocks: Vec::with_capacity(32),
        }
    }
    
    fn clear(&mut self) {
        self.updates.clear();
        self.overlapping_blocks.clear();
        self.solid_blocks.clear();
    }
}

/// Optimized physics world with zero allocations per frame
pub struct OptimizedPhysicsWorld {
    bodies: HashMap<EntityId, Box<dyn PhysicsBody + Send + Sync>>,
    next_entity_id: EntityId,
    accumulator: f32,
    /// Pre-allocated buffers for physics calculations
    buffers: PhysicsBuffers,
}

impl OptimizedPhysicsWorld {
    pub fn new() -> Self {
        Self {
            bodies: HashMap::new(),
            next_entity_id: 1,
            accumulator: 0.0,
            buffers: PhysicsBuffers::new(128), // Support up to 128 entities
        }
    }
    
    pub fn add_body(&mut self, body: Box<dyn PhysicsBody + Send + Sync>) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.bodies.insert(id, body);
        id
    }
    
    pub fn remove_body(&mut self, id: EntityId) -> Option<Box<dyn PhysicsBody + Send + Sync>> {
        self.bodies.remove(&id)
    }
    
    pub fn get_body(&self, id: EntityId) -> Option<&(dyn PhysicsBody + Send + Sync)> {
        self.bodies.get(&id).map(|b| b.as_ref())
    }
    
    pub fn get_body_mut(&mut self, id: EntityId) -> Option<&mut Box<dyn PhysicsBody + Send + Sync>> {
        self.bodies.get_mut(&id)
    }
    
    // Fixed timestep update
    pub fn update(&mut self, world: &World, delta_time: f32) {
        self.accumulator += delta_time;
        
        // Run physics at fixed timestep
        while self.accumulator >= FIXED_TIMESTEP {
            self.step(world, FIXED_TIMESTEP);
            self.accumulator -= FIXED_TIMESTEP;
        }
    }
    
    fn step(&mut self, world: &World, dt: f32) {
        // Clear buffers without deallocating
        self.buffers.clear();
        
        // Collect updates in pre-allocated buffer
        for (id, body) in self.bodies.iter_mut() {
            // Get current state
            let pos = body.get_position();
            let vel = body.get_velocity();
            let aabb = body.get_aabb();
            
            // Apply gravity and integrate velocity
            if !body.is_grounded() {
                let mut new_vel = vel;
                new_vel.y += super::GRAVITY * dt;
                
                // Terminal velocity
                if new_vel.y < super::TERMINAL_VELOCITY {
                    new_vel.y = super::TERMINAL_VELOCITY;
                }
                
                body.set_velocity(new_vel);
            }
            
            // Calculate new position
            let velocity = body.get_velocity();
            let delta = velocity * dt;
            
            // Store update in pre-allocated buffer
            self.buffers.updates.push((*id, aabb, pos, delta, velocity));
        }
        
        // Apply collision detection and updates
        for i in 0..self.buffers.updates.len() {
            let (id, aabb, pos, delta, velocity) = self.buffers.updates[i];
            
            // Collision detection and response
            let (resolved_pos, resolved_vel, grounded, in_water, on_ladder) = self.resolve_collisions(
                world,
                id,
                aabb,
                pos,
                delta,
                velocity
            );
            
            // Update body
            if let Some(body) = self.bodies.get_mut(&id) {
                body.set_position(resolved_pos);
                body.set_velocity(resolved_vel);
                body.set_grounded(grounded);
                
                // Update player-specific states
                if let Some(player_body) = body.as_any_mut().downcast_mut::<crate::physics::body::PlayerBody>() {
                    player_body.is_in_water = in_water;
                    player_body.is_on_ladder = on_ladder;
                    
                    // Update movement state based on environment
                    if in_water && player_body.movement_state != MovementState::Swimming {
                        player_body.set_movement_state(MovementState::Swimming);
                    } else if on_ladder && player_body.movement_state != MovementState::Climbing {
                        player_body.set_movement_state(MovementState::Climbing);
                    } else if !in_water && !on_ladder && 
                             (player_body.movement_state == MovementState::Swimming ||
                              player_body.movement_state == MovementState::Climbing) {
                        player_body.set_movement_state(MovementState::Normal);
                    }
                    
                    // Apply water physics
                    if in_water {
                        let mut vel = player_body.get_velocity();
                        // Buoyancy and water resistance
                        vel.y *= 0.95; // Water resistance
                        if vel.y < -2.0 {
                            vel.y = -2.0; // Slower fall in water
                        }
                        player_body.set_velocity(vel);
                        player_body.rigid_body.gravity_enabled = false;
                    } else {
                        player_body.rigid_body.gravity_enabled = true;
                    }
                    
                    // Apply ladder physics
                    if on_ladder {
                        player_body.rigid_body.gravity_enabled = false;
                        // Stop horizontal movement on ladders
                        let mut vel = player_body.get_velocity();
                        vel.x = 0.0;
                        vel.z = 0.0;
                        // Slow vertical movement if not actively climbing
                        if vel.y.abs() < 0.1 {
                            vel.y = 0.0;
                        }
                        player_body.set_velocity(vel);
                    }
                    
                    // Update fall damage tracking
                    player_body.update_fall_damage();
                }
            }
        }
    }
    
    fn resolve_collisions(
        &mut self,
        world: &World,
        _body_id: EntityId,
        aabb: AABB,
        position: Point3<f32>,
        delta: Vec3,
        velocity: Vec3,
    ) -> (Point3<f32>, Vec3, bool, bool, bool) {
        let mut new_pos = position;
        let mut new_vel = velocity;
        let mut grounded = false;
        let mut in_water = false;
        let mut on_ladder = false;
        
        // Try step-up first if moving horizontally and hitting something
        let horizontal_delta = Vector3::new(delta.x, 0.0, delta.z);
        if horizontal_delta.magnitude() > 0.001 {
            // Check if we can step up
            let step_height = 0.55; // Slightly more than half a block
            let raised_pos = position + Vector3::new(0.0, step_height, 0.0);
            let raised_aabb = aabb.translated(raised_pos - position);
            
            // Test horizontal movement at raised position
            let test_raised = raised_aabb.translated(horizontal_delta);
            self.get_overlapping_blocks_buffered(world, test_raised);
            
            let mut can_step_up = true;
            for block_pos in &self.buffers.overlapping_blocks {
                if self.is_solid_block(world, *block_pos) {
                    can_step_up = false;
                    break;
                }
            }
            
            if can_step_up {
                // Check if there's ground to step onto
                let step_test_aabb = test_raised.translated(Vector3::new(0.0, -step_height, 0.0));
                
                // Save current overlapping blocks
                let saved_len = self.buffers.overlapping_blocks.len();
                self.get_overlapping_blocks_buffered(world, step_test_aabb);
                
                let mut found_ground = false;
                for i in saved_len..self.buffers.overlapping_blocks.len() {
                    let block_pos = self.buffers.overlapping_blocks[i];
                    if self.is_solid_block(world, block_pos) {
                        // Calculate exact step height
                        let block_top = block_pos.y as f32 + 1.0;
                        let current_bottom = position.y - aabb.half_extents().y;
                        let step_up_amount = block_top - current_bottom;
                        
                        if step_up_amount > 0.0 && step_up_amount <= step_height {
                            new_pos.x += horizontal_delta.x;
                            new_pos.y += step_up_amount;
                            new_pos.z += horizontal_delta.z;
                            found_ground = true;
                            grounded = true;
                            break;
                        }
                    }
                }
                
                // Restore overlapping blocks buffer
                self.buffers.overlapping_blocks.truncate(saved_len);
                
                if found_ground {
                    return (new_pos, new_vel, grounded, in_water, on_ladder);
                }
            }
        }
        
        // Normal collision resolution
        for axis in 0..3 {
            let mut axis_delta = Vector3::zero();
            match axis {
                0 => axis_delta.x = delta.x,
                1 => axis_delta.y = delta.y,
                2 => axis_delta.z = delta.z,
                _ => unreachable!(),
            }
            
            if axis_delta.magnitude() < 0.0001 {
                continue;
            }
            
            // Test movement along this axis
            let test_aabb = aabb.translated(new_pos - position + axis_delta);
            
            // Check collision with blocks using buffered function
            self.get_overlapping_blocks_buffered(world, test_aabb);
            
            let mut collision = false;
            for block_pos in &self.buffers.overlapping_blocks {
                let block_id = world.get_block(*block_pos);
                
                // Check for special blocks
                if block_id == BlockId(6) { // Water
                    in_water = true;
                } else if block_id == BlockId(7) { // Ladder
                    on_ladder = true;
                }
                
                if self.is_solid_block_id(block_id) {
                    // Get block AABB
                    let block_aabb = AABB::new(
                        Point3::new(
                            block_pos.x as f32,
                            block_pos.y as f32,
                            block_pos.z as f32,
                        ),
                        Point3::new(
                            block_pos.x as f32 + 1.0,
                            block_pos.y as f32 + 1.0,
                            block_pos.z as f32 + 1.0,
                        ),
                    );
                    
                    if test_aabb.intersects(&block_aabb) {
                        collision = true;
                        
                        // Stop velocity on this axis
                        match axis {
                            0 => new_vel.x = 0.0,
                            1 => {
                                new_vel.y = 0.0;
                                // Check if we hit ground
                                if delta.y < 0.0 {
                                    grounded = true;
                                }
                            },
                            2 => new_vel.z = 0.0,
                            _ => unreachable!(),
                        }
                        
                        break;
                    }
                }
            }
            
            // Apply movement if no collision
            if !collision {
                new_pos += axis_delta;
            }
            
            // Clear buffer for next axis
            self.buffers.overlapping_blocks.clear();
        }
        
        (new_pos, new_vel, grounded, in_water, on_ladder)
    }
    
    fn is_solid_block(&self, world: &World, pos: VoxelPos) -> bool {
        let block_id = world.get_block(pos);
        self.is_solid_block_id(block_id)
    }
    
    fn is_solid_block_id(&self, block_id: BlockId) -> bool {
        // Air, water, and ladder are not solid
        block_id != BlockId::AIR && block_id != BlockId(6) && block_id != BlockId(7)
    }
    
    /// Get overlapping blocks using pre-allocated buffer
    fn get_overlapping_blocks_buffered(&mut self, world: &World, aabb: AABB) {
        self.buffers.overlapping_blocks.clear();
        
        // Convert AABB to block coordinates
        let min_x = aabb.min.x.floor() as i32;
        let min_y = aabb.min.y.floor() as i32;
        let min_z = aabb.min.z.floor() as i32;
        let max_x = aabb.max.x.ceil() as i32;
        let max_y = aabb.max.y.ceil() as i32;
        let max_z = aabb.max.z.ceil() as i32;
        
        // Check all blocks in range
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                for z in min_z..=max_z {
                    let pos = VoxelPos::new(x, y, z);
                    if world.is_block_in_bounds(pos) {
                        self.buffers.overlapping_blocks.push(pos);
                    }
                }
            }
        }
    }
    
    // Get interpolated position for rendering (between physics steps)
    pub fn get_interpolated_position(&self, id: EntityId, alpha: f32) -> Option<Point3<f32>> {
        self.bodies.get(&id).map(|body| {
            let pos = body.get_position();
            let vel = body.get_velocity();
            pos + vel * (alpha * FIXED_TIMESTEP)
        })
    }
}