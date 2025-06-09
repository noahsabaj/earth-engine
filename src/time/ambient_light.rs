use glam::Vec3;
use serde::{Serialize, Deserialize};

/// Ambient light settings for different times of day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientLightSettings {
    /// Ambient light color
    pub color: Vec3,
    /// Ambient light intensity
    pub intensity: f32,
    /// Directional light color (sun/moon)
    pub directional_color: Vec3,
    /// Directional light intensity
    pub directional_intensity: f32,
    /// Sky light contribution
    pub sky_contribution: f32,
    /// Shadow intensity (0 = no shadows, 1 = black shadows)
    pub shadow_intensity: f32,
}

impl AmbientLightSettings {
    /// Create settings for bright daylight
    pub fn daylight() -> Self {
        Self {
            color: Vec3::new(0.7, 0.8, 1.0), // Slightly blue ambient
            intensity: 0.4,
            directional_color: Vec3::new(1.0, 0.95, 0.8), // Warm sunlight
            directional_intensity: 1.0,
            sky_contribution: 0.3,
            shadow_intensity: 0.7,
        }
    }
    
    /// Create settings for sunrise
    pub fn sunrise() -> Self {
        Self {
            color: Vec3::new(0.9, 0.6, 0.4), // Warm orange ambient
            intensity: 0.3,
            directional_color: Vec3::new(1.0, 0.7, 0.4), // Orange sunlight
            directional_intensity: 0.6,
            sky_contribution: 0.4,
            shadow_intensity: 0.5,
        }
    }
    
    /// Create settings for sunset
    pub fn sunset() -> Self {
        Self {
            color: Vec3::new(0.8, 0.5, 0.4), // Red-orange ambient
            intensity: 0.25,
            directional_color: Vec3::new(1.0, 0.5, 0.3), // Red sunlight
            directional_intensity: 0.5,
            sky_contribution: 0.4,
            shadow_intensity: 0.6,
        }
    }
    
    /// Create settings for night
    pub fn night(moon_brightness: f32) -> Self {
        Self {
            color: Vec3::new(0.1, 0.1, 0.2), // Dark blue ambient
            intensity: 0.05 + moon_brightness * 0.05,
            directional_color: Vec3::new(0.7, 0.7, 0.9), // Cool moonlight
            directional_intensity: moon_brightness * 0.3,
            sky_contribution: 0.1,
            shadow_intensity: 0.9,
        }
    }
    
    /// Create settings for overcast day
    pub fn overcast() -> Self {
        Self {
            color: Vec3::new(0.6, 0.6, 0.7), // Gray ambient
            intensity: 0.5,
            directional_color: Vec3::new(0.8, 0.8, 0.9), // Diffuse light
            directional_intensity: 0.4,
            sky_contribution: 0.6,
            shadow_intensity: 0.3,
        }
    }
    
    /// Interpolate between two lighting settings
    pub fn interpolate(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        
        Self {
            color: self.color.lerp(other.color, t),
            intensity: self.intensity + (other.intensity - self.intensity) * t,
            directional_color: self.directional_color.lerp(other.directional_color, t),
            directional_intensity: self.directional_intensity + (other.directional_intensity - self.directional_intensity) * t,
            sky_contribution: self.sky_contribution + (other.sky_contribution - self.sky_contribution) * t,
            shadow_intensity: self.shadow_intensity + (other.shadow_intensity - self.shadow_intensity) * t,
        }
    }
    
    /// Apply weather modifications
    pub fn apply_weather(&mut self, weather_visibility: f32, is_thunderstorm: bool) {
        // Reduce intensity based on visibility
        self.intensity *= weather_visibility;
        self.directional_intensity *= weather_visibility.powf(0.5); // Less reduction for directional
        
        // Make lighting more gray in bad weather
        let gray_factor = 1.0 - weather_visibility;
        let gray = (self.color.x + self.color.y + self.color.z) / 3.0;
        self.color = self.color.lerp(Vec3::splat(gray), gray_factor * 0.5);
        
        // Reduce shadows in poor visibility
        self.shadow_intensity *= weather_visibility;
        
        // Thunder flash
        if is_thunderstorm {
            self.intensity = (self.intensity + 0.5).min(1.0);
            self.directional_intensity = (self.directional_intensity + 0.3).min(1.0);
        }
    }
}

