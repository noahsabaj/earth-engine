use glam::Vec3;
use std::time::Duration;

use crate::particles::particle_data::{
    ParticleData, EmitterData, ParticleGPUData, 
    prepare_render_data, MAX_PARTICLES
};
use crate::particles::update::{
    update_particles, update_emitters, spawn_particle,
    apply_force_field, apply_vortex, apply_turbulence
};
use crate::particles::ParticleType;
use crate::world::World;

/// Data-oriented particle system
pub struct DOPParticleSystem {
    /// Particle data buffers
    pub particles: ParticleData,
    
    /// Emitter data buffers
    pub emitters: EmitterData,
    
    /// GPU render buffer
    pub gpu_buffer: Vec<ParticleGPUData>,
    
    /// Physics parameters
    pub wind_velocity: Vec3,
    pub collision_enabled: bool,
    
    /// ID counter for emitters
    pub next_emitter_id: u64,
    
    /// Statistics
    pub stats: ParticleStats,
}

/// Particle system statistics
#[derive(Debug, Default, Clone)]
pub struct ParticleStats {
    pub total_particles: usize,
    pub particles_by_type: [usize; 16],
    pub active_emitters: usize,
    pub capacity_used: f32,
    pub particles_spawned_last_frame: usize,
    pub particles_removed_last_frame: usize,
}

impl DOPParticleSystem {
    /// Create a new data-oriented particle system
    pub fn new(max_particles: usize) -> Self {
        let max_particles = max_particles.min(MAX_PARTICLES);
        
        Self {
            particles: ParticleData::new(max_particles),
            emitters: EmitterData::new(1024),
            gpu_buffer: Vec::with_capacity(max_particles),
            wind_velocity: Vec3::ZERO,
            collision_enabled: true,
            next_emitter_id: 0,
            stats: ParticleStats::default(),
        }
    }
    
    /// Update the entire particle system
    pub fn update(&mut self, dt: Duration, world: &World) {
        let dt_secs = dt.as_secs_f32();
        
        // Update statistics
        let initial_count = self.particles.count;
        
        // Update emitters and spawn new particles
        let spawned = update_emitters(
            &mut self.emitters,
            &mut self.particles,
            dt_secs,
            &mut self.next_emitter_id,
        );
        
        // Update all particles
        update_particles(
            &mut self.particles,
            world,
            dt_secs,
            self.wind_velocity,
            self.collision_enabled,
        );
        
        // Update statistics
        self.update_stats(initial_count, spawned);
        
        // Prepare render data
        prepare_render_data(&self.particles, &mut self.gpu_buffer);
    }
    
    /// Add a new emitter
    pub fn add_emitter(
        &mut self,
        position: Vec3,
        particle_type: ParticleType,
        emission_rate: f32,
        duration: Option<Duration>,
    ) -> u64 {
        let id = self.next_emitter_id;
        self.next_emitter_id += 1;
        
        // Add to emitter buffers
        self.emitters.id.push(id);
        
        self.emitters.position_x.push(position.x);
        self.emitters.position_y.push(position.y);
        self.emitters.position_z.push(position.z);
        
        self.emitters.emission_rate.push(emission_rate);
        self.emitters.accumulated_particles.push(0.0);
        self.emitters.particle_type.push(particle_type.to_id());
        
        self.emitters.elapsed_time.push(0.0);
        self.emitters.duration.push(duration.map_or(-1.0, |d| d.as_secs_f32()));
        
        // Default shape (point emitter)
        self.emitters.shape_type.push(0);
        self.emitters.shape_param1.push(0.0);
        self.emitters.shape_param2.push(0.0);
        self.emitters.shape_param3.push(0.0);
        
        // Default velocity
        self.emitters.base_velocity_x.push(0.0);
        self.emitters.base_velocity_y.push(0.0);
        self.emitters.base_velocity_z.push(0.0);
        self.emitters.velocity_variance.push(0.1);
        
        self.emitters.count += 1;
        
        id
    }
    
    /// Add sphere emitter
    pub fn add_sphere_emitter(
        &mut self,
        position: Vec3,
        radius: f32,
        particle_type: ParticleType,
        emission_rate: f32,
        duration: Option<Duration>,
    ) -> u64 {
        let id = self.add_emitter(position, particle_type, emission_rate, duration);
        
        // Find the emitter we just added and update shape
        for i in 0..self.emitters.count {
            if self.emitters.id[i] == id {
                self.emitters.shape_type[i] = 1; // Sphere
                self.emitters.shape_param1[i] = radius;
                break;
            }
        }
        
        id
    }
    
