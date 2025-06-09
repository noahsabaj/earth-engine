use glam::Vec3;
use rand::Rng;
use serde::{Serialize, Deserialize};
use std::time::Duration;

use crate::particles::{Particle, ParticleType};

/// Particle emitter that spawns particles
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    /// Position in world space
    pub position: Vec3,
    /// Emitter shape
    pub shape: EmitterShape,
    /// Emission pattern
    pub pattern: EmissionPattern,
    /// Particle type to emit
    pub particle_type: ParticleType,
    /// Emission rate (particles per second)
    pub emission_rate: f32,
    /// Initial velocity range
    pub velocity_range: (Vec3, Vec3),
    /// Initial size range
    pub size_range: (f32, f32),
    /// Lifetime range
    pub lifetime_range: (f32, f32),
    /// Color variation
    pub color_variation: f32,
    /// Whether emitter is active
    pub active: bool,
    /// Duration to emit (None = infinite)
    pub duration: Option<Duration>,
    /// Time elapsed
    pub elapsed: Duration,
    /// Particles to emit accumulator
    spawn_accumulator: f32,
}

/// Shape of the emitter volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmitterShape {
    /// Single point
    Point,
    /// Sphere with radius
    Sphere { radius: f32 },
    /// Box with dimensions
    Box { size: Vec3 },
    /// Cone with angle and height
    Cone { angle: f32, height: f32 },
    /// Cylinder with radius and height
    Cylinder { radius: f32, height: f32 },
    /// Line between two points
    Line { start: Vec3, end: Vec3 },
    /// Disc with radius and normal
    Disc { radius: f32, normal: Vec3 },
}

/// Pattern of particle emission
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EmissionPattern {
    /// Continuous emission
    Continuous,
    /// Burst emission
    Burst { count: u32, interval: f32 },
    /// Random bursts
    RandomBurst { min: u32, max: u32, min_interval: f32, max_interval: f32 },
}

