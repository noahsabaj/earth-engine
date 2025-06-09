use std::time::Duration;
use glam::Vec3;
use serde::{Serialize, Deserialize};

use crate::time::{
    TimeOfDay, TimeSpeed, DayPhase,
    CelestialBodies, SunPosition, MoonPhase,
    AmbientLightSettings, calculate_ambient_light,
};

/// Time update event
#[derive(Debug, Clone)]
pub struct TimeUpdate {
    pub time_of_day: TimeOfDay,
    pub day_phase: DayPhase,
    pub sun_position: SunPosition,
    pub moon_phase: MoonPhase,
    pub ambient_light: AmbientLightSettings,
    pub sky_color: Vec3,
}

/// Day/night cycle system
pub struct DayNightCycle {
    /// Current time of day
    time_of_day: TimeOfDay,
    /// Time speed multiplier
    time_speed: TimeSpeed,
    /// Total days elapsed
    day_count: u32,
    /// Celestial bodies calculator
    celestial: CelestialBodies,
    /// Current season (0-1)
    season: f32,
    /// Sky color calculator
    sky_colors: SkyColors,
    /// Weather visibility for lighting
    weather_visibility: f32,
    /// Thunder flash active
    thunder_active: bool,
}

/// Sky color configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyColors {
    pub day_zenith: Vec3,
    pub day_horizon: Vec3,
    pub sunset_zenith: Vec3,
    pub sunset_horizon: Vec3,
    pub night_zenith: Vec3,
    pub night_horizon: Vec3,
    pub star_color: Vec3,
}

impl Default for SkyColors {
    fn default() -> Self {
        Self {
            day_zenith: Vec3::new(0.3, 0.5, 0.9),
            day_horizon: Vec3::new(0.7, 0.8, 0.9),
            sunset_zenith: Vec3::new(0.4, 0.3, 0.6),
            sunset_horizon: Vec3::new(1.0, 0.6, 0.2),
            night_zenith: Vec3::new(0.02, 0.02, 0.08),
            night_horizon: Vec3::new(0.05, 0.05, 0.15),
            star_color: Vec3::new(0.9, 0.9, 1.0),
        }
    }
}

impl DayNightCycle {
    /// Create a new day/night cycle
    pub fn new(start_time: TimeOfDay, latitude: f32) -> Self {
        Self {
            time_of_day: start_time,
            time_speed: TimeSpeed::Fast,
            day_count: 0,
            celestial: CelestialBodies::new(latitude),
            season: 0.5, // Summer
            sky_colors: SkyColors::default(),
            weather_visibility: 1.0,
            thunder_active: false,
        }
    }
    
    /// Update the day/night cycle
    pub fn update(&mut self, dt: Duration) -> TimeUpdate {
        // Advance time
        let delta_seconds = dt.as_secs_f32() * self.time_speed.multiplier();
        let previous_day = self.time_of_day.hours / 24;
        self.time_of_day.advance(delta_seconds);
        
        // Check for day transition
        let current_day = self.time_of_day.hours / 24;
        if current_day != previous_day {
            self.day_count = self.day_count.wrapping_add(1);
            self.celestial.advance_day();
        }
        
        // Calculate celestial positions
        let normalized_time = self.time_of_day.normalized();
        let sun_position = self.celestial.calculate_sun_position(normalized_time);
        let moon_position = self.celestial.calculate_moon_position(normalized_time);
        let moon_phase = self.celestial.calculate_moon_phase();
        let moon_brightness = self.celestial.moon_brightness();
        
        // Calculate ambient lighting
        let ambient_light = calculate_ambient_light(
            sun_position.elevation,
            moon_brightness,
            normalized_time,
            self.weather_visibility,
            self.thunder_active,
        );
        
        // Calculate sky color
        let sky_color = self.calculate_sky_color(sun_position.elevation, normalized_time);
        
        TimeUpdate {
            time_of_day: self.time_of_day,
            day_phase: self.time_of_day.phase(),
            sun_position,
            moon_phase,
            ambient_light,
            sky_color,
        }
    }
    
