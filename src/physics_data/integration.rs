use super::{PhysicsData, EntityId, FIXED_TIMESTEP};
use rayon::prelude::*;

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
    pub fn get_interpolated_position(&self, entity: EntityId) -> Option<[f32; 3]> {
        let idx = entity.index();
        if idx >= self.previous_positions.len() {
            return None;
        }
        
        let prev = self.previous_positions[idx];
        let curr = self.previous_positions.get(idx)?;
        
        Some([
            prev[0] + (curr[0] - prev[0]) * self.alpha,
            prev[1] + (curr[1] - prev[1]) * self.alpha,
            prev[2] + (curr[2] - prev[2]) * self.alpha,
        ])
    }
    
    /// Apply forces to entities
    pub fn apply_forces(physics_data: &mut PhysicsData, forces: &[[f32; 3]], dt: f32) {
        let count = physics_data.entity_count().min(forces.len());
        
        (0..count).into_par_iter().for_each(|i| {
            if physics_data.flags[i].is_dynamic() {
                let inv_mass = physics_data.inverse_masses[i];
                
                // F = ma, so a = F/m = F * inv_mass
                physics_data.velocities[i][0] += forces[i][0] * inv_mass * dt;
                physics_data.velocities[i][1] += forces[i][1] * inv_mass * dt;
                physics_data.velocities[i][2] += forces[i][2] * inv_mass * dt;
            }
        });
    }
    
    /// Apply impulses to entities
    pub fn apply_impulses(physics_data: &mut PhysicsData, impulses: &[(EntityId, [f32; 3])]) {
        impulses.par_iter().for_each(|(entity, impulse)| {
            let idx = entity.index();
            if idx < physics_data.entity_count() && physics_data.flags[idx].is_dynamic() {
                let inv_mass = physics_data.inverse_masses[idx];
                
                // Impulse = change in momentum = m * Δv
                // So Δv = impulse / m = impulse * inv_mass
                unsafe {
                    let vel = &mut *(&mut physics_data.velocities[idx] as *mut [f32; 3]);
                    vel[0] += impulse[0] * inv_mass;
                    vel[1] += impulse[1] * inv_mass;
                    vel[2] += impulse[2] * inv_mass;
                }
            }
        });
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
            physics_data.positions[idx] = position;
            // Clear velocity to prevent overshooting
            physics_data.velocities[idx] = [0.0, 0.0, 0.0];
            // Update bounding box
            let half_extents = [0.5, 0.5, 0.5]; // Simplified
            physics_data.bounding_boxes[idx] = super::physics_tables::AABB::from_center_half_extents(
                position,
                half_extents,
            );
        }
    }
    
    /// Set velocity directly
    pub fn set_velocity(physics_data: &mut PhysicsData, entity: EntityId, velocity: [f32; 3]) {
        let idx = entity.index();
        if idx < physics_data.entity_count() {
            physics_data.velocities[idx] = velocity;
            // Wake up entity if it was sleeping
            physics_data.flags[idx].set_flag(super::physics_tables::PhysicsFlags::SLEEPING, false);
        }
    }
    
    /// Get current interpolation alpha for rendering
    pub fn get_alpha(&self) -> f32 {
        self.alpha
    }
}

/// Parallel integration utilities
pub mod parallel {
    use super::*;
    
    /// Integrate positions in parallel
    pub fn integrate_positions(
        positions: &mut [[f32; 3]],
        velocities: &[[f32; 3]],
        flags: &[super::physics_tables::PhysicsFlags],
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
        flags: &[super::physics_tables::PhysicsFlags],
        gravity: f32,
        dt: f32,
    ) {
        velocities.par_iter_mut()
            .zip(flags.par_iter())
            .for_each(|(vel, flag)| {
                if flag.is_active() && flag.is_dynamic() && flag.has_gravity() {
                    vel[1] += gravity * dt;
                    
                    // Clamp to terminal velocity
                    if vel[1] < super::TERMINAL_VELOCITY {
                        vel[1] = super::TERMINAL_VELOCITY;
                    }
                }
            });
    }
    
    /// Update bounding boxes in parallel
    pub fn update_bounding_boxes(
        bounding_boxes: &mut [super::physics_tables::AABB],
        positions: &[[f32; 3]],
        half_extents: &[[f32; 3]],
    ) {
        bounding_boxes.par_iter_mut()
            .zip(positions.par_iter())
            .zip(half_extents.par_iter())
            .for_each(|((aabb, pos), extents)| {
                *aabb = super::physics_tables::AABB::from_center_half_extents(*pos, *extents);
            });
    }
}