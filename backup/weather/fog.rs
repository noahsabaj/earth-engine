use glam::Vec3;
use serde::{Serialize, Deserialize};

/// Fog density levels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FogDensity {
    None,
    Light,
    Medium,
    Heavy,
    VeryHeavy,
}

/// Fog configuration and settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FogSettings {
    pub density: FogDensity,
    pub color: Vec3,
    pub start_distance: f32,
    pub end_distance: f32,
    pub height_falloff: f32,
    pub height_density: f32,
}

impl FogSettings {
    /// Create fog settings from density
    pub fn from_density(density: FogDensity) -> Self {
        let (start, end, height_density) = match density {
            FogDensity::None => (1000.0, 2000.0, 0.0),
            FogDensity::Light => (100.0, 500.0, 0.01),
            FogDensity::Medium => (50.0, 250.0, 0.03),
            FogDensity::Heavy => (20.0, 100.0, 0.05),
            FogDensity::VeryHeavy => (5.0, 50.0, 0.1),
        };
        
        Self {
            density,
            color: Vec3::new(0.7, 0.7, 0.8), // Light gray-blue
            start_distance: start,
            end_distance: end,
            height_falloff: 0.01,
            height_density,
        }
    }
    
    /// Create morning mist settings
    pub fn morning_mist() -> Self {
        Self {
            density: FogDensity::Light,
            color: Vec3::new(0.9, 0.85, 0.7), // Warm morning color
            start_distance: 50.0,
            end_distance: 300.0,
            height_falloff: 0.05,
            height_density: 0.02,
        }
    }
    
    /// Create thick fog settings
    pub fn thick_fog() -> Self {
        Self {
            density: FogDensity::Heavy,
            color: Vec3::new(0.6, 0.6, 0.65),
            start_distance: 10.0,
            end_distance: 80.0,
            height_falloff: 0.02,
            height_density: 0.08,
        }
    }
    
    /// Calculate fog factor at a given distance
    pub fn calculate_fog_factor(&self, distance: f32) -> f32 {
        if self.density == FogDensity::None {
            return 0.0;
        }
        
        // Linear fog
        let fog = (distance - self.start_distance) / (self.end_distance - self.start_distance);
        fog.clamp(0.0, 1.0)
    }
    
    /// Calculate height-based fog density
    pub fn calculate_height_fog(&self, world_height: f32, fog_base_height: f32) -> f32 {
        if self.density == FogDensity::None {
            return 0.0;
        }
        
        let height_above_fog = world_height - fog_base_height;
        if height_above_fog <= 0.0 {
            return 1.0; // Full fog below base height
        }
        
        // Exponential falloff with height
        ((-height_above_fog * self.height_falloff).exp() * self.height_density).min(1.0)
    }
    
    /// Interpolate between two fog settings
    pub fn interpolate(&self, other: &FogSettings, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        
        Self {
            density: if t < 0.5 { self.density } else { other.density },
            color: self.color + (other.color - self.color) * t,
            start_distance: self.start_distance + (other.start_distance - self.start_distance) * t,
            end_distance: self.end_distance + (other.end_distance - self.end_distance) * t,
            height_falloff: self.height_falloff + (other.height_falloff - self.height_falloff) * t,
            height_density: self.height_density + (other.height_density - self.height_density) * t,
        }
    }
}

impl Default for FogSettings {
    fn default() -> Self {
        Self::from_density(FogDensity::None)
    }
}

/// Volumetric fog system for more realistic fog rendering
pub struct VolumetricFog {
    /// 3D density field for fog
    density_field: Vec<f32>,
    /// Dimensions of the density field
    dimensions: (usize, usize, usize),
    /// World space size of the fog volume
    world_size: Vec3,
    /// Origin of the fog volume in world space
    origin: Vec3,
}

impl VolumetricFog {
    /// Create a new volumetric fog system
    pub fn new(dimensions: (usize, usize, usize), world_size: Vec3, origin: Vec3) -> Self {
        let total_cells = dimensions.0 * dimensions.1 * dimensions.2;
        Self {
            density_field: vec![0.0; total_cells],
            dimensions,
            world_size,
            origin,
        }
    }
    
