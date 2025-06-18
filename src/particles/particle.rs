use glam::{Vec3, Vec4};
use serde::{Serialize, Deserialize};

// Import physics constants for voxel-scaled gravity
include!("../../constants.rs");
use physics::GRAVITY;

/// Individual particle in the particle system
#[derive(Debug, Clone)]
pub struct Particle {
    /// Position in world space
    pub position: Vec3,
    /// Velocity
    pub velocity: Vec3,
    /// Acceleration
    pub acceleration: Vec3,
    /// Color (RGBA)
    pub color: Vec4,
    /// Size
    pub size: f32,
    /// Current lifetime (seconds)
    pub lifetime: f32,
    /// Maximum lifetime
    pub max_lifetime: f32,
    /// Particle type
    pub particle_type: ParticleType,
    /// Custom properties
    pub properties: ParticleProperties,
}

/// Types of particles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParticleType {
    // Environmental
    Rain,
    Snow,
    Smoke,
    Fire,
    Spark,
    Dust,
    Fog,
    
    // Block effects
    BlockBreak,
    BlockPlace,
    BlockDust,
    
    // Entity effects
    Damage,
    Heal,
    Experience,
    
    // Magic/special
    Magic,
    Enchantment,
    Portal,
    
    // Liquid
    WaterSplash,
    LavaSpark,
    Bubble,
    
    // Custom
    Custom(u32),
}

/// Particle-specific properties
#[derive(Debug, Clone)]
pub struct ParticleProperties {
    /// Gravity multiplier
    pub gravity: f32,
    /// Air resistance
    pub drag: f32,
    /// Rotation angle
    pub rotation: f32,
    /// Rotation speed
    pub rotation_speed: f32,
    /// Size change over lifetime
    pub size_curve: SizeCurve,
    /// Color change over lifetime
    pub color_curve: ColorCurve,
    /// Bounce factor when hitting surfaces
    pub bounce: f32,
    /// Whether particle emits light
    pub emissive: bool,
    /// Light emission intensity
    pub emission_intensity: f32,
    /// Texture frame for animated particles
    pub texture_frame: u32,
    /// Animation speed
    pub animation_speed: f32,
}

/// Size change over particle lifetime
#[derive(Debug, Clone)]
pub enum SizeCurve {
    /// Constant size
    Constant,
    /// Linear decrease
    Linear(f32, f32), // start, end
    /// Grow then shrink
    GrowShrink(f32, f32, f32), // start, peak, end
    /// Custom curve points
    Custom(Vec<(f32, f32)>), // (time, size)
}

/// Color change over particle lifetime
#[derive(Debug, Clone)]
pub enum ColorCurve {
    /// Constant color
    Constant,
    /// Fade alpha only
    FadeOut,
    /// Linear interpolation
    Linear(Vec4, Vec4), // start, end
    /// Temperature-based (for fire)
    Temperature(f32, f32), // start_temp, end_temp
    /// Custom curve points
    Custom(Vec<(f32, Vec4)>), // (time, color)
}

impl Particle {
    /// Create a new particle
    pub fn new(position: Vec3, velocity: Vec3, particle_type: ParticleType) -> Self {
        let (color, size, lifetime) = particle_type.default_properties();
        
        Self {
            position,
            velocity,
            acceleration: Vec3::ZERO,
            color,
            size,
            lifetime,
            max_lifetime: lifetime,
            particle_type,
            properties: ParticleProperties::default_for_type(particle_type),
        }
    }
    
    /// Update particle physics
    pub fn update(&mut self, dt: f32) {
        // Apply acceleration
        self.velocity += self.acceleration * dt;
        
        // Apply drag
        self.velocity *= 1.0 - self.properties.drag * dt;
        
        // Apply gravity (voxel-scaled for 1dcm³ world)
        self.velocity.y -= (-GRAVITY) * self.properties.gravity * dt;
        
        // Update position
        self.position += self.velocity * dt;
        
        // Update lifetime
        self.lifetime -= dt;
        
        // Update rotation
        self.properties.rotation += self.properties.rotation_speed * dt;
        
        // Update animation
        if self.properties.animation_speed > 0.0 {
            self.properties.texture_frame = 
                ((self.max_lifetime - self.lifetime) * self.properties.animation_speed) as u32;
        }
        
        // Update size based on curve
        self.update_size();
        
        // Update color based on curve
        self.update_color();
    }
    
