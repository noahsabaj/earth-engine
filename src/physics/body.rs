use super::{AABB, Vec3, GRAVITY, TERMINAL_VELOCITY};
use cgmath::{Point3, Vector3, Zero};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovementState {
    Normal,
    Sprinting,
    Crouching,
    Swimming,
    Climbing,
}

pub trait PhysicsBody {
    fn get_position(&self) -> Point3<f32>;
    fn set_position(&mut self, pos: Point3<f32>);
    fn get_velocity(&self) -> Vec3;
    fn set_velocity(&mut self, vel: Vec3);
    fn get_aabb(&self) -> AABB;
    fn apply_force(&mut self, force: Vec3);
    fn is_grounded(&self) -> bool;
    fn set_grounded(&mut self, grounded: bool);
    fn get_mass(&self) -> f32;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Debug, Clone)]
pub struct RigidBody {
    pub position: Point3<f32>,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub half_extents: Vec3,
    pub mass: f32,
    pub grounded: bool,
    pub gravity_enabled: bool,
}

impl RigidBody {
    pub fn new(position: Point3<f32>, half_extents: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::zero(),
            acceleration: Vec3::zero(),
            half_extents,
            mass: 1.0,
            grounded: false,
            gravity_enabled: true,
        }
    }
    
    pub fn update(&mut self, dt: f32) {
        // Apply gravity if enabled and not grounded
        if self.gravity_enabled {
            self.acceleration.y = GRAVITY;
        }
        
        // Update velocity
        self.velocity += self.acceleration * dt;
        
        // Clamp to terminal velocity
        if self.velocity.y < TERMINAL_VELOCITY {
            self.velocity.y = TERMINAL_VELOCITY;
        }
        
        // Update position
        self.position += self.velocity * dt;
        
        // Reset acceleration (forces are applied each frame)
        self.acceleration = Vec3::zero();
    }
    
    pub fn apply_impulse(&mut self, impulse: Vec3) {
        self.velocity += impulse / self.mass;
    }
}

impl PhysicsBody for RigidBody {
    fn get_position(&self) -> Point3<f32> {
        self.position
    }
    
    fn set_position(&mut self, pos: Point3<f32>) {
        self.position = pos;
    }
    
    fn get_velocity(&self) -> Vec3 {
        self.velocity
    }
    
    fn set_velocity(&mut self, vel: Vec3) {
        self.velocity = vel;
    }
    
    fn get_aabb(&self) -> AABB {
        AABB::from_center_half_extents(self.position, self.half_extents)
    }
    
    fn apply_force(&mut self, force: Vec3) {
        self.acceleration += force / self.mass;
    }
    
    fn is_grounded(&self) -> bool {
        self.grounded
    }
    
    fn set_grounded(&mut self, grounded: bool) {
        self.grounded = grounded;
    }
    