impl ParticleEmitter {
    /// Create a new particle emitter
    pub fn new(position: Vec3, particle_type: ParticleType) -> Self {
        Self {
            position,
            shape: EmitterShape::Point,
            pattern: EmissionPattern::Continuous,
            particle_type,
            emission_rate: 10.0,
            velocity_range: (Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 2.0, 1.0)),
            size_range: (0.8, 1.2),
            lifetime_range: (0.8, 1.2),
            color_variation: 0.1,
            active: true,
            duration: None,
            elapsed: Duration::ZERO,
            spawn_accumulator: 0.0,
        }
    }
    
    /// Create a fire emitter
    pub fn fire(position: Vec3) -> Self {
        Self {
            position,
            shape: EmitterShape::Disc { 
                radius: 0.3, 
                normal: Vec3::Y,
            },
            pattern: EmissionPattern::Continuous,
            particle_type: ParticleType::Fire,
            emission_rate: 30.0,
            velocity_range: (Vec3::new(-0.5, 1.0, -0.5), Vec3::new(0.5, 3.0, 0.5)),
            size_range: (0.2, 0.4),
            lifetime_range: (0.5, 1.5),
            color_variation: 0.2,
            active: true,
            duration: None,
            elapsed: Duration::ZERO,
            spawn_accumulator: 0.0,
        }
    }
    
    /// Create a smoke emitter
    pub fn smoke(position: Vec3) -> Self {
        Self {
            position,
            shape: EmitterShape::Sphere { radius: 0.5 },
            pattern: EmissionPattern::Continuous,
            particle_type: ParticleType::Smoke,
            emission_rate: 5.0,
            velocity_range: (Vec3::new(-0.3, 0.5, -0.3), Vec3::new(0.3, 1.5, 0.3)),
            size_range: (0.3, 0.6),
            lifetime_range: (2.0, 4.0),
            color_variation: 0.3,
            active: true,
            duration: None,
            elapsed: Duration::ZERO,
            spawn_accumulator: 0.0,
        }
    }
    
    /// Create a magic effect emitter
    pub fn magic(position: Vec3) -> Self {
        Self {
            position,
            shape: EmitterShape::Sphere { radius: 0.2 },
            pattern: EmissionPattern::Continuous,
            particle_type: ParticleType::Magic,
            emission_rate: 20.0,
            velocity_range: (Vec3::new(-2.0, -2.0, -2.0), Vec3::new(2.0, 2.0, 2.0)),
            size_range: (0.05, 0.15),
            lifetime_range: (1.0, 2.0),
            color_variation: 0.5,
            active: true,
            duration: Some(Duration::from_secs(2)),
            elapsed: Duration::ZERO,
            spawn_accumulator: 0.0,
        }
    }
    
    /// Update the emitter and spawn particles
    pub fn update(&mut self, dt: Duration) -> Vec<Particle> {
        if !self.active {
            return Vec::new();
        }
        
        self.elapsed += dt;
        
        // Check duration
        if let Some(duration) = self.duration {
            if self.elapsed >= duration {
                self.active = false;
                return Vec::new();
            }
        }
        
        // Calculate particles to spawn
        let dt_secs = dt.as_secs_f32();
        self.spawn_accumulator += self.emission_rate * dt_secs;
        
        let particles_to_spawn = match self.pattern {
            EmissionPattern::Continuous => {
                let count = self.spawn_accumulator as u32;
                self.spawn_accumulator -= count as f32;
                count
            },
            EmissionPattern::Burst { count, interval } => {
                // TODO: Implement burst pattern
                0
            },
            EmissionPattern::RandomBurst { .. } => {
                // TODO: Implement random burst pattern
                0
            },
        };
        
        // Spawn particles
        let mut particles = Vec::with_capacity(particles_to_spawn as usize);
        let mut rng = rand::thread_rng();
        
        for _ in 0..particles_to_spawn {
            let spawn_pos = self.generate_spawn_position(&mut rng);
            let velocity = self.generate_velocity(&mut rng);
            let mut particle = Particle::new(spawn_pos, velocity, self.particle_type);
            
            // Apply variations
            particle.size *= rng.gen_range(self.size_range.0..self.size_range.1);
            particle.max_lifetime *= rng.gen_range(self.lifetime_range.0..self.lifetime_range.1);
            particle.lifetime = particle.max_lifetime;
            
            // Apply color variation
            if self.color_variation > 0.0 {
                let variation = self.color_variation;
                particle.color.x = (particle.color.x + rng.gen_range(-variation..variation)).clamp(0.0, 1.0);
                particle.color.y = (particle.color.y + rng.gen_range(-variation..variation)).clamp(0.0, 1.0);
                particle.color.z = (particle.color.z + rng.gen_range(-variation..variation)).clamp(0.0, 1.0);
            }
            
            particles.push(particle);
        }
        
        particles
    }
    
    /// Generate spawn position based on emitter shape
    fn generate_spawn_position(&self, rng: &mut impl Rng) -> Vec3 {
        match &self.shape {
            EmitterShape::Point => self.position,
            
            EmitterShape::Sphere { radius } => {
                let theta = rng.gen::<f32>() * std::f32::consts::TAU;
                let phi = rng.gen::<f32>() * std::f32::consts::PI;
                let r = rng.gen::<f32>().powf(1.0/3.0) * radius;
                
                self.position + Vec3::new(
                    r * phi.sin() * theta.cos(),
                    r * phi.cos(),
                    r * phi.sin() * theta.sin(),
                )
            },
            
            EmitterShape::Box { size } => {
                self.position + Vec3::new(
                    rng.gen_range(-size.x/2.0..size.x/2.0),
                    rng.gen_range(-size.y/2.0..size.y/2.0),
                    rng.gen_range(-size.z/2.0..size.z/2.0),
                )
            },
            
            EmitterShape::Cone { angle, height } => {
                let h = rng.gen::<f32>() * height;
                let r = h * angle.tan() * rng.gen::<f32>().sqrt();
                let theta = rng.gen::<f32>() * std::f32::consts::TAU;
                
                self.position + Vec3::new(
                    r * theta.cos(),
                    h,
                    r * theta.sin(),
                )
            },
            
            EmitterShape::Cylinder { radius, height } => {
                let r = radius * rng.gen::<f32>().sqrt();
                let theta = rng.gen::<f32>() * std::f32::consts::TAU;
                let h = rng.gen_range(-height/2.0..height/2.0);
                
                self.position + Vec3::new(
                    r * theta.cos(),
                    h,
                    r * theta.sin(),
                )
            },
            
            EmitterShape::Line { start, end } => {
                let t = rng.gen::<f32>();
                self.position + start.lerp(*end, t)
            },
            
            EmitterShape::Disc { radius, normal } => {
                let r = radius * rng.gen::<f32>().sqrt();
                let theta = rng.gen::<f32>() * std::f32::consts::TAU;
                
                // Create perpendicular vectors
                let tangent = if normal.x.abs() < 0.9 {
                    normal.cross(Vec3::X).normalize()
                } else {
                    normal.cross(Vec3::Y).normalize()
                };
                let bitangent = normal.cross(tangent);
                
                self.position + tangent * r * theta.cos() + bitangent * r * theta.sin()
            },
        }
    }
    
    /// Generate initial velocity
    fn generate_velocity(&self, rng: &mut impl Rng) -> Vec3 {
        Vec3::new(
            rng.gen_range(self.velocity_range.0.x..self.velocity_range.1.x),
            rng.gen_range(self.velocity_range.0.y..self.velocity_range.1.y),
            rng.gen_range(self.velocity_range.0.z..self.velocity_range.1.z),
        )
    }
    
    /// Start emitting
    pub fn start(&mut self) {
        self.active = true;
        self.elapsed = Duration::ZERO;
        self.spawn_accumulator = 0.0;
    }
    
    /// Stop emitting
    pub fn stop(&mut self) {
        self.active = false;
    }
    
    /// Reset the emitter
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.spawn_accumulator = 0.0;
    }
    
    /// Check if emitter has finished (for finite duration emitters)
    pub fn is_finished(&self) -> bool {
        if let Some(duration) = self.duration {
            self.elapsed >= duration
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_emitter_spawn() {
        let mut emitter = ParticleEmitter::new(Vec3::ZERO, ParticleType::Dust);
        emitter.emission_rate = 10.0;
        
        let particles = emitter.update(Duration::from_secs_f32(0.1));
        assert_eq!(particles.len(), 1); // Should spawn 1 particle
        
        let particles = emitter.update(Duration::from_secs_f32(1.0));
        assert_eq!(particles.len(), 10); // Should spawn 10 particles
    }
    
    #[test]
    fn test_emitter_shapes() {
        let mut rng = rand::thread_rng();
        
        // Test sphere shape
        let emitter = ParticleEmitter {
            position: Vec3::ZERO,
            shape: EmitterShape::Sphere { radius: 1.0 },
            ..ParticleEmitter::new(Vec3::ZERO, ParticleType::Dust)
        };
        
        let pos = emitter.generate_spawn_position(&mut rng);
        assert!(pos.length() <= 1.0);
    }
}