    /// Update particle size based on lifetime
    fn update_size(&mut self) {
        let t = 1.0 - (self.lifetime / self.max_lifetime);
        
        self.size = match &self.properties.size_curve {
            SizeCurve::Constant => self.size,
            SizeCurve::Linear(start, end) => start + (end - start) * t,
            SizeCurve::GrowShrink(start, peak, end) => {
                if t < 0.5 {
                    start + (peak - start) * (t * 2.0)
                } else {
                    peak + (end - peak) * ((t - 0.5) * 2.0)
                }
            },
            SizeCurve::Custom(points) => {
                // Find interpolation points
                Self::interpolate_curve(points, t, |a, b, t| a + (b - a) * t)
            },
        };
    }
    
    /// Update particle color based on lifetime
    fn update_color(&mut self) {
        let t = 1.0 - (self.lifetime / self.max_lifetime);
        
        self.color = match &self.properties.color_curve {
            ColorCurve::Constant => self.color,
            ColorCurve::FadeOut => {
                let mut color = self.color;
                color.w = 1.0 - t;
                color
            },
            ColorCurve::Linear(start, end) => {
                Vec4::new(
                    start.x + (end.x - start.x) * t,
                    start.y + (end.y - start.y) * t,
                    start.z + (end.z - start.z) * t,
                    start.w + (end.w - start.w) * t,
                )
            },
            ColorCurve::Temperature(start_temp, end_temp) => {
                let temp = start_temp + (end_temp - start_temp) * t;
                Self::temperature_to_color(temp)
            },
            ColorCurve::Custom(points) => {
                // Find interpolation points
                Self::interpolate_curve(points, t, |a, b, t| {
                    Vec4::new(
                        a.x + (b.x - a.x) * t,
                        a.y + (b.y - a.y) * t,
                        a.z + (b.z - a.z) * t,
                        a.w + (b.w - a.w) * t,
                    )
                })
            },
        };
    }
    
    /// Interpolate a curve
    fn interpolate_curve<T: Clone>(points: &[(f32, T)], t: f32, interp: impl Fn(&T, &T, f32) -> T) -> T {
        if points.is_empty() {
            panic!("Empty curve");
        }
        
        if t <= points[0].0 {
            return points[0].1.clone();
        }
        
        if t >= points[points.len() - 1].0 {
            return points[points.len() - 1].1.clone();
        }
        
        // Find surrounding points
        for i in 0..points.len() - 1 {
            if t >= points[i].0 && t <= points[i + 1].0 {
                let local_t = (t - points[i].0) / (points[i + 1].0 - points[i].0);
                return interp(&points[i].1, &points[i + 1].1, local_t);
            }
        }
        
        points[0].1.clone()
    }
    
    /// Convert temperature to color (for fire effects)
    fn temperature_to_color(temp: f32) -> Vec4 {
        // Simplified black-body radiation approximation
        let t = temp.clamp(0.0, 1.0);
        
        if t < 0.5 {
            // Black to red to orange
            let t2 = t * 2.0;
            Vec4::new(t2, t2 * 0.3, 0.0, 1.0)
        } else {
            // Orange to yellow to white
            let t2 = (t - 0.5) * 2.0;
            Vec4::new(1.0, 0.3 + t2 * 0.7, t2, 1.0)
        }
    }
    
    /// Check if particle is alive
    pub fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }
    
    /// Get normalized lifetime (0-1)
    pub fn lifetime_normalized(&self) -> f32 {
        (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
    }
}

impl ParticleType {
    /// Convert particle type to u32 ID for data-oriented storage
    pub fn to_id(&self) -> u32 {
        match self {
            ParticleType::Rain => 0,
            ParticleType::Snow => 1,
            ParticleType::Smoke => 2,
            ParticleType::Fire => 3,
            ParticleType::Spark => 4,
            ParticleType::Dust => 5,
            ParticleType::Fog => 6,
            ParticleType::BlockBreak => 7,
            ParticleType::BlockPlace => 8,
            ParticleType::BlockDust => 9,
            ParticleType::Damage => 10,
            ParticleType::Heal => 11,
            ParticleType::Experience => 12,
            ParticleType::Magic => 13,
            ParticleType::Enchantment => 14,
            ParticleType::Portal => 15,
            ParticleType::WaterSplash => 16,
            ParticleType::LavaSpark => 17,
            ParticleType::Bubble => 18,
            ParticleType::Custom(id) => 1000 + id,
        }
    }
    
