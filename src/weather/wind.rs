use glam::Vec3;
use serde::{Serialize, Deserialize};

/// Wind direction (compass directions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindDirection {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

/// Wind strength categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindStrength {
    Calm,
    Light,
    Moderate,
    Strong,
    Gale,
    Storm,
}

impl WindDirection {
    /// Convert to angle in degrees (0 = North, 90 = East, etc.)
    pub fn to_angle(&self) -> f32 {
        match self {
            WindDirection::North => 0.0,
            WindDirection::NorthEast => 45.0,
            WindDirection::East => 90.0,
            WindDirection::SouthEast => 135.0,
            WindDirection::South => 180.0,
            WindDirection::SouthWest => 225.0,
            WindDirection::West => 270.0,
            WindDirection::NorthWest => 315.0,
        }
    }
    
    /// Convert from angle in degrees
    pub fn from_angle(angle: f32) -> Self {
        let normalized = ((angle % 360.0) + 360.0) % 360.0;
        
        match normalized {
            a if a < 22.5 || a >= 337.5 => WindDirection::North,
            a if a < 67.5 => WindDirection::NorthEast,
            a if a < 112.5 => WindDirection::East,
            a if a < 157.5 => WindDirection::SouthEast,
            a if a < 202.5 => WindDirection::South,
            a if a < 247.5 => WindDirection::SouthWest,
            a if a < 292.5 => WindDirection::West,
            _ => WindDirection::NorthWest,
        }
    }
    
    /// Get unit vector for this direction
    pub fn to_vector(&self) -> Vec3 {
        let angle = self.to_angle().to_radians();
        Vec3::new(angle.sin(), 0.0, -angle.cos())
    }
}

impl WindStrength {
    /// Get wind speed range in m/s
    pub fn speed_range(&self) -> (f32, f32) {
        match self {
            WindStrength::Calm => (0.0, 0.5),
            WindStrength::Light => (0.5, 5.0),
            WindStrength::Moderate => (5.0, 10.0),
            WindStrength::Strong => (10.0, 20.0),
            WindStrength::Gale => (20.0, 30.0),
            WindStrength::Storm => (30.0, 50.0),
        }
    }
    
    /// Get average wind speed
    pub fn average_speed(&self) -> f32 {
        let (min, max) = self.speed_range();
        (min + max) / 2.0
    }
    
    /// Create from wind speed
    pub fn from_speed(speed: f32) -> Self {
        match speed {
            s if s < 0.5 => WindStrength::Calm,
            s if s < 5.0 => WindStrength::Light,
            s if s < 10.0 => WindStrength::Moderate,
            s if s < 20.0 => WindStrength::Strong,
            s if s < 30.0 => WindStrength::Gale,
            _ => WindStrength::Storm,
        }
    }
}

/// Wind system that manages wind behavior
pub struct WindSystem {
    /// Current wind velocity
    current_velocity: Vec3,
    /// Target wind velocity (for smooth transitions)
    target_velocity: Vec3,
    /// Transition speed
    transition_speed: f32,
    /// Gust system
    gust_timer: f32,
    gust_frequency: f32,
    gust_strength: f32,
    /// Turbulence
    turbulence_scale: f32,
    turbulence_time: f32,
}

impl WindSystem {
    /// Create a new wind system
    pub fn new() -> Self {
        Self {
            current_velocity: Vec3::ZERO,
            target_velocity: Vec3::ZERO,
            transition_speed: 0.5,
            gust_timer: 0.0,
            gust_frequency: 0.1,
            gust_strength: 5.0,
            turbulence_scale: 0.1,
            turbulence_time: 0.0,
        }
    }
    
    /// Set wind from direction and strength
    pub fn set_wind(&mut self, direction: WindDirection, strength: WindStrength) {
        let speed = strength.average_speed();
        self.target_velocity = direction.to_vector() * speed;
    }
    
    /// Set wind from velocity vector
    pub fn set_wind_velocity(&mut self, velocity: Vec3) {
        self.target_velocity = velocity;
    }
    
