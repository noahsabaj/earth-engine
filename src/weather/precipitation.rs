use glam::Vec3;
use rand::Rng;
use serde::{Serialize, Deserialize};
use std::time::Duration;

/// Type of precipitation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecipitationType {
    Rain,
    Snow,
    Sleet,
    Hail,
}

/// A single precipitation particle
#[derive(Debug, Clone)]
pub struct PrecipitationParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub lifetime: f32,
    pub particle_type: PrecipitationType,
    pub size: f32,
}

/// Manages precipitation particles
pub struct PrecipitationSystem {
    particles: Vec<PrecipitationParticle>,
    spawn_rate: f32,
    spawn_timer: f32,
    bounds: PrecipitationBounds,
    wind_offset: Vec3,
}

/// Bounds for precipitation spawning
#[derive(Debug, Clone)]
struct PrecipitationBounds {
    center: Vec3,
    radius: f32,
    min_height: f32,
    max_height: f32,
}

impl PrecipitationSystem {
    /// Create a new precipitation system
    pub fn new(center: Vec3, radius: f32, height_range: (f32, f32)) -> Self {
        Self {
            particles: Vec::with_capacity(10000),
            spawn_rate: 100.0, // particles per second
            spawn_timer: 0.0,
            bounds: PrecipitationBounds {
                center,
                radius,
                min_height: height_range.0,
                max_height: height_range.1,
            },
            wind_offset: Vec3::ZERO,
        }
    }
    
    /// Update the precipitation system
    pub fn update(&mut self, dt: f32, precip_type: PrecipitationType, intensity: f32) {
        // Update spawn rate based on intensity
        self.spawn_rate = intensity * 1000.0;
        
        // Spawn new particles
        self.spawn_timer += dt;
        let particles_to_spawn = (self.spawn_timer * self.spawn_rate) as usize;
        if particles_to_spawn > 0 {
            self.spawn_timer -= particles_to_spawn as f32 / self.spawn_rate;
            self.spawn_particles(particles_to_spawn, precip_type);
        }
        
        // Update existing particles
        self.update_particles(dt, precip_type);
        
        // Remove dead particles
        self.particles.retain(|p| p.lifetime > 0.0 && p.position.y > self.bounds.min_height);
    }
    
    /// Set wind effect on precipitation
    pub fn set_wind(&mut self, wind_velocity: Vec3) {
        self.wind_offset = wind_velocity * 0.5; // Reduce wind effect on precipitation
    }
    
    /// Spawn new particles
    fn spawn_particles(&mut self, count: usize, precip_type: PrecipitationType) {
        let mut rng = rand::thread_rng();
        
        for _ in 0..count {
            let angle = rng.gen::<f32>() * std::f32::consts::TAU;
            let distance = rng.gen::<f32>() * self.bounds.radius;
            
            let position = Vec3::new(
                self.bounds.center.x + angle.cos() * distance,
                self.bounds.max_height,
                self.bounds.center.z + angle.sin() * distance,
            );
            
            let velocity = match precip_type {
                PrecipitationType::Rain => Vec3::new(
                    self.wind_offset.x + rng.gen_range(-1.0..1.0),
                    -rng.gen_range(15.0..25.0),
                    self.wind_offset.z + rng.gen_range(-1.0..1.0),
                ),
                PrecipitationType::Snow => Vec3::new(
                    self.wind_offset.x + rng.gen_range(-2.0..2.0),
                    -rng.gen_range(1.0..3.0),
                    self.wind_offset.z + rng.gen_range(-2.0..2.0),
                ),
                PrecipitationType::Sleet => Vec3::new(
                    self.wind_offset.x + rng.gen_range(-1.5..1.5),
                    -rng.gen_range(8.0..15.0),
                    self.wind_offset.z + rng.gen_range(-1.5..1.5),
                ),
                PrecipitationType::Hail => Vec3::new(
                    self.wind_offset.x + rng.gen_range(-0.5..0.5),
                    -rng.gen_range(20.0..30.0),
                    self.wind_offset.z + rng.gen_range(-0.5..0.5),
                ),
            };
            
            let size = match precip_type {
                PrecipitationType::Rain => rng.gen_range(0.05..0.15),
                PrecipitationType::Snow => rng.gen_range(0.1..0.3),
                PrecipitationType::Sleet => rng.gen_range(0.08..0.2),
                PrecipitationType::Hail => rng.gen_range(0.2..0.5),
            };
            
            self.particles.push(PrecipitationParticle {
                position,
                velocity,
                lifetime: 10.0, // seconds
                particle_type: precip_type,
                size,
            });
        }
    }
    
    /// Update particle positions and physics
    fn update_particles(&mut self, dt: f32, precip_type: PrecipitationType) {
        let gravity = match precip_type {
            PrecipitationType::Snow => Vec3::new(0.0, -2.0, 0.0),
            _ => Vec3::new(0.0, -9.81, 0.0),
        };
        
        for particle in &mut self.particles {
            // Apply physics
            particle.velocity += gravity * dt;
            particle.position += particle.velocity * dt;
            particle.lifetime -= dt;
            
            // Add some turbulence for snow
            if precip_type == PrecipitationType::Snow {
                let time = particle.lifetime;
                particle.position.x += (time * 2.0).sin() * 0.1 * dt;
                particle.position.z += (time * 3.0).cos() * 0.1 * dt;
            }
        }
    }
    
    /// Get current particles for rendering
    pub fn get_particles(&self) -> &[PrecipitationParticle] {
        &self.particles
    }
    
    /// Clear all particles
    pub fn clear(&mut self) {
        self.particles.clear();
    }
    
    /// Update the bounds of the precipitation area
    pub fn update_bounds(&mut self, center: Vec3, radius: f32) {
        self.bounds.center = center;
        self.bounds.radius = radius;
    }
    
    /// Get particle count
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}

/// Configuration for precipitation rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecipitationConfig {
    pub max_particles: usize,
    pub spawn_radius: f32,
    pub height_range: (f32, f32),
    pub wind_effect: f32,
    pub gravity_multiplier: f32,
}

impl Default for PrecipitationConfig {
    fn default() -> Self {
        Self {
            max_particles: 10000,
            spawn_radius: 100.0,
            height_range: (0.0, 200.0),
            wind_effect: 0.5,
            gravity_multiplier: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_precipitation_system() {
        let mut system = PrecipitationSystem::new(Vec3::ZERO, 50.0, (0.0, 100.0));
        
        // Update with rain
        system.update(0.1, PrecipitationType::Rain, 0.5);
        assert!(system.particle_count() > 0);
        
        // Update multiple times
        for _ in 0..10 {
            system.update(0.1, PrecipitationType::Rain, 0.5);
        }
        
        // Should have accumulated particles
        assert!(system.particle_count() > 10);
    }
}