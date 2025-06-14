/// Data-Oriented Time of Day Functions
/// 
/// Pure functions for time calculations. No methods, no self, just data transformations.
/// Follows DOP principles from Sprint 37.

use cgmath::{Vector3, InnerSpace};

/// Time of day data (DOP - no methods)
/// Pure data structure for time state
#[derive(Debug, Clone, Copy)]
pub struct TimeOfDayData {
    /// Current time in hours (0.0 - 24.0)
    pub hours: f32,
}

/// Create new time of day data
/// Pure function - returns data structure, no behavior
pub fn create_time_of_day(hours: f32) -> TimeOfDayData {
    TimeOfDayData {
        hours: hours % 24.0,
    }
}

/// Create noon time
/// Pure function - constant time data
pub fn noon_time() -> TimeOfDayData {
    TimeOfDayData { hours: 12.0 }
}

/// Create midnight time
/// Pure function - constant time data
pub fn midnight_time() -> TimeOfDayData {
    TimeOfDayData { hours: 0.0 }
}

/// Get the sun angle in radians (0 at sunrise, PI at sunset)
/// Pure function - calculates sun position based on time data
pub fn calculate_sun_angle(time: &TimeOfDayData) -> f32 {
    // Sun rises at 6:00 and sets at 18:00
    let day_progress = (time.hours - 6.0) / 12.0;
    day_progress.clamp(0.0, 1.0) * std::f32::consts::PI
}

/// Get the moon angle in radians
/// Pure function - calculates moon position based on time data
pub fn calculate_moon_angle(time: &TimeOfDayData) -> f32 {
    // Moon rises at 18:00 and sets at 6:00
    let night_progress = if time.hours >= 18.0 {
        (time.hours - 18.0) / 12.0
    } else {
        (time.hours + 6.0) / 12.0
    };
    night_progress.clamp(0.0, 1.0) * std::f32::consts::PI
}

/// Get sun direction vector
/// Pure function - transforms time data to 3D sun direction
pub fn calculate_sun_direction(time: &TimeOfDayData) -> Vector3<f32> {
    let angle = calculate_sun_angle(time);
    Vector3::new(
        angle.cos(),
        angle.sin(),
        0.0,
    ).normalize()
}

/// Get moon direction vector  
/// Pure function - transforms time data to 3D moon direction
pub fn calculate_moon_direction(time: &TimeOfDayData) -> Vector3<f32> {
    let angle = calculate_moon_angle(time);
    Vector3::new(
        angle.cos(),
        angle.sin(),
        0.0,
    ).normalize()
}

/// Get ambient light level (0.0 - 1.0)
/// Pure function - calculates ambient lighting based on time data
pub fn calculate_ambient_light(time: &TimeOfDayData) -> f32 {
    // Maximum during day, minimum at night
    if time.hours >= 6.0 && time.hours <= 18.0 {
        // Daytime
        let day_progress = (time.hours - 6.0) / 12.0;
        let light = 1.0 - (2.0 * (day_progress - 0.5)).abs() * 0.2;
        light.max(0.8)
    } else {
        // Nighttime - much darker
        0.1
    }
}

/// Get sky color based on time
/// Pure function - calculates sky color from time data
pub fn calculate_sky_color(time: &TimeOfDayData) -> [f32; 3] {
    if time.hours >= 6.0 && time.hours <= 18.0 {
        // Daytime
        if time.hours < 7.0 || time.hours > 17.0 {
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
/// Pure function - calculates sun color from time data
pub fn calculate_sun_color(time: &TimeOfDayData) -> [f32; 3] {
    if time.hours < 7.0 || time.hours > 17.0 {
        // Sunrise/sunset - orange
        [1.0, 0.7, 0.4]
    } else {
        // Midday - white/yellow
        [1.0, 0.95, 0.8]
    }
}

/// Advance time by delta seconds
/// Function - transforms time data by advancing it
pub fn advance_time(time: &mut TimeOfDayData, delta_seconds: f32, day_length_seconds: f32) {
    // Convert delta to hours based on day length
    let hours_per_second = 24.0 / day_length_seconds;
    time.hours += delta_seconds * hours_per_second;
    
    // Wrap around at 24 hours
    while time.hours >= 24.0 {
        time.hours -= 24.0;
    }
}

/// Is it daytime?
/// Pure function - determines day/night from time data
pub fn is_day_time(time: &TimeOfDayData) -> bool {
    time.hours >= 6.0 && time.hours < 18.0
}

/// Is it nighttime?
/// Pure function - determines day/night from time data
pub fn is_night_time(time: &TimeOfDayData) -> bool {
    !is_day_time(time)
}

/// Day/night cycle data (DOP - no methods)
/// Pure data structure for managing time progression
pub struct DayNightCycleData {
    /// Current time of day
    pub time: TimeOfDayData,
    /// Length of a full day in seconds
    pub day_length_seconds: f32,
    /// Speed multiplier for time progression
    pub time_scale: f32,
}

/// Create new day/night cycle data
/// Pure function - returns data structure, no behavior
pub fn create_day_night_cycle(starting_time: TimeOfDayData, day_length_seconds: f32) -> DayNightCycleData {
    DayNightCycleData {
        time: starting_time,
        day_length_seconds,
        time_scale: 1.0,
    }
}

/// Create with default settings (20 minute days, starting at noon)
/// Pure function - returns default cycle configuration
pub fn create_default_day_night_cycle() -> DayNightCycleData {
    create_day_night_cycle(noon_time(), 20.0 * 60.0)
}

/// Update the time of day
/// Function - transforms cycle data by advancing time
pub fn update_day_night_cycle(cycle: &mut DayNightCycleData, delta_time: f32) {
    advance_time(&mut cycle.time, delta_time * cycle.time_scale, cycle.day_length_seconds);
}

/// Get the current global light level (0-15)
/// Pure function - calculates light level from cycle data
pub fn calculate_global_light_level(cycle: &DayNightCycleData) -> u8 {
    if is_day_time(&cycle.time) {
        15
    } else {
        // Night has reduced skylight
        4
    }
}

/// Set time scale (for debugging or gameplay features)
/// Function - transforms cycle time scale
pub fn set_time_scale(cycle: &mut DayNightCycleData, scale: f32) {
    cycle.time_scale = scale.max(0.0);
}

// ===== COMPATIBILITY LAYER =====
// Temporary aliases for code that hasn't been converted yet

