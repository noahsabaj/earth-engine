use glam::Vec3;
use serde::{Serialize, Deserialize};

/// Position of the sun in the sky
#[derive(Debug, Clone, Copy)]
pub struct SunPosition {
    /// Azimuth angle (0 = North, PI/2 = East, PI = South, 3PI/2 = West)
    pub azimuth: f32,
    /// Elevation angle (0 = horizon, PI/2 = zenith)
    pub elevation: f32,
    /// Direction vector
    pub direction: Vec3,
}

/// Moon phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoonPhase {
    NewMoon,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
    FullMoon,
    WaningGibbous,
    LastQuarter,
    WaningCrescent,
}

/// Celestial bodies system
pub struct CelestialBodies {
    /// Current day (for moon phase calculation)
    day_count: u32,
    /// Latitude for sun position calculation
    latitude: f32,
    /// Season offset (0-1)
    season_offset: f32,
}

impl CelestialBodies {
    /// Create a new celestial bodies system
    pub fn new(latitude: f32) -> Self {
        Self {
            day_count: 0,
            latitude,
            season_offset: 0.0,
        }
    }
    
    /// Calculate sun position for a given time
    pub fn calculate_sun_position(&self, normalized_time: f32) -> SunPosition {
        // Simple sun path calculation
        // Real implementation would consider latitude, season, etc.
        
        // Convert normalized time (0-1) to hour angle
        // 0 = midnight, 0.25 = 6 AM, 0.5 = noon, 0.75 = 6 PM
        let hour_angle = (normalized_time - 0.5) * std::f32::consts::TAU;
        
        // Simple elevation calculation
        // Maximum elevation at noon, below horizon at night
        let base_elevation = hour_angle.cos();
        
        // Add seasonal variation
        let seasonal_modifier = self.season_offset * 0.4; // ±0.4 radians
        let elevation = (base_elevation * std::f32::consts::FRAC_PI_2 + seasonal_modifier)
            .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        
        // Azimuth moves from East to West
        let azimuth = hour_angle + std::f32::consts::PI;
        
        // Calculate direction vector
        let direction = Vec3::new(
            azimuth.sin() * elevation.cos(),
            elevation.sin(),
            azimuth.cos() * elevation.cos(),
        ).normalize();
        
        SunPosition {
            azimuth,
            elevation,
            direction,
        }
    }
    
    /// Calculate moon phase based on day count
    pub fn calculate_moon_phase(&self) -> MoonPhase {
        // Lunar cycle is approximately 29.5 days
        let lunar_cycle = 29.5;
        let phase_progress = (self.day_count as f32 % lunar_cycle) / lunar_cycle;
        
        match (phase_progress * 8.0) as u32 {
            0 => MoonPhase::NewMoon,
            1 => MoonPhase::WaxingCrescent,
            2 => MoonPhase::FirstQuarter,
            3 => MoonPhase::WaxingGibbous,
            4 => MoonPhase::FullMoon,
            5 => MoonPhase::WaningGibbous,
            6 => MoonPhase::LastQuarter,
            _ => MoonPhase::WaningCrescent,
        }
    }
    
    /// Calculate moon position (similar to sun but offset)
    pub fn calculate_moon_position(&self, normalized_time: f32) -> SunPosition {
        // Moon follows a similar path but with different timing
        // Full moon is highest at midnight, new moon at noon
        let moon_offset = match self.calculate_moon_phase() {
            MoonPhase::NewMoon => 0.0,
            MoonPhase::WaxingCrescent => 0.125,
            MoonPhase::FirstQuarter => 0.25,
            MoonPhase::WaxingGibbous => 0.375,
            MoonPhase::FullMoon => 0.5,
            MoonPhase::WaningGibbous => 0.625,
            MoonPhase::LastQuarter => 0.75,
            MoonPhase::WaningCrescent => 0.875,
        };
        
        let adjusted_time = (normalized_time + moon_offset) % 1.0;
        self.calculate_sun_position(adjusted_time)
    }
    
    /// Get moon brightness based on phase
    pub fn moon_brightness(&self) -> f32 {
        match self.calculate_moon_phase() {
            MoonPhase::NewMoon => 0.0,
            MoonPhase::WaxingCrescent => 0.25,
            MoonPhase::FirstQuarter => 0.5,
            MoonPhase::WaxingGibbous => 0.75,
            MoonPhase::FullMoon => 1.0,
            MoonPhase::WaningGibbous => 0.75,
            MoonPhase::LastQuarter => 0.5,
            MoonPhase::WaningCrescent => 0.25,
        }
    }
    
    /// Advance to next day
    pub fn advance_day(&mut self) {
        self.day_count = self.day_count.wrapping_add(1);
    }
    