    /// Calculate sky color gradient
    fn calculate_sky_color(&self, sun_elevation: f32, normalized_time: f32) -> Vec3 {
        if sun_elevation > 0.1 {
            // Daytime
            self.sky_colors.day_zenith
        } else if sun_elevation > -0.1 {
            // Sunrise/sunset
            let is_morning = normalized_time < 0.5;
            let transition_factor = (sun_elevation + 0.1) / 0.2;
            
            if is_morning {
                // Sunrise
                self.sky_colors.sunset_zenith.lerp(self.sky_colors.day_zenith, transition_factor)
            } else {
                // Sunset
                self.sky_colors.day_zenith.lerp(self.sky_colors.sunset_zenith, 1.0 - transition_factor)
            }
        } else {
            // Nighttime
            let star_visibility = self.celestial.star_visibility(sun_elevation);
            self.sky_colors.night_zenith.lerp(
                self.sky_colors.night_zenith + self.sky_colors.star_color * 0.1,
                star_visibility,
            )
        }
    }
    
    /// Set time speed
    pub fn set_time_speed(&mut self, speed: TimeSpeed) {
        self.time_speed = speed;
    }
    
    /// Get current time speed
    pub fn get_time_speed(&self) -> TimeSpeed {
        self.time_speed
    }
    
    /// Set time of day
    pub fn set_time(&mut self, time: TimeOfDay) {
        self.time_of_day = time;
    }
    
    /// Get current time
    pub fn get_time(&self) -> TimeOfDay {
        self.time_of_day
    }
    
    /// Skip to next phase of day
    pub fn skip_to_next_phase(&mut self) {
        let next_hour = match self.time_of_day.phase() {
            DayPhase::Night => 4,          // Skip to dawn
            DayPhase::Dawn => 6,           // Skip to early morning
            DayPhase::EarlyMorning => 8,   // Skip to morning
            DayPhase::Morning => 10,        // Skip to late morning
            DayPhase::LateMorning => 12,   // Skip to noon
            DayPhase::Noon => 14,          // Skip to afternoon
            DayPhase::Afternoon => 16,     // Skip to late afternoon
            DayPhase::LateAfternoon => 18, // Skip to dusk
            DayPhase::Dusk => 20,          // Skip to evening
            DayPhase::Evening => 22,       // Skip to late night
            DayPhase::LateNight => 0,      // Skip to night
        };
        
        self.time_of_day = TimeOfDay::new(next_hour, 0, 0.0);
    }
    
    /// Set season (0 = winter, 0.5 = summer, 1 = winter again)
    pub fn set_season(&mut self, season: f32) {
        self.season = season % 1.0;
        self.celestial.set_season(self.season);
    }
    
    /// Get current season
    pub fn get_season(&self) -> f32 {
        self.season
    }
    
    /// Set weather visibility (affects lighting)
    pub fn set_weather_visibility(&mut self, visibility: f32) {
        self.weather_visibility = visibility.clamp(0.0, 1.0);
    }
    
    /// Set thunder flash
    pub fn set_thunder_active(&mut self, active: bool) {
        self.thunder_active = active;
    }
    
    /// Get total days elapsed
    pub fn get_day_count(&self) -> u32 {
        self.day_count
    }
    
    /// Get sunrise and sunset times for current season
    pub fn get_sun_times(&self) -> (f32, f32) {
        (self.celestial.sunrise_time(), self.celestial.sunset_time())
    }
}

/// Configuration for day/night cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayNightConfig {
    pub start_time: TimeOfDay,
    pub time_speed: f32,
    pub latitude: f32,
    pub start_season: f32,
    pub sky_colors: SkyColors,
}

impl Default for DayNightConfig {
    fn default() -> Self {
        Self {
            start_time: TimeOfDay::new(8, 0, 0.0), // 8 AM
            time_speed: 20.0, // 20x speed
            latitude: 45.0,   // Mid-latitude
            start_season: 0.5, // Summer
            sky_colors: SkyColors::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_day_night_cycle() {
        let mut cycle = DayNightCycle::new(TimeOfDay::new(12, 0, 0.0), 45.0);
        
        // Test time advancement
        let update = cycle.update(Duration::from_secs(60));
        assert_eq!(update.day_phase, DayPhase::Noon);
        
        // Test phase skipping
        cycle.skip_to_next_phase();
        assert_eq!(cycle.get_time().hours, 14);
    }
    
    #[test]
    fn test_sun_position() {
        let mut cycle = DayNightCycle::new(TimeOfDay::new(6, 0, 0.0), 45.0);
        let update = cycle.update(Duration::from_secs(0));
        
        // At 6 AM, sun should be near horizon
        assert!(update.sun_position.elevation.abs() < 0.2);
    }
}