    /// Update the wind system
    pub fn update(&mut self, dt: f32) {
        // Smooth transition to target velocity
        let diff = self.target_velocity - self.current_velocity;
        self.current_velocity += diff * (self.transition_speed * dt).min(1.0);
        
        // Update turbulence time
        self.turbulence_time += dt;
        
        // Update gust timer
        self.gust_timer -= dt;
        if self.gust_timer <= 0.0 {
            self.gust_timer = 1.0 / self.gust_frequency;
        }
    }
    
    /// Get current wind velocity (including gusts and turbulence)
    pub fn get_wind_velocity(&self, world_pos: Vec3) -> Vec3 {
        let mut wind = self.current_velocity;
        
        // Add turbulence based on position and time
        let turbulence = self.calculate_turbulence(world_pos);
        wind += turbulence * self.turbulence_scale * self.current_velocity.length();
        
        // Add gusts
        if self.gust_timer < 0.2 {
            let gust_factor = (self.gust_timer / 0.2).sin() * std::f32::consts::PI;
            wind *= 1.0 + gust_factor * self.gust_strength * 0.1;
        }
        
        wind
    }
    
    /// Calculate turbulence at a position
    fn calculate_turbulence(&self, pos: Vec3) -> Vec3 {
        // Simple noise-based turbulence
        let x = (pos.x * 0.1 + self.turbulence_time).sin();
        let y = (pos.y * 0.15 + self.turbulence_time * 1.3).cos();
        let z = (pos.z * 0.12 + self.turbulence_time * 0.7).sin();
        
        Vec3::new(x, y * 0.5, z)
    }
    
    /// Get current wind direction
    pub fn get_wind_direction(&self) -> WindDirection {
        let angle = self.current_velocity.z.atan2(self.current_velocity.x).to_degrees();
        WindDirection::from_angle(angle + 90.0) // Adjust for coordinate system
    }
    
    /// Get current wind strength
    pub fn get_wind_strength(&self) -> WindStrength {
        WindStrength::from_speed(self.current_velocity.length())
    }
    
    /// Set gust parameters
    pub fn set_gust_params(&mut self, frequency: f32, strength: f32) {
        self.gust_frequency = frequency.max(0.01);
        self.gust_strength = strength.clamp(0.0, 10.0);
    }
    
    /// Set turbulence scale
    pub fn set_turbulence(&mut self, scale: f32) {
        self.turbulence_scale = scale.clamp(0.0, 1.0);
    }
}

/// Wind effects on different game elements
pub struct WindEffects {
    /// Effect on particles
    pub particle_drift: Vec3,
    /// Effect on vegetation sway
    pub vegetation_sway: f32,
    /// Effect on water waves
    pub wave_height: f32,
    /// Effect on clouds
    pub cloud_speed: f32,
}

impl WindEffects {
    /// Calculate wind effects from wind velocity
    pub fn from_wind(wind_velocity: Vec3) -> Self {
        let speed = wind_velocity.length();
        
        Self {
            particle_drift: wind_velocity * 0.5, // Particles drift at half wind speed
            vegetation_sway: (speed * 0.1).min(1.0), // Sway amount 0-1
            wave_height: (speed * 0.05).min(2.0), // Wave height in meters
            cloud_speed: speed * 2.0, // Clouds move faster than ground wind
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wind_direction() {
        assert_eq!(WindDirection::from_angle(0.0), WindDirection::North);
        assert_eq!(WindDirection::from_angle(90.0), WindDirection::East);
        assert_eq!(WindDirection::from_angle(180.0), WindDirection::South);
        assert_eq!(WindDirection::from_angle(270.0), WindDirection::West);
        
        let north_vec = WindDirection::North.to_vector();
        assert!(north_vec.x.abs() < 0.01);
        assert!(north_vec.z < -0.9);
    }
    
    #[test]
    fn test_wind_strength() {
        assert_eq!(WindStrength::from_speed(0.1), WindStrength::Calm);
        assert_eq!(WindStrength::from_speed(7.0), WindStrength::Moderate);
        assert_eq!(WindStrength::from_speed(25.0), WindStrength::Gale);
    }
    
    #[test]
    fn test_wind_system() {
        let mut wind = WindSystem::new();
        wind.set_wind(WindDirection::North, WindStrength::Moderate);
        
        // Update should move towards target
        wind.update(1.0);
        let velocity = wind.get_wind_velocity(Vec3::ZERO);
        assert!(velocity.length() > 0.0);
    }
}