    /// Get default properties for particle type
    pub fn default_properties(&self) -> (Vec4, f32, f32) {
        // (color, size, lifetime)
        match self {
            ParticleType::Rain => (Vec4::new(0.6, 0.6, 0.8, 0.6), 0.05, 2.0),
            ParticleType::Snow => (Vec4::new(1.0, 1.0, 1.0, 0.8), 0.1, 5.0),
            ParticleType::Smoke => (Vec4::new(0.3, 0.3, 0.3, 0.5), 0.5, 3.0),
            ParticleType::Fire => (Vec4::new(1.0, 0.5, 0.1, 1.0), 0.3, 1.0),
            ParticleType::Spark => (Vec4::new(1.0, 0.8, 0.2, 1.0), 0.05, 0.5),
            ParticleType::Dust => (Vec4::new(0.7, 0.6, 0.5, 0.4), 0.2, 2.0),
            ParticleType::Fog => (Vec4::new(0.8, 0.8, 0.8, 0.2), 2.0, 10.0),
            ParticleType::BlockBreak => (Vec4::new(0.5, 0.5, 0.5, 1.0), 0.1, 1.0),
            ParticleType::BlockPlace => (Vec4::new(0.6, 0.6, 0.6, 0.8), 0.15, 0.5),
            ParticleType::BlockDust => (Vec4::new(0.6, 0.5, 0.4, 0.6), 0.08, 1.5),
            ParticleType::Damage => (Vec4::new(1.0, 0.0, 0.0, 1.0), 0.2, 0.8),
            ParticleType::Heal => (Vec4::new(0.0, 1.0, 0.0, 1.0), 0.2, 1.2),
            ParticleType::Experience => (Vec4::new(1.0, 1.0, 0.0, 1.0), 0.1, 2.0),
            ParticleType::Magic => (Vec4::new(0.5, 0.0, 1.0, 0.8), 0.15, 1.5),
            ParticleType::Enchantment => (Vec4::new(0.8, 0.0, 0.8, 0.9), 0.1, 2.0),
            ParticleType::Portal => (Vec4::new(0.5, 0.0, 0.5, 0.7), 0.12, 3.0),
            ParticleType::WaterSplash => (Vec4::new(0.4, 0.6, 0.9, 0.7), 0.1, 1.0),
            ParticleType::LavaSpark => (Vec4::new(1.0, 0.3, 0.0, 1.0), 0.08, 1.5),
            ParticleType::Bubble => (Vec4::new(0.7, 0.8, 0.9, 0.4), 0.15, 3.0),
            ParticleType::Custom(_) => (Vec4::new(1.0, 1.0, 1.0, 1.0), 0.1, 1.0),
        }
    }
}

impl ParticleProperties {
    /// Get default properties for a particle type
    pub fn default_for_type(particle_type: ParticleType) -> Self {
        match particle_type {
            ParticleType::Rain => Self {
                gravity: 2.0,
                drag: 0.1,
                rotation: 0.0,
                rotation_speed: 0.0,
                size_curve: SizeCurve::Constant,
                color_curve: ColorCurve::Constant,
                bounce: 0.0,
                emissive: false,
                emission_intensity: 0.0,
                texture_frame: 0,
                animation_speed: 0.0,
            },
            ParticleType::Snow => Self {
                gravity: 0.1,
                drag: 0.5,
                rotation: 0.0,
                rotation_speed: 1.0,
                size_curve: SizeCurve::Constant,
                color_curve: ColorCurve::Constant,
                bounce: 0.0,
                emissive: false,
                emission_intensity: 0.0,
                texture_frame: 0,
                animation_speed: 0.0,
            },
            ParticleType::Fire => Self {
                gravity: -0.5, // Fire rises
                drag: 0.8,
                rotation: 0.0,
                rotation_speed: 0.0,
                size_curve: SizeCurve::Linear(0.3, 0.0),
                color_curve: ColorCurve::Temperature(1.0, 0.0),
                bounce: 0.0,
                emissive: true,
                emission_intensity: 1.0,
                texture_frame: 0,
                animation_speed: 10.0,
            },
            ParticleType::Smoke => Self {
                gravity: -0.2,
                drag: 0.5,
                rotation: 0.0,
                rotation_speed: 0.5,
                size_curve: SizeCurve::Linear(0.3, 1.0),
                color_curve: ColorCurve::FadeOut,
                bounce: 0.0,
                emissive: false,
                emission_intensity: 0.0,
                texture_frame: 0,
                animation_speed: 0.0,
            },
            _ => Self::default(),
        }
    }
}

impl Default for ParticleProperties {
    fn default() -> Self {
        Self {
            gravity: 1.0,
            drag: 0.0,
            rotation: 0.0,
            rotation_speed: 0.0,
            size_curve: SizeCurve::Constant,
            color_curve: ColorCurve::Constant,
            bounce: 0.5,
            emissive: false,
            emission_intensity: 0.0,
            texture_frame: 0,
            animation_speed: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_particle_update() {
        let mut particle = Particle::new(
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            ParticleType::Dust,
        );
        
        let initial_y = particle.position.y;
        particle.update(0.1);
        
        // Particle should have moved
        assert!(particle.position.x > 0.0);
        // Gravity should have affected Y position
        assert!(particle.position.y < initial_y);
        // Lifetime should decrease
        assert!(particle.lifetime < particle.max_lifetime);
    }
}