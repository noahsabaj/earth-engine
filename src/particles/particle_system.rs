use glam::Vec3;
use std::collections::HashMap;
use std::time::Duration;

use crate::particles::{EffectPreset, Particle, ParticleEffect, ParticleEmitter, ParticlePhysics};
use crate::World;

/// Particle system update result
#[derive(Debug, Clone)]
pub struct ParticleUpdate {
    pub active_particles: usize,
    pub active_emitters: usize,
    pub active_effects: usize,
}

/// Main particle system that manages all particles
pub struct ParticleSystem {
    /// All active particles
    particles: Vec<Particle>,
    /// Active emitters
    emitters: HashMap<u64, ParticleEmitter>,
    /// Active effects
    effects: HashMap<u64, ParticleEffect>,
    /// Physics system
    physics: ParticlePhysics,
    /// Maximum particles allowed
    max_particles: usize,
    /// Next ID for emitters/effects
    next_id: u64,
    /// Particle buffer for rendering
    render_buffer: Vec<ParticleRenderData>,
}

/// Data needed for rendering a particle
#[derive(Debug, Clone, Copy)]
pub struct ParticleRenderData {
    pub position: Vec3,
    pub size: f32,
    pub color: [f32; 4],
    pub rotation: f32,
    pub texture_index: u32,
}

impl ParticleSystem {
    /// Create a new particle system
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles),
            emitters: HashMap::new(),
            effects: HashMap::new(),
            physics: ParticlePhysics::new(),
            max_particles,
            next_id: 0,
            render_buffer: Vec::with_capacity(max_particles),
        }
    }

    /// Update all particles
    pub fn update(&mut self, dt: Duration, world: &World) -> ParticleUpdate {
        let dt_secs = dt.as_secs_f32();

        // Update emitters and spawn new particles
        let mut new_particles = Vec::new();
        for emitter in self.emitters.values_mut() {
            new_particles.extend(emitter.update(dt));
        }

        // Update effects and spawn new particles
        for effect in self.effects.values_mut() {
            new_particles.extend(effect.update(dt));
        }

        // Add new particles (up to limit)
        let available_space = self.max_particles.saturating_sub(self.particles.len());
        let to_add = new_particles.len().min(available_space);
        self.particles
            .extend(new_particles.into_iter().take(to_add));

        // Update existing particles
        for particle in &mut self.particles {
            // Reset acceleration
            particle.acceleration = Vec3::ZERO;

            // Update physics
            self.physics.update_particle(particle, world, dt_secs);

            // Update particle
            particle.update(dt_secs);
        }

        // Remove dead particles
        self.particles.retain(|p| p.is_alive());

        // Remove finished emitters
        self.emitters.retain(|_, e| !e.is_finished());

        // Remove finished effects
        self.effects.retain(|_, e| !e.is_finished());

        // Update render buffer
        self.update_render_buffer();

        ParticleUpdate {
            active_particles: self.particles.len(),
            active_emitters: self.emitters.len(),
            active_effects: self.effects.len(),
        }
    }

    /// Add a new emitter
    pub fn add_emitter(&mut self, emitter: ParticleEmitter) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.emitters.insert(id, emitter);
        id
    }

    /// Remove an emitter
    pub fn remove_emitter(&mut self, id: u64) -> Option<ParticleEmitter> {
        self.emitters.remove(&id)
    }

    /// Get emitter by ID
    pub fn get_emitter(&self, id: u64) -> Option<&ParticleEmitter> {
        self.emitters.get(&id)
    }

    /// Get mutable emitter by ID
    pub fn get_emitter_mut(&mut self, id: u64) -> Option<&mut ParticleEmitter> {
        self.emitters.get_mut(&id)
    }

    /// Add a particle effect
    pub fn add_effect(&mut self, effect: ParticleEffect) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.effects.insert(id, effect);
        id
    }

    /// Add effect from preset
    pub fn add_effect_preset(&mut self, preset: EffectPreset, position: Vec3) -> u64 {
        let effect = ParticleEffect::from_preset(preset, position);
        self.add_effect(effect)
    }

    /// Remove an effect
    pub fn remove_effect(&mut self, id: u64) -> Option<ParticleEffect> {
        self.effects.remove(&id)
    }

    /// Spawn particles directly
    pub fn spawn_particles(&mut self, particles: Vec<Particle>) {
        let available_space = self.max_particles.saturating_sub(self.particles.len());
        let to_add = particles.len().min(available_space);
        self.particles.extend(particles.into_iter().take(to_add));
    }

    /// Create a one-shot burst of particles
    pub fn burst(
        &mut self,
        position: Vec3,
        particle_type: crate::particles::ParticleType,
        count: u32,
    ) {
        let mut emitter = ParticleEmitter::new(position, particle_type);
        emitter.emission_rate = count as f32 * 10.0; // High rate
        emitter.duration = Some(Duration::from_millis(100)); // Short duration
        self.add_emitter(emitter);
    }

    /// Clear all particles
    pub fn clear(&mut self) {
        self.particles.clear();
        self.emitters.clear();
        self.effects.clear();
        self.render_buffer.clear();
    }

    /// Set wind velocity
    pub fn set_wind(&mut self, wind_velocity: Vec3) {
        self.physics.wind_velocity = wind_velocity;
    }

    /// Enable/disable collisions
    pub fn set_collision_enabled(&mut self, enabled: bool) {
        self.physics.collision_enabled = enabled;
    }

    /// Get particle count
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Get render data for all particles
    pub fn get_render_data(&self) -> &[ParticleRenderData] {
        &self.render_buffer
    }

    /// Update render buffer with current particle data
    fn update_render_buffer(&mut self) {
        self.render_buffer.clear();

        for particle in &self.particles {
            self.render_buffer.push(ParticleRenderData {
                position: particle.position,
                size: particle.size,
                color: [
                    particle.color.x,
                    particle.color.y,
                    particle.color.z,
                    particle.color.w,
                ],
                rotation: particle.properties.rotation,
                texture_index: particle.properties.texture_frame,
            });
        }
    }

    /// Apply a force field to all particles
    pub fn apply_force_field(&mut self, center: Vec3, strength: f32, radius: f32) {
        for particle in &mut self.particles {
            self.physics
                .apply_force_field(particle, center, strength, radius);
        }
    }

    /// Apply vortex force to all particles
    pub fn apply_vortex(&mut self, center: Vec3, axis: Vec3, strength: f32, radius: f32) {
        for particle in &mut self.particles {
            self.physics
                .apply_vortex(particle, center, axis, strength, radius);
        }
    }

    /// Apply turbulence to all particles
    pub fn apply_turbulence(&mut self, strength: f32, scale: f32, time: f32) {
        for particle in &mut self.particles {
            self.physics
                .apply_turbulence(particle, strength, scale, time);
        }
    }

    /// Get statistics about the particle system
    pub fn get_stats(&self) -> ParticleSystemStats {
        let mut stats = ParticleSystemStats::default();

        // Count particles by type
        use crate::particles::ParticleType;
        for particle in &self.particles {
            match particle.particle_type {
                ParticleType::Rain => stats.rain_particles += 1,
                ParticleType::Snow => stats.snow_particles += 1,
                ParticleType::Fire => stats.fire_particles += 1,
                ParticleType::Smoke => stats.smoke_particles += 1,
                _ => stats.other_particles += 1,
            }
        }

        stats.total_particles = self.particles.len();
        stats.active_emitters = self.emitters.len();
        stats.active_effects = self.effects.len();
        stats.capacity_used = self.particles.len() as f32 / self.max_particles as f32;

        stats
    }
}