    /// Add box emitter
    pub fn add_box_emitter(
        &mut self,
        position: Vec3,
        size: Vec3,
        particle_type: ParticleType,
        emission_rate: f32,
        duration: Option<Duration>,
    ) -> u64 {
        let id = self.add_emitter(position, particle_type, emission_rate, duration);
        
        // Find the emitter we just added and update shape
        for i in 0..self.emitters.count {
            if self.emitters.id[i] == id {
                self.emitters.shape_type[i] = 2; // Box
                self.emitters.shape_param1[i] = size.x;
                self.emitters.shape_param2[i] = size.y;
                self.emitters.shape_param3[i] = size.z;
                break;
            }
        }
        
        id
    }
    
    /// Remove an emitter
    pub fn remove_emitter(&mut self, id: u64) -> bool {
        for i in 0..self.emitters.count {
            if self.emitters.id[i] == id {
                // Remove by swapping with last
                let last = self.emitters.count - 1;
                if i != last {
                    self.emitters.id.swap(i, last);
                    self.emitters.position_x.swap(i, last);
                    self.emitters.position_y.swap(i, last);
                    self.emitters.position_z.swap(i, last);
                    self.emitters.emission_rate.swap(i, last);
                    self.emitters.accumulated_particles.swap(i, last);
                    self.emitters.particle_type.swap(i, last);
                    self.emitters.elapsed_time.swap(i, last);
                    self.emitters.duration.swap(i, last);
                    self.emitters.shape_type.swap(i, last);
                    self.emitters.shape_param1.swap(i, last);
                    self.emitters.shape_param2.swap(i, last);
                    self.emitters.shape_param3.swap(i, last);
                    self.emitters.base_velocity_x.swap(i, last);
                    self.emitters.base_velocity_y.swap(i, last);
                    self.emitters.base_velocity_z.swap(i, last);
                    self.emitters.velocity_variance.swap(i, last);
                }
                
                // Remove last
                self.emitters.id.pop();
                self.emitters.position_x.pop();
                self.emitters.position_y.pop();
                self.emitters.position_z.pop();
                self.emitters.emission_rate.pop();
                self.emitters.accumulated_particles.pop();
                self.emitters.particle_type.pop();
                self.emitters.elapsed_time.pop();
                self.emitters.duration.pop();
                self.emitters.shape_type.pop();
                self.emitters.shape_param1.pop();
                self.emitters.shape_param2.pop();
                self.emitters.shape_param3.pop();
                self.emitters.base_velocity_x.pop();
                self.emitters.base_velocity_y.pop();
                self.emitters.base_velocity_z.pop();
                self.emitters.velocity_variance.pop();
                
                self.emitters.count -= 1;
                return true;
            }
        }
        false
    }
    
    /// Set emitter velocity
    pub fn set_emitter_velocity(&mut self, id: u64, velocity: Vec3, variance: f32) {
        for i in 0..self.emitters.count {
            if self.emitters.id[i] == id {
                self.emitters.base_velocity_x[i] = velocity.x;
                self.emitters.base_velocity_y[i] = velocity.y;
                self.emitters.base_velocity_z[i] = velocity.z;
                self.emitters.velocity_variance[i] = variance;
                break;
            }
        }
    }
    
    /// Create a burst of particles
    pub fn burst(&mut self, position: Vec3, particle_type: ParticleType, count: u32) {
        // Create a short-lived emitter with high emission rate
        self.add_emitter(
            position,
            particle_type,
            count as f32 * 10.0,
            Some(Duration::from_millis(100)),
        );
    }
    
    /// Spawn particles directly
    pub fn spawn_particles(&mut self, positions: &[Vec3], velocities: &[Vec3], particle_type: ParticleType) {
        let count = positions.len().min(velocities.len());
        for i in 0..count {
            spawn_particle(&mut self.particles, positions[i], velocities[i], particle_type.to_id());
        }
    }
    
    /// Clear all particles and emitters
    pub fn clear(&mut self) {
        self.particles.clear();
        self.emitters.clear();
        self.gpu_buffer.clear();
        self.stats = ParticleStats::default();
    }
    
    /// Apply force field
    pub fn apply_force_field(&mut self, center: Vec3, strength: f32, radius: f32) {
        apply_force_field(&mut self.particles, center, strength, radius);
    }
    
    /// Apply vortex force
    pub fn apply_vortex(&mut self, center: Vec3, axis: Vec3, strength: f32, radius: f32) {
        apply_vortex(&mut self.particles, center, axis, strength, radius);
    }
    
    /// Apply turbulence
    pub fn apply_turbulence(&mut self, strength: f32, scale: f32, time: f32) {
        apply_turbulence(&mut self.particles, strength, scale, time);
    }
    
    /// Get GPU render data
    pub fn get_gpu_data(&self) -> &[ParticleGPUData] {
        &self.gpu_buffer
    }
    
    /// Get particle count
    pub fn particle_count(&self) -> usize {
        self.particles.count
    }
    
    /// Get emitter count
    pub fn emitter_count(&self) -> usize {
        self.emitters.count
    }
    