    /// Set season (0 = winter solstice, 0.5 = summer solstice)
    pub fn set_season(&mut self, season: f32) {
        self.season_offset = (season - 0.5) * 2.0; // Convert to -1 to 1 range
    }
    
    /// Calculate star visibility based on sun position
    pub fn star_visibility(&self, sun_elevation: f32) -> f32 {
        // Stars become visible as sun goes below horizon
        if sun_elevation > 0.0 {
            0.0
        } else {
            // Gradual transition
            (-sun_elevation / std::f32::consts::FRAC_PI_4).min(1.0)
        }
    }
    
    /// Get sunrise time (normalized 0-1)
    pub fn sunrise_time(&self) -> f32 {
        // Simple approximation - varies with season
        0.25 + self.season_offset * 0.05 // 6 AM ± seasonal variation
    }
    
    /// Get sunset time (normalized 0-1)
    pub fn sunset_time(&self) -> f32 {
        // Simple approximation - varies with season
        0.75 - self.season_offset * 0.05 // 6 PM ± seasonal variation
    }
}

/// Sky color calculator
pub struct SkyColors {
    /// Base sky color during day
    pub day_sky: Vec3,
    /// Base sky color during night
    pub night_sky: Vec3,
    /// Horizon color at sunrise
    pub sunrise_horizon: Vec3,
    /// Horizon color at sunset
    pub sunset_horizon: Vec3,
    /// Sun color
    pub sun_color: Vec3,
    /// Moon color
    pub moon_color: Vec3,
}

impl Default for SkyColors {
    fn default() -> Self {
        Self {
            day_sky: Vec3::new(0.4, 0.6, 1.0),      // Light blue
            night_sky: Vec3::new(0.01, 0.01, 0.05), // Very dark blue
            sunrise_horizon: Vec3::new(1.0, 0.6, 0.3), // Orange
            sunset_horizon: Vec3::new(1.0, 0.4, 0.2),  // Red-orange
            sun_color: Vec3::new(1.0, 0.95, 0.8),   // Warm white
            moon_color: Vec3::new(0.9, 0.9, 1.0),   // Cool white
        }
    }
}

impl SkyColors {
    /// Calculate sky color based on sun position and time
    pub fn calculate_sky_color(&self, sun_elevation: f32, normalized_time: f32) -> Vec3 {
        if sun_elevation > 0.0 {
            // Daytime
            let elevation_factor = (sun_elevation / std::f32::consts::FRAC_PI_2).min(1.0);
            
            // Check if near sunrise or sunset
            let is_sunrise = normalized_time < 0.35; // Before ~8:30 AM
            let is_sunset = normalized_time > 0.65;  // After ~3:30 PM
            
            if elevation_factor < 0.3 && (is_sunrise || is_sunset) {
                // Near horizon during sunrise/sunset
                let horizon_color = if is_sunrise {
                    self.sunrise_horizon
                } else {
                    self.sunset_horizon
                };
                
                let mix_factor = elevation_factor / 0.3;
                horizon_color.lerp(self.day_sky, mix_factor)
            } else {
                // Regular daytime
                self.day_sky
            }
        } else {
            // Nighttime
            let night_factor = (-sun_elevation / std::f32::consts::FRAC_PI_4).min(1.0);
            
            // Transition from dusk/dawn colors to night sky
            if night_factor < 0.5 {
                let dusk_color = Vec3::new(0.2, 0.1, 0.3); // Purple dusk
                dusk_color.lerp(self.night_sky, night_factor * 2.0)
            } else {
                self.night_sky
            }
        }
    }
    
    /// Calculate fog color based on time of day
    pub fn calculate_fog_color(&self, sun_elevation: f32, normalized_time: f32) -> Vec3 {
        let sky_color = self.calculate_sky_color(sun_elevation, normalized_time);
        // Fog is slightly grayer than sky
        sky_color.lerp(Vec3::new(0.7, 0.7, 0.7), 0.3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sun_position() {
        let celestial = CelestialBodies::new(45.0);
        
        // Test noon
        let noon = celestial.calculate_sun_position(0.5);
        assert!(noon.elevation > 0.0);
        
        // Test midnight
        let midnight = celestial.calculate_sun_position(0.0);
        assert!(midnight.elevation < 0.0);
    }
    
    #[test]
    fn test_moon_phases() {
        let mut celestial = CelestialBodies::new(45.0);
        
        // Test phase progression
        let initial_phase = celestial.calculate_moon_phase();
        
        // Advance half lunar cycle
        for _ in 0..15 {
            celestial.advance_day();
        }
        
        let later_phase = celestial.calculate_moon_phase();
        assert_ne!(initial_phase, later_phase);
    }
}