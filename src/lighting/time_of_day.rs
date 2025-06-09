use cgmath::{Vector3, InnerSpace};

/// Time of day represented as hours (0-24)
#[derive(Debug, Clone, Copy)]
pub struct TimeOfDay {
    /// Current time in hours (0.0 - 24.0)
    pub hours: f32,
}

impl TimeOfDay {
    pub fn new(hours: f32) -> Self {
        Self {
            hours: hours % 24.0,
        }
    }
    
    /// Create noon time
    pub fn noon() -> Self {
        Self { hours: 12.0 }
    }
    
    /// Create midnight time
    pub fn midnight() -> Self {
        Self { hours: 0.0 }
    }
    
    /// Get the sun angle in radians (0 at sunrise, PI at sunset)
    pub fn sun_angle(&self) -> f32 {
        // Sun rises at 6:00 and sets at 18:00
        let day_progress = (self.hours - 6.0) / 12.0;
        day_progress.clamp(0.0, 1.0) * std::f32::consts::PI
    }
    
    /// Get the moon angle in radians
    pub fn moon_angle(&self) -> f32 {
        // Moon rises at 18:00 and sets at 6:00
        let night_progress = if self.hours >= 18.0 {
            (self.hours - 18.0) / 12.0
        } else {
            (self.hours + 6.0) / 12.0
        };
        night_progress.clamp(0.0, 1.0) * std::f32::consts::PI
    }
    
    /// Get sun direction vector
    pub fn sun_direction(&self) -> Vector3<f32> {
        let angle = self.sun_angle();
        Vector3::new(
            angle.cos(),
            angle.sin(),
            0.0,
        ).normalize()
    }
    
    /// Get moon direction vector  
    pub fn moon_direction(&self) -> Vector3<f32> {
        let angle = self.moon_angle();
        Vector3::new(
            angle.cos(),
            angle.sin(),
            0.0,
        ).normalize()
    }
    
    /// Get ambient light level (0.0 - 1.0)
    pub fn ambient_light(&self) -> f32 {
        // Maximum during day, minimum at night
        if self.hours >= 6.0 && self.hours <= 18.0 {
            // Daytime
            let day_progress = (self.hours - 6.0) / 12.0;
            let light = 1.0 - (2.0 * (day_progress - 0.5)).abs() * 0.2;
            light.max(0.8)
        } else {
            // Nighttime - much darker
            0.1
        }
    }
    
    /// Get sky color based on time
    pub fn sky_color(&self) -> [f32; 3] {
        if self.hours >= 6.0 && self.hours <= 18.0 {
            // Daytime
            if self.hours < 7.0 || self.hours > 17.0 {
                // Sunrise/sunset
                [0.9, 0.5, 0.3]
            } else {
                // Day sky
                [0.5, 0.8, 1.0]
            }
        } else {
            // Night sky
            [0.05, 0.05, 0.2]
        }
    }
    
    /// Get sun color based on time
    pub fn sun_color(&self) -> [f32; 3] {
        if self.hours < 7.0 || self.hours > 17.0 {
            // Sunrise/sunset - orange
            [1.0, 0.7, 0.4]
        } else {
            // Midday - white/yellow
            [1.0, 0.95, 0.8]
        }
    }
    
    /// Advance time by delta seconds
    pub fn advance(&mut self, delta_seconds: f32, day_length_seconds: f32) {
        // Convert delta to hours based on day length
        let hours_per_second = 24.0 / day_length_seconds;
        self.hours += delta_seconds * hours_per_second;
        
        // Wrap around at 24 hours
        while self.hours >= 24.0 {
            self.hours -= 24.0;
        }
    }
    
    /// Is it daytime?
    pub fn is_day(&self) -> bool {
        self.hours >= 6.0 && self.hours < 18.0
    }
    
    /// Is it nighttime?
    pub fn is_night(&self) -> bool {
        !self.is_day()
    }
}

/// Manages the day/night cycle
pub struct DayNightCycle {
    /// Current time of day
    pub time: TimeOfDay,
    /// Length of a full day in seconds
    pub day_length_seconds: f32,
    /// Speed multiplier for time progression
    pub time_scale: f32,
}

impl DayNightCycle {
    pub fn new(starting_time: TimeOfDay, day_length_seconds: f32) -> Self {
        Self {
            time: starting_time,
            day_length_seconds,
            time_scale: 1.0,
        }
    }
    
    /// Create with default settings (20 minute days, starting at noon)
    pub fn default() -> Self {
        Self::new(TimeOfDay::noon(), 20.0 * 60.0)
    }
    
    /// Update the time of day
    pub fn update(&mut self, delta_time: f32) {
        self.time.advance(delta_time * self.time_scale, self.day_length_seconds);
    }
    
    /// Get the current global light level (0-15)
    pub fn global_light_level(&self) -> u8 {
        if self.time.is_day() {
            15
        } else {
            // Night has reduced skylight
            4
        }
    }
    
    /// Set time scale (for debugging or gameplay features)
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.max(0.0);
    }
}