/// Data-Oriented Physics System
/// 
/// Sprint 35: Zero-allocation physics with pre-allocated buffers.
/// All physics data stored in contiguous arrays for cache efficiency.

use super::{AABB, FIXED_TIMESTEP, GRAVITY, TERMINAL_VELOCITY};
use crate::{World, VoxelPos, BlockId};
use cgmath::{Point3, Vector3, Zero, InnerSpace};
use bytemuck::{Pod, Zeroable};
use std::any::Any;

/// Maximum entities in physics system
pub const MAX_PHYSICS_ENTITIES: usize = 16384;
/// Maximum blocks to check per collision
pub const MAX_COLLISION_BLOCKS: usize = 64;

/// Entity ID type
pub type EntityId = u32;

/// Physics body data (SoA layout)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PhysicsBodyData {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub flags: u32, // Bit 0: active, Bit 1: grounded, Bit 2: in_water, Bit 3: on_ladder
}

/// Physics system flags
pub mod flags {
    pub const ACTIVE: u32 = 1 << 0;
    pub const GROUNDED: u32 = 1 << 1;
    pub const IN_WATER: u32 = 1 << 2;
    pub const ON_LADDER: u32 = 1 << 3;
    pub const STATIC: u32 = 1 << 4;
}

/// Physics update data (for batch processing)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PhysicsUpdate {
    pub entity_index: u32,
    pub delta_position: [f32; 3],
    pub new_velocity: [f32; 3],
    pub new_flags: u32,
}

/// Pre-allocated collision block buffer
pub struct CollisionBlockBuffer {
    pub blocks: [VoxelPos; MAX_COLLISION_BLOCKS],
    pub count: usize,
}

impl Default for CollisionBlockBuffer {
    fn default() -> Self {
        Self {
            blocks: [VoxelPos::new(0, 0, 0); MAX_COLLISION_BLOCKS],
            count: 0,
        }
    }
}

/// Data-oriented physics world
pub struct PhysicsWorldData {
    /// Entity data arrays (SoA)
    pub bodies: Vec<PhysicsBodyData>,
    
    /// Active entity count
    pub active_count: usize,
    
    /// Entity ID to index mapping
    pub id_to_index: rustc_hash::FxHashMap<EntityId, usize>,
    
    /// Pre-allocated update buffer
    pub update_buffer: Vec<PhysicsUpdate>,
    
    /// Pre-allocated collision block buffer
    pub collision_buffer: CollisionBlockBuffer,
    
    /// Physics accumulator
    pub accumulator: f32,
    
    /// Next entity ID
    pub next_entity_id: EntityId,
}

impl PhysicsWorldData {
    pub fn new() -> Self {
        let mut bodies = Vec::with_capacity(MAX_PHYSICS_ENTITIES);
        bodies.resize(MAX_PHYSICS_ENTITIES, PhysicsBodyData {
            position: [0.0; 3],
            velocity: [0.0; 3],
            aabb_min: [-0.5; 3],
            aabb_max: [0.5; 3],
            mass: 1.0,
            friction: 0.8,
            restitution: 0.0,
            flags: 0,
        });
        
        let mut update_buffer = Vec::with_capacity(MAX_PHYSICS_ENTITIES);
        update_buffer.resize(MAX_PHYSICS_ENTITIES, PhysicsUpdate {
            entity_index: 0,
            delta_position: [0.0; 3],
            new_velocity: [0.0; 3],
            new_flags: 0,
        });
        
        Self {
            bodies,
            active_count: 0,
            id_to_index: rustc_hash::FxHashMap::with_capacity_and_hasher(
                MAX_PHYSICS_ENTITIES,
                Default::default()
            ),
            update_buffer,
            collision_buffer: CollisionBlockBuffer::default(),
            accumulator: 0.0,
            next_entity_id: 1,
        }
    }
}

/// Physics operations - pure functions
pub mod operations {
    use super::*;
    
    /// Add entity to physics world
    pub fn add_entity(
        data: &mut PhysicsWorldData,
        position: Point3<f32>,
        velocity: Vector3<f32>,
        aabb_size: Vector3<f32>,
    ) -> Option<EntityId> {
        if data.active_count >= MAX_PHYSICS_ENTITIES {
            return None;
        }
        
        let id = data.next_entity_id;
        data.next_entity_id += 1;
        
        let idx = data.active_count;
        data.active_count += 1;
        
        // Initialize body data
        let half_size = aabb_size * 0.5;
        if let Some(body) = data.bodies.get_mut(idx) {
            *body = PhysicsBodyData {
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            aabb_min: [-half_size.x, -half_size.y, -half_size.z],
            aabb_max: [half_size.x, half_size.y, half_size.z],
            mass: 1.0,
            friction: 0.8,
            restitution: 0.0,
                flags: flags::ACTIVE,
            };
        } else {
            return None;
        }
        
        data.id_to_index.insert(id, idx);
        Some(id)
    }
    