/// Calculate ambient light based on time and conditions
pub fn calculate_ambient_light(
    sun_elevation: f32,
    moon_brightness: f32,
    normalized_time: f32,
    weather_visibility: f32,
    is_thunderstorm: bool,
) -> AmbientLightSettings {
    let mut settings = if sun_elevation > 0.3 {
        // Full daylight
        AmbientLightSettings::daylight()
    } else if sun_elevation > 0.0 {
        // Near sunrise/sunset
        let is_morning = normalized_time < 0.5;
        let transition_settings = if is_morning {
            AmbientLightSettings::sunrise()
        } else {
            AmbientLightSettings::sunset()
        };
        
        // Interpolate with daylight
        let t = sun_elevation / 0.3;
        transition_settings.interpolate(&AmbientLightSettings::daylight(), t)
    } else if sun_elevation > -0.2 {
        // Twilight
        let night_settings = AmbientLightSettings::night(moon_brightness);
        let twilight_settings = if normalized_time < 0.5 {
            AmbientLightSettings::sunrise()
        } else {
            AmbientLightSettings::sunset()
        };
        
        // Interpolate between twilight and night
        let t = (sun_elevation + 0.2) / 0.2;
        night_settings.interpolate(&twilight_settings, t)
    } else {
        // Full night
        AmbientLightSettings::night(moon_brightness)
    };
    
    // Apply weather effects
    settings.apply_weather(weather_visibility, is_thunderstorm);
    
    settings
}

/// Light source types in the world
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightSourceType {
    Torch,
    Lantern,
    Campfire,
    Lava,
    Glowstone,
    Lightning,
}

impl LightSourceType {
    /// Get light color for this source
    pub fn color(&self) -> Vec3 {
        match self {
            LightSourceType::Torch => Vec3::new(1.0, 0.7, 0.3),     // Warm orange
            LightSourceType::Lantern => Vec3::new(1.0, 0.9, 0.6),   // Warm yellow
            LightSourceType::Campfire => Vec3::new(1.0, 0.5, 0.2),  // Deep orange
            LightSourceType::Lava => Vec3::new(1.0, 0.3, 0.1),      // Red-orange
            LightSourceType::Glowstone => Vec3::new(0.9, 1.0, 0.8), // Cool white
            LightSourceType::Lightning => Vec3::new(0.8, 0.8, 1.0),  // Blue-white
        }
    }
    
    /// Get light intensity
    pub fn intensity(&self) -> f32 {
        match self {
            LightSourceType::Torch => 0.8,
            LightSourceType::Lantern => 1.0,
            LightSourceType::Campfire => 1.2,
            LightSourceType::Lava => 1.5,
            LightSourceType::Glowstone => 1.0,
            LightSourceType::Lightning => 10.0,
        }
    }
    
    /// Get light radius in blocks
    pub fn radius(&self) -> f32 {
        match self {
            LightSourceType::Torch => 12.0,
            LightSourceType::Lantern => 15.0,
            LightSourceType::Campfire => 15.0,
            LightSourceType::Lava => 15.0,
            LightSourceType::Glowstone => 15.0,
            LightSourceType::Lightning => 50.0,
        }
    }
    
    /// Check if this light flickers
    pub fn flickers(&self) -> bool {
        matches!(self, LightSourceType::Torch | LightSourceType::Campfire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ambient_light_interpolation() {
        let day = AmbientLightSettings::daylight();
        let night = AmbientLightSettings::night(0.5);
        
        let mid = day.interpolate(&night, 0.5);
        assert!(mid.intensity > night.intensity && mid.intensity < day.intensity);
    }
    
    #[test]
    fn test_weather_effects() {
        let mut settings = AmbientLightSettings::daylight();
        let original_intensity = settings.intensity;
        
        settings.apply_weather(0.5, false);
        assert!(settings.intensity < original_intensity);
    }
}