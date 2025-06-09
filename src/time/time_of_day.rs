use serde::{Serialize, Deserialize};

/// Time of day in 24-hour format
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimeOfDay {
    /// Hours (0-23)
    pub hours: u8,
    /// Minutes (0-59)
    pub minutes: u8,
    /// Seconds (0-59)
    pub seconds: f32,
}

/// Phases of the day
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DayPhase {
    Night,          // 00:00 - 04:00
    Dawn,           // 04:00 - 06:00
    EarlyMorning,   // 06:00 - 08:00
    Morning,        // 08:00 - 10:00
    LateMorning,    // 10:00 - 12:00
    Noon,           // 12:00 - 14:00
    Afternoon,      // 14:00 - 16:00
    LateAfternoon,  // 16:00 - 18:00
    Dusk,           // 18:00 - 20:00
    Evening,        // 20:00 - 22:00
    LateNight,      // 22:00 - 00:00
}

impl TimeOfDay {
    /// Create a new time of day
    pub fn new(hours: u8, minutes: u8, seconds: f32) -> Self {
        Self {
            hours: hours % 24,
            minutes: minutes % 60,
            seconds: seconds % 60.0,
        }
    }
    
    /// Create from total seconds since midnight
    pub fn from_seconds(total_seconds: f32) -> Self {
        let seconds_in_day = 86400.0;
        let normalized = total_seconds % seconds_in_day;
        
        let hours = (normalized / 3600.0) as u8;
        let minutes = ((normalized % 3600.0) / 60.0) as u8;
        let seconds = normalized % 60.0;
        
        Self::new(hours, minutes, seconds)
    }
    
    /// Convert to total seconds since midnight
    pub fn to_seconds(&self) -> f32 {
        self.hours as f32 * 3600.0 + self.minutes as f32 * 60.0 + self.seconds
    }
    
    /// Get the current phase of day
    pub fn phase(&self) -> DayPhase {
        match self.hours {
            0..=3 => DayPhase::Night,
            4..=5 => DayPhase::Dawn,
            6..=7 => DayPhase::EarlyMorning,
            8..=9 => DayPhase::Morning,
            10..=11 => DayPhase::LateMorning,
            12..=13 => DayPhase::Noon,
            14..=15 => DayPhase::Afternoon,
            16..=17 => DayPhase::LateAfternoon,
            18..=19 => DayPhase::Dusk,
            20..=21 => DayPhase::Evening,
            22..=23 => DayPhase::LateNight,
            _ => DayPhase::Night,
        }
    }
    
    /// Advance time by delta seconds
    pub fn advance(&mut self, delta_seconds: f32) {
        let total = self.to_seconds() + delta_seconds;
        *self = Self::from_seconds(total);
    }
    
    /// Get normalized time (0-1) where 0 is midnight and 1 is next midnight
    pub fn normalized(&self) -> f32 {
        self.to_seconds() / 86400.0
    }
    
    /// Get sun angle (0 at noon, PI at midnight)
    pub fn sun_angle(&self) -> f32 {
        let normalized = self.normalized();
        // Adjust so noon (0.5) = 0 radians, midnight (0.0 or 1.0) = PI
        ((normalized + 0.5) % 1.0) * std::f32::consts::TAU
    }
    