    /// Set fog density at a grid position
    pub fn set_density(&mut self, x: usize, y: usize, z: usize, density: f32) {
        if x < self.dimensions.0 && y < self.dimensions.1 && z < self.dimensions.2 {
            let index = x + y * self.dimensions.0 + z * self.dimensions.0 * self.dimensions.1;
            if let Some(field_entry) = self.density_field.get_mut(index) {
                *field_entry = density.clamp(0.0, 1.0);
            }
        }
    }
    
    /// Get fog density at a grid position
    pub fn get_density(&self, x: usize, y: usize, z: usize) -> f32 {
        if x < self.dimensions.0 && y < self.dimensions.1 && z < self.dimensions.2 {
            let index = x + y * self.dimensions.0 + z * self.dimensions.0 * self.dimensions.1;
            self.density_field.get(index).copied().unwrap_or(0.0)
        } else {
            0.0
        }
    }
    
    /// Sample fog density at a world position (trilinear interpolation)
    pub fn sample_world(&self, world_pos: Vec3) -> f32 {
        let local_pos = world_pos - self.origin;
        let normalized = local_pos / self.world_size;
        
        // Convert to grid coordinates
        let x = normalized.x * self.dimensions.0 as f32;
        let y = normalized.y * self.dimensions.1 as f32;
        let z = normalized.z * self.dimensions.2 as f32;
        
        // Get integer coordinates
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let z0 = z.floor() as usize;
        
        let x1 = (x0 + 1).min(self.dimensions.0 - 1);
        let y1 = (y0 + 1).min(self.dimensions.1 - 1);
        let z1 = (z0 + 1).min(self.dimensions.2 - 1);
        
        // Get fractional parts
        let fx = x.fract();
        let fy = y.fract();
        let fz = z.fract();
        
        // Trilinear interpolation
        let d000 = self.get_density(x0, y0, z0);
        let d100 = self.get_density(x1, y0, z0);
        let d010 = self.get_density(x0, y1, z0);
        let d110 = self.get_density(x1, y1, z0);
        let d001 = self.get_density(x0, y0, z1);
        let d101 = self.get_density(x1, y0, z1);
        let d011 = self.get_density(x0, y1, z1);
        let d111 = self.get_density(x1, y1, z1);
        
        let dx00 = d000 + (d100 - d000) * fx;
        let dx10 = d010 + (d110 - d010) * fx;
        let dx01 = d001 + (d101 - d001) * fx;
        let dx11 = d011 + (d111 - d011) * fx;
        
        let dxy0 = dx00 + (dx10 - dx00) * fy;
        let dxy1 = dx01 + (dx11 - dx01) * fy;
        
        dxy0 + (dxy1 - dxy0) * fz
    }
    
    /// Apply a height-based fog gradient
    pub fn apply_height_gradient(&mut self, base_height: f32, falloff: f32) {
        let cell_height = self.world_size.y / self.dimensions.1 as f32;
        
        for y in 0..self.dimensions.1 {
            let world_y = self.origin.y + y as f32 * cell_height;
            let height_factor = if world_y <= base_height {
                1.0
            } else {
                (-(world_y - base_height) * falloff).exp()
            };
            
            for z in 0..self.dimensions.2 {
                for x in 0..self.dimensions.0 {
                    let current = self.get_density(x, y, z);
                    self.set_density(x, y, z, current * height_factor);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fog_settings() {
        let light_fog = FogSettings::from_density(FogDensity::Light);
        let heavy_fog = FogSettings::from_density(FogDensity::Heavy);
        
        assert!(light_fog.start_distance > heavy_fog.start_distance);
        assert!(light_fog.end_distance > heavy_fog.end_distance);
        
        // Test fog factor calculation
        let factor_near = heavy_fog.calculate_fog_factor(10.0);
        let factor_far = heavy_fog.calculate_fog_factor(200.0);
        assert!(factor_near < factor_far);
    }
    
    #[test]
    fn test_volumetric_fog() {
        let mut fog = VolumetricFog::new((10, 10, 10), Vec3::new(100.0, 100.0, 100.0), Vec3::ZERO);
        
        fog.set_density(5, 5, 5, 0.8);
        assert_eq!(fog.get_density(5, 5, 5), 0.8);
        
        // Test world sampling
        let density = fog.sample_world(Vec3::new(50.0, 50.0, 50.0));
        assert!(density >= 0.0 && density <= 1.0);
    }
}