    /// Update statistics
    fn update_stats(&mut self, initial_count: usize, spawned: usize) {
        self.stats.total_particles = self.particles.count;
        self.stats.active_emitters = self.emitters.count;
        self.stats.capacity_used = self.particles.count as f32 / self.particles.position_x.capacity() as f32;
        self.stats.particles_spawned_last_frame = spawned;
        
        let removed = initial_count + spawned - self.particles.count;
        self.stats.particles_removed_last_frame = removed;
        
        // Count particles by type
        self.stats.particles_by_type = [0; 16];
        for i in 0..self.particles.count {
            let type_idx = (self.particles.particle_type[i] as usize).min(15);
            self.stats.particles_by_type[type_idx] += 1;
        }
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> &ParticleStats {
        &self.stats
    }
}

/// Create preset particle effects
pub fn create_fire_effect(system: &mut DOPParticleSystem, position: Vec3, size: f32) {
    // Main fire emitter
    let fire_id = system.add_sphere_emitter(
        position,
        size * 0.5,
        ParticleType::Fire,
        100.0 * size,
        None,
    );
    system.set_emitter_velocity(fire_id, Vec3::new(0.0, 2.0, 0.0), 0.5);
    
    // Smoke emitter
    let smoke_id = system.add_sphere_emitter(
        position + Vec3::new(0.0, size, 0.0),
        size * 0.3,
        ParticleType::Smoke,
        50.0 * size,
        None,
    );
    system.set_emitter_velocity(smoke_id, Vec3::new(0.0, 1.0, 0.0), 0.3);
    
    // Sparks
    let spark_id = system.add_emitter(
        position,
        ParticleType::Spark,
        20.0 * size,
        None,
    );
    system.set_emitter_velocity(spark_id, Vec3::new(0.0, 3.0, 0.0), 1.0);
}

/// Create rain effect
pub fn create_rain_effect(system: &mut DOPParticleSystem, center: Vec3, area: Vec3, intensity: f32) {
    let rain_id = system.add_box_emitter(
        center + Vec3::new(0.0, area.y, 0.0),
        area * 2.0,
        ParticleType::Rain,
        1000.0 * intensity,
        None,
    );
    system.set_emitter_velocity(rain_id, Vec3::new(0.0, -10.0, 0.0), 0.5);
}

/// Create snow effect
pub fn create_snow_effect(system: &mut DOPParticleSystem, center: Vec3, area: Vec3, intensity: f32) {
    let snow_id = system.add_box_emitter(
        center + Vec3::new(0.0, area.y, 0.0),
        area * 2.0,
        ParticleType::Snow,
        500.0 * intensity,
        None,
    );
    system.set_emitter_velocity(snow_id, Vec3::new(0.0, -1.0, 0.0), 0.2);
}

/// Create explosion effect
pub fn create_explosion_effect(system: &mut DOPParticleSystem, position: Vec3, power: f32) {
    // Initial burst of fire
    system.burst(position, ParticleType::Fire, (power * 100.0) as u32);
    
    // Sparks flying out
    let spark_id = system.add_sphere_emitter(
        position,
        power * 0.1,
        ParticleType::Spark,
        power * 200.0,
        Some(Duration::from_millis(200)),
    );
    system.set_emitter_velocity(spark_id, Vec3::ZERO, power * 5.0);
    
    // Smoke cloud
    let smoke_id = system.add_sphere_emitter(
        position,
        power * 0.5,
        ParticleType::Smoke,
        power * 50.0,
        Some(Duration::from_secs(2)),
    );
    system.set_emitter_velocity(smoke_id, Vec3::new(0.0, 1.0, 0.0), power * 0.5);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_particle_system_creation() {
        let system = DOPParticleSystem::new(10000);
        assert_eq!(system.particle_count(), 0);
        assert_eq!(system.emitter_count(), 0);
    }
    
    #[test]
    fn test_emitter_management() {
        let mut system = DOPParticleSystem::new(10000);
        
        // Add emitter
        let id = system.add_emitter(Vec3::ZERO, ParticleType::Dust, 10.0, None);
        assert_eq!(system.emitter_count(), 1);
        
        // Remove emitter
        assert!(system.remove_emitter(id));
        assert_eq!(system.emitter_count(), 0);
        
        // Try to remove non-existent emitter
        assert!(!system.remove_emitter(999));
    }
    
    #[test]
    fn test_particle_spawning() {
        let mut system = DOPParticleSystem::new(10000);
        
        // Spawn particles directly
        let positions = vec![Vec3::ZERO, Vec3::ONE, Vec3::new(2.0, 0.0, 0.0)];
        let velocities = vec![Vec3::ZERO; 3];
        
        system.spawn_particles(&positions, &velocities, ParticleType::Dust);
        assert_eq!(system.particle_count(), 3);
    }
}