    /// Remove entity from physics world
    pub fn remove_entity(data: &mut PhysicsWorldData, id: EntityId) {
        if let Some(idx) = data.id_to_index.remove(&id) {
            // Swap with last active entity
            if idx < data.active_count - 1 {
                data.bodies.swap(idx, data.active_count - 1);
                
                // Update mapping for swapped entity
                for (eid, eidx) in data.id_to_index.iter_mut() {
                    if *eidx == data.active_count - 1 {
                        *eidx = idx;
                        break;
                    }
                }
            }
            
            data.active_count -= 1;
        }
    }
    
    /// Update physics world with fixed timestep
    pub fn update(data: &mut PhysicsWorldData, world: &World, delta_time: f32) {
        data.accumulator += delta_time;
        
        while data.accumulator >= FIXED_TIMESTEP {
            step(data, world, FIXED_TIMESTEP);
            data.accumulator -= FIXED_TIMESTEP;
        }
    }
    
    /// Single physics step
    fn step(data: &mut PhysicsWorldData, world: &World, dt: f32) {
        let mut update_count = 0;
        
        // Phase 1: Calculate updates
        for i in 0..data.active_count {
            let body = match data.bodies.get_mut(i) {
                Some(b) => b,
                None => continue,
            };
            
            if body.flags & flags::ACTIVE == 0 {
                continue;
            }
            
            let mut velocity = Vector3::new(body.velocity[0], body.velocity[1], body.velocity[2]);
            
            // Apply gravity if not grounded
            if body.flags & flags::GROUNDED == 0 {
                velocity.y += GRAVITY * dt;
                if velocity.y < TERMINAL_VELOCITY {
                    velocity.y = TERMINAL_VELOCITY;
                }
            }
            
            // Calculate delta position
            let delta = velocity * dt;
            
            // Store update
            if let Some(update) = data.update_buffer.get_mut(update_count) {
                *update = PhysicsUpdate {
                entity_index: i as u32,
                delta_position: [delta.x, delta.y, delta.z],
                new_velocity: [velocity.x, velocity.y, velocity.z],
                    new_flags: body.flags,
                };
            }
            update_count += 1;
        }
        
        // Phase 2: Apply updates with collision detection
        for i in 0..update_count {
            let update = match data.update_buffer.get(i) {
                Some(u) => u,
                None => continue,
            };
            let idx = update.entity_index as usize;
            let body = match data.bodies.get(idx) {
                Some(b) => b,
                None => continue,
            };
            
            // Get overlapping blocks (reuses collision buffer)
            get_overlapping_blocks(
                &mut data.collision_buffer,
                world,
                body.position,
                body.aabb_min,
                body.aabb_max,
                update.delta_position,
            );
            
            // Resolve collisions
            let (resolved_pos, resolved_vel, grounded) = resolve_collisions(
                world,
                &data.collision_buffer,
                body.position,
                update.delta_position,
                update.new_velocity,
            );
            
            // Apply resolved values
            let body = match data.bodies.get_mut(idx) {
                Some(b) => b,
                None => continue,
            };
            body.position = resolved_pos;
            body.velocity = resolved_vel;
            
            if grounded {
                body.flags |= flags::GROUNDED;
            } else {
                body.flags &= !flags::GROUNDED;
            }
        }
    }
    
