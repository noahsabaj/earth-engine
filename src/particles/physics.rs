use glam::Vec3;
use crate::particles::Particle;
use crate::{World, VoxelPos};

/// Particle physics system
pub struct ParticlePhysics {
    /// Global wind velocity
    pub wind_velocity: Vec3,
    /// Collision enabled
    pub collision_enabled: bool,
    /// Water drag multiplier
    pub water_drag: f32,
}

/// Collision result
#[derive(Debug, Clone)]
pub struct ParticleCollision {
    pub position: Vec3,
    pub normal: Vec3,
    pub depth: f32,
    pub is_liquid: bool,
}

impl ParticlePhysics {
    /// Create a new particle physics system
    pub fn new() -> Self {
        Self {
            wind_velocity: Vec3::ZERO,
            collision_enabled: true,
            water_drag: 3.0,
        }
    }
    
    /// Update particle with physics
    pub fn update_particle(&self, particle: &mut Particle, world: &World, dt: f32) {
        // Apply wind
        if self.wind_velocity.length_squared() > 0.0 {
            let wind_effect = self.wind_velocity * particle.properties.drag * dt;
            particle.velocity += wind_effect;
        }
        
        // Check for collisions if enabled
        if self.collision_enabled {
            if let Some(collision) = self.check_collision(particle, world) {
                self.resolve_collision(particle, &collision);
                
                // Apply water physics if in liquid
                if collision.is_liquid {
                    particle.velocity *= 1.0 - self.water_drag * dt;
                    particle.properties.gravity *= 0.5; // Buoyancy
                }
            }
        }
    }
    
    /// Check for particle collision with world
    fn check_collision(&self, particle: &Particle, world: &World) -> Option<ParticleCollision> {
        let pos = particle.position;
        let voxel_pos = VoxelPos::new(
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        );
        
        let block = world.get_block(voxel_pos);
        if block.0 == 0 { // Air
            return None;
        }
        
        // Simple AABB collision with voxel
        let block_center = Vec3::new(
            voxel_pos.x as f32 + 0.5,
            voxel_pos.y as f32 + 0.5,
            voxel_pos.z as f32 + 0.5,
        );
        
        let half_size = 0.5 + particle.size * 0.5;
        let diff = pos - block_center;
        
        // Check if particle overlaps block
        if diff.x.abs() < half_size && diff.y.abs() < half_size && diff.z.abs() < half_size {
            // Find closest face
            let mut min_dist = f32::MAX;
            let mut normal = Vec3::ZERO;
            
            // Check each face
            let faces = [
                (Vec3::X, diff.x),
                (Vec3::NEG_X, -diff.x),
                (Vec3::Y, diff.y),
                (Vec3::NEG_Y, -diff.y),
                (Vec3::Z, diff.z),
                (Vec3::NEG_Z, -diff.z),
            ];
            
            for (face_normal, dist) in faces {
                let face_dist = half_size - dist;
                if face_dist < min_dist && face_dist > 0.0 {
                    min_dist = face_dist;
                    normal = face_normal;
                }
            }
            
            // Check if block is liquid
            let is_liquid = block.0 == 6; // Water block ID
            
            Some(ParticleCollision {
                position: block_center + normal * 0.5,
                normal,
                depth: min_dist,
                is_liquid,
            })
        } else {
            None
        }
    }
    
    /// Resolve particle collision
    fn resolve_collision(&self, particle: &mut Particle, collision: &ParticleCollision) {
        // Push particle out of collision
        particle.position += collision.normal * collision.depth;
        
        // Reflect velocity
        let dot = particle.velocity.dot(collision.normal);
        if dot < 0.0 {
            particle.velocity -= collision.normal * dot * (1.0 + particle.properties.bounce);
            
            // Apply friction
            let tangent_velocity = particle.velocity - collision.normal * particle.velocity.dot(collision.normal);
            particle.velocity -= tangent_velocity * 0.2; // 20% friction
        }
    }
    
    /// Apply force field to particle
    pub fn apply_force_field(&self, particle: &mut Particle, center: Vec3, strength: f32, radius: f32) {
        let diff = particle.position - center;
        let dist = diff.length();
        
        if dist < radius && dist > 0.001 {
            let force = diff.normalize() * strength * (1.0 - dist / radius);
            particle.acceleration += force;
        }
    }
    
    /// Apply vortex force to particle
    pub fn apply_vortex(&self, particle: &mut Particle, center: Vec3, axis: Vec3, strength: f32, radius: f32) {
        let diff = particle.position - center;
        let dist = diff.length();
        
        if dist < radius && dist > 0.001 {
            let tangent = axis.cross(diff).normalize();
            let force = tangent * strength * (1.0 - dist / radius);
            particle.acceleration += force;
        }
    }
    
    /// Apply turbulence to particle
    pub fn apply_turbulence(&self, particle: &mut Particle, strength: f32, scale: f32, time: f32) {
        let pos = particle.position * scale;
        
        // Simple turbulence using sin/cos
        let turb = Vec3::new(
            (pos.y + time).sin() * (pos.z + time * 0.7).cos(),
            (pos.z + time * 1.3).sin() * (pos.x + time * 0.5).cos(),
            (pos.x + time * 0.9).sin() * (pos.y + time * 1.1).cos(),
        );
        
        particle.acceleration += turb * strength;
    }
}

/// Particle attractor/repulsor
pub struct ParticleForceField {
    pub position: Vec3,
    pub strength: f32,
    pub radius: f32,
    pub falloff: FalloffType,
}

/// Force field falloff types
#[derive(Debug, Clone, Copy)]
pub enum FalloffType {
    Linear,
    Quadratic,
    Exponential,
}

impl ParticleForceField {
    /// Apply force to particle
    pub fn apply(&self, particle: &mut Particle) {
        let diff = particle.position - self.position;
        let dist = diff.length();
        
        if dist < self.radius && dist > 0.001 {
            let factor = match self.falloff {
                FalloffType::Linear => 1.0 - dist / self.radius,
                FalloffType::Quadratic => {
                    let t = 1.0 - dist / self.radius;
                    t * t
                },
                FalloffType::Exponential => {
                    (-dist / self.radius * 3.0).exp()
                },
            };
            
            let force = diff.normalize() * self.strength * factor;
            particle.acceleration += force;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::ParticleType;
    
    #[test]
    fn test_force_field() {
        let mut particle = Particle::new(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO, ParticleType::Dust);
        
        let field = ParticleForceField {
            position: Vec3::ZERO,
            strength: 10.0,
            radius: 5.0,
            falloff: FalloffType::Linear,
        };
        
        field.apply(&mut particle);
        
        // Should push particle away from origin
        assert!(particle.acceleration.x > 0.0);
    }
}