    fn get_mass(&self) -> f32 {
        self.mass
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Player-specific physics body
pub struct PlayerBody {
    pub rigid_body: RigidBody,
    pub movement_state: MovementState,
    pub jump_force: f32,
    pub walk_speed: f32,
    pub sprint_speed: f32,
    pub swim_speed: f32,
    pub climb_speed: f32,
    pub standing_height: f32,
    pub crouching_height: f32,
    pub radius: f32,
    pub fall_start_y: Option<f32>,
    pub is_in_water: bool,
    pub is_on_ladder: bool,
}

impl PlayerBody {
    pub fn new(position: Point3<f32>) -> Self {
        // Player is roughly 1.8m tall standing, 0.9m crouching, 0.3m radius
        let half_extents = Vector3::new(0.3, 0.9, 0.3);
        let mut rigid_body = RigidBody::new(position, half_extents);
        rigid_body.mass = 70.0; // 70kg player
        
        Self {
            rigid_body,
            movement_state: MovementState::Normal,
            jump_force: 8.0,      // Jump velocity
            walk_speed: 4.5,      // Walking speed m/s
            sprint_speed: 7.0,    // Sprint speed m/s
            swim_speed: 2.0,      // Swimming speed m/s
            climb_speed: 3.0,     // Ladder climbing speed m/s
            standing_height: 1.8,
            crouching_height: 0.9,
            radius: 0.3,
            fall_start_y: None,
            is_in_water: false,
            is_on_ladder: false,
        }
    }
    
    pub fn jump(&mut self) {
        match self.movement_state {
            MovementState::Swimming => {
                // Swimming upward
                self.rigid_body.velocity.y = self.swim_speed;
            }
            MovementState::Climbing => {
                // Can't jump while on ladder
            }
            MovementState::Crouching => {
                // Can't jump while crouching
            }
            _ => {
                if self.rigid_body.grounded {
                    self.rigid_body.velocity.y = self.jump_force;
                    self.rigid_body.grounded = false;
                }
            }
        }
    }
    
    pub fn move_horizontal(&mut self, direction: Vec3) {
        let speed = match self.movement_state {
            MovementState::Sprinting => {
                if self.rigid_body.grounded {
                    self.sprint_speed
                } else {
                    self.sprint_speed * 0.3 // Reduced air control
                }
            }
            MovementState::Crouching => {
                if self.rigid_body.grounded {
                    self.walk_speed * 0.3 // Slow while crouching
                } else {
                    self.walk_speed * 0.1
                }
            }
            MovementState::Swimming => self.swim_speed,
            MovementState::Climbing => 0.0, // No horizontal movement on ladders
            MovementState::Normal => {
                if self.rigid_body.grounded {
                    self.walk_speed
                } else {
                    self.walk_speed * 0.3 // Reduced air control
                }
            }
        };
        
        self.rigid_body.velocity.x = direction.x * speed;
        self.rigid_body.velocity.z = direction.z * speed;
    }
    
    pub fn move_vertical_on_ladder(&mut self, up: bool) {
        if self.movement_state == MovementState::Climbing {
            self.rigid_body.velocity.y = if up { self.climb_speed } else { -self.climb_speed };
        }
    }
    
    pub fn set_movement_state(&mut self, state: MovementState) {
        if self.movement_state == state {
            return;
        }
        
        self.movement_state = state;
        
        // Update hitbox based on state
        match state {
            MovementState::Crouching => {
                let crouch_half_height = self.crouching_height / 2.0;
                self.rigid_body.half_extents.y = crouch_half_height;
                // Move down to keep feet at same position
                self.rigid_body.position.y -= (self.standing_height - self.crouching_height) / 2.0;
            }
            _ => {
                // Return to standing height
                if self.rigid_body.half_extents.y < self.standing_height / 2.0 {
                    self.rigid_body.half_extents.y = self.standing_height / 2.0;
                    // Move up to keep feet at same position
                    self.rigid_body.position.y += (self.standing_height - self.crouching_height) / 2.0;
                }
            }
        }
    }
    
    pub fn update_fall_damage(&mut self) {
        // Track fall start position
        if self.rigid_body.grounded || self.is_in_water || self.is_on_ladder {
            self.fall_start_y = None;
        } else if self.fall_start_y.is_none() && self.rigid_body.velocity.y < 0.0 {
            self.fall_start_y = Some(self.rigid_body.position.y);
        }
    }
    
    pub fn calculate_fall_damage(&self) -> f32 {
        if let Some(start_y) = self.fall_start_y {
            let fall_distance = start_y - self.rigid_body.position.y;
            if fall_distance > 3.0 { // 3 blocks safe fall
                // 1 damage per block after 3
                (fall_distance - 3.0) * 10.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

impl PhysicsBody for PlayerBody {
    fn get_position(&self) -> Point3<f32> {
        self.rigid_body.position
    }
    
    fn set_position(&mut self, pos: Point3<f32>) {
        self.rigid_body.position = pos;
    }
    
    fn get_velocity(&self) -> Vec3 {
        self.rigid_body.velocity
    }
    
    fn set_velocity(&mut self, vel: Vec3) {
        self.rigid_body.velocity = vel;
    }
    
    fn get_aabb(&self) -> AABB {
        AABB::from_center_half_extents(self.rigid_body.position, self.rigid_body.half_extents)
    }
    
    fn apply_force(&mut self, force: Vec3) {
        self.rigid_body.apply_force(force);
    }
    
    fn is_grounded(&self) -> bool {
        self.rigid_body.grounded
    }
    
    fn set_grounded(&mut self, grounded: bool) {
        self.rigid_body.grounded = grounded;
    }
    
    fn get_mass(&self) -> f32 {
        self.rigid_body.mass
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}