    /// Get overlapping blocks (fills pre-allocated buffer)
    fn get_overlapping_blocks(
        buffer: &mut CollisionBlockBuffer,
        world: &World,
        position: [f32; 3],
        aabb_min: [f32; 3],
        aabb_max: [f32; 3],
        delta: [f32; 3],
    ) {
        buffer.count = 0;
        
        // Calculate bounds
        let min_x = (position[0] + aabb_min[0] + delta[0].min(0.0)).floor() as i32;
        let min_y = (position[1] + aabb_min[1] + delta[1].min(0.0)).floor() as i32;
        let min_z = (position[2] + aabb_min[2] + delta[2].min(0.0)).floor() as i32;
        let max_x = (position[0] + aabb_max[0] + delta[0].max(0.0)).ceil() as i32;
        let max_y = (position[1] + aabb_max[1] + delta[1].max(0.0)).ceil() as i32;
        let max_z = (position[2] + aabb_max[2] + delta[2].max(0.0)).ceil() as i32;
        
        // Fill buffer with overlapping blocks
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                for z in min_z..=max_z {
                    if buffer.count >= MAX_COLLISION_BLOCKS {
                        return;
                    }
                    
                    let pos = VoxelPos::new(x, y, z);
                    if world.is_block_in_bounds(pos) {
                        if let Some(block_slot) = buffer.blocks.get_mut(buffer.count) {
                            *block_slot = pos;
                            buffer.count += 1;
                        }
                    }
                }
            }
        }
    }
    
    /// Resolve collisions (simplified)
    fn resolve_collisions(
        world: &World,
        collision_buffer: &CollisionBlockBuffer,
        position: [f32; 3],
        delta: [f32; 3],
        velocity: [f32; 3],
    ) -> ([f32; 3], [f32; 3], bool) {
        let mut resolved_pos = [
            position[0] + delta[0],
            position[1] + delta[1],
            position[2] + delta[2],
        ];
        let mut resolved_vel = velocity;
        let mut grounded = false;
        
        // Check collisions with blocks
        for i in 0..collision_buffer.count {
            let block_pos = match collision_buffer.blocks.get(i) {
                Some(pos) => *pos,
                None => continue,
            };
            let block_id = world.get_block(block_pos);
            
            if block_id != BlockId::AIR {
                // Simple collision response
                // (In real implementation, this would be more sophisticated)
                if delta[1] < 0.0 && position[1] > block_pos.y as f32 {
                    resolved_pos[1] = block_pos.y as f32 + 1.0;
                    resolved_vel[1] = 0.0;
                    grounded = true;
                }
            }
        }
        
        (resolved_pos, resolved_vel, grounded)
    }
    
    /// Get interpolated position for rendering
    pub fn get_interpolated_position(
        data: &PhysicsWorldData,
        id: EntityId,
        alpha: f32,
    ) -> Option<Point3<f32>> {
        data.id_to_index.get(&id).and_then(|&idx| {
            data.bodies.get(idx).map(|body| {
                Point3::new(
                    body.position[0] + body.velocity[0] * (alpha * FIXED_TIMESTEP),
                    body.position[1] + body.velocity[1] * (alpha * FIXED_TIMESTEP),
                    body.position[2] + body.velocity[2] * (alpha * FIXED_TIMESTEP),
                )
            })
        })
    }
}

// Convenience methods for easier migration
impl PhysicsWorldData {
    /// Add entity to physics world
    pub fn add_entity(
        &mut self,
        position: Point3<f32>,
        velocity: Vector3<f32>,
        aabb_size: Vector3<f32>,
        mass: f32,
        friction: f32,
        restitution: f32,
    ) -> EntityId {
        let id = operations::add_entity(self, position, velocity, aabb_size).expect("Failed to add entity");
        
        // Set additional properties
        if let Some(&idx) = self.id_to_index.get(&id) {
            if let Some(body) = self.bodies.get_mut(idx) {
                body.mass = mass;
                body.friction = friction;
                body.restitution = restitution;
            }
        }
        
        id
    }
    
    /// Update physics world
    pub fn update<W: crate::world::WorldInterface + 'static>(&mut self, world: &W, delta_time: f32) {
        // Downcast to concrete World type for now
        // In a real implementation, we'd make operations work with WorldInterface
        if let Some(world) = (world as &dyn std::any::Any).downcast_ref::<crate::world::World>() {
            operations::update(self, world, delta_time);
        }
    }
    
    /// Get body data by entity ID
    pub fn get_body(&self, id: EntityId) -> Option<&PhysicsBodyData> {
        self.id_to_index.get(&id).and_then(|&idx| self.bodies.get(idx))
    }
    
    /// Get mutable body data by entity ID
    pub fn get_body_mut(&mut self, id: EntityId) -> Option<&mut PhysicsBodyData> {
        self.id_to_index.get(&id).and_then(|&idx| self.bodies.get_mut(idx))
    }
    
    /// Set entity velocity
    pub fn set_velocity(&mut self, id: EntityId, velocity: Vector3<f32>) {
        if let Some(body) = self.get_body_mut(id) {
            body.velocity = [velocity.x, velocity.y, velocity.z];
        }
    }
    
    /// Get entity position
    pub fn get_position(&self, id: EntityId) -> Option<Point3<f32>> {
        self.get_body(id).map(|body| {
            Point3::new(body.position[0], body.position[1], body.position[2])
        })
    }
    
    /// Set entity position
    pub fn set_position(&mut self, id: EntityId, position: Point3<f32>) {
        if let Some(body) = self.get_body_mut(id) {
            body.position = [position.x, position.y, position.z];
        }
    }
}

// Usage:
// let mut physics = PhysicsWorldData::new();
// let id = physics.add_entity(Point3::new(0.0, 10.0, 0.0), Vector3::zero(), Vector3::new(1.0, 2.0, 1.0), 80.0, 0.8, 0.0);
// physics.update(&world, delta_time);