/// Statistics about the particle system
#[derive(Debug, Default, Clone)]
pub struct ParticleSystemStats {
    pub total_particles: usize,
    pub rain_particles: usize,
    pub snow_particles: usize,
    pub fire_particles: usize,
    pub smoke_particles: usize,
    pub other_particles: usize,
    pub active_emitters: usize,
    pub active_effects: usize,
    pub capacity_used: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::ParticleType;

    #[test]
    fn test_particle_system() {
        let world = World::new(32);
        let mut system = ParticleSystem::new(1000);

        // Add an emitter
        let emitter = ParticleEmitter::new(Vec3::ZERO, ParticleType::Dust);
        let id = system.add_emitter(emitter);

        // Update system
        let update = system.update(Duration::from_secs_f32(0.1), &world);
        assert!(update.active_particles > 0);
        assert_eq!(update.active_emitters, 1);

        // Remove emitter
        system.remove_emitter(id);
        let update = system.update(Duration::from_secs_f32(0.1), &world);
        assert_eq!(update.active_emitters, 0);
    }

    #[test]
    fn test_particle_limit() {
        let world = World::new(32);
        let mut system = ParticleSystem::new(10); // Very low limit

        // Try to spawn many particles
        let mut particles = Vec::new();
        for i in 0..20 {
            particles.push(Particle::new(
                Vec3::new(i as f32, 0.0, 0.0),
                Vec3::ZERO,
                ParticleType::Dust,
            ));
        }

        system.spawn_particles(particles);
        assert_eq!(system.particle_count(), 10); // Should be capped at max
    }
}