    /// Format as HH:MM:SS
    pub fn format(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hours, self.minutes, self.seconds as u8)
    }
    
    /// Format as 12-hour time with AM/PM
    pub fn format_12h(&self) -> String {
        let (hours_12, am_pm) = if self.hours == 0 {
            (12, "AM")
        } else if self.hours < 12 {
            (self.hours, "AM")
        } else if self.hours == 12 {
            (12, "PM")
        } else {
            (self.hours - 12, "PM")
        };
        
        format!("{:02}:{:02} {}", hours_12, self.minutes, am_pm)
    }
    
    /// Check if it's daytime (roughly 6 AM to 6 PM)
    pub fn is_daytime(&self) -> bool {
        self.hours >= 6 && self.hours < 18
    }
    
    /// Check if it's nighttime
    pub fn is_nighttime(&self) -> bool {
        !self.is_daytime()
    }
    
    /// Get brightness factor (0-1) based on time
    pub fn brightness(&self) -> f32 {
        match self.phase() {
            DayPhase::Night | DayPhase::LateNight => 0.0,
            DayPhase::Dawn => {
                // Interpolate from 0 to 0.5 during dawn
                let dawn_progress = (self.to_seconds() - 4.0 * 3600.0) / (2.0 * 3600.0);
                dawn_progress.clamp(0.0, 1.0) * 0.5
            },
            DayPhase::EarlyMorning => 0.5 + (self.to_seconds() - 6.0 * 3600.0) / (2.0 * 3600.0) * 0.3,
            DayPhase::Morning | DayPhase::LateMorning => 0.8 + (self.to_seconds() - 8.0 * 3600.0) / (4.0 * 3600.0) * 0.2,
            DayPhase::Noon => 1.0,
            DayPhase::Afternoon => 1.0 - (self.to_seconds() - 14.0 * 3600.0) / (2.0 * 3600.0) * 0.1,
            DayPhase::LateAfternoon => 0.9 - (self.to_seconds() - 16.0 * 3600.0) / (2.0 * 3600.0) * 0.2,
            DayPhase::Dusk => {
                // Interpolate from 0.7 to 0.2 during dusk
                let dusk_progress = (self.to_seconds() - 18.0 * 3600.0) / (2.0 * 3600.0);
                0.7 - dusk_progress.clamp(0.0, 1.0) * 0.5
            },
            DayPhase::Evening => 0.2 - (self.to_seconds() - 20.0 * 3600.0) / (2.0 * 3600.0) * 0.2,
        }
    }
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self::new(12, 0, 0.0) // Noon
    }
}

/// Time speed multiplier presets
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeSpeed {
    Paused,
    RealTime,   // 1:1 with real time
    Fast,       // 20:1 (1 day = 72 minutes)
    VeryFast,   // 60:1 (1 day = 24 minutes)
    UltraFast,  // 360:1 (1 day = 4 minutes)
    Custom(f32),
}

impl TimeSpeed {
    /// Get the multiplier value
    pub fn multiplier(&self) -> f32 {
        match self {
            TimeSpeed::Paused => 0.0,
            TimeSpeed::RealTime => 1.0,
            TimeSpeed::Fast => 20.0,
            TimeSpeed::VeryFast => 60.0,
            TimeSpeed::UltraFast => 360.0,
            TimeSpeed::Custom(m) => *m,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_time_of_day() {
        let mut time = TimeOfDay::new(12, 30, 0.0);
        assert_eq!(time.phase(), DayPhase::Noon);
        assert!(time.is_daytime());
        assert_eq!(time.format(), "12:30:00");
        assert_eq!(time.format_12h(), "12:30 PM");
        
        // Test advancement
        time.advance(3600.0); // Add 1 hour
        assert_eq!(time.hours, 13);
        assert_eq!(time.minutes, 30);
        
        // Test wrap around
        let mut night = TimeOfDay::new(23, 30, 0.0);
        night.advance(3600.0); // Add 1 hour
        assert_eq!(night.hours, 0);
        assert_eq!(night.minutes, 30);
    }
    
    #[test]
    fn test_brightness() {
        let noon = TimeOfDay::new(12, 0, 0.0);
        assert_eq!(noon.brightness(), 1.0);
        
        let night = TimeOfDay::new(0, 0, 0.0);
        assert_eq!(night.brightness(), 0.0);
        
        let dawn = TimeOfDay::new(5, 0, 0.0);
        assert!(dawn.brightness() > 0.0 && dawn.brightness() < 0.5);
    }
}