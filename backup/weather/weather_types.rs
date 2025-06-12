use serde::{Serialize, Deserialize};

/// Different types of weather
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeatherType {
    Clear,
    Cloudy,
    Rain,
    Snow,
    Thunderstorm,
    Fog,
    Sandstorm,
}

/// Weather intensity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeatherIntensity {
    None,
    Light,
    Moderate,
    Heavy,
    Extreme,
}

/// Current weather conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditions {
    pub weather_type: WeatherType,
    pub intensity: WeatherIntensity,
    pub temperature: f32,
    pub humidity: f32,
    pub wind_speed: f32,
    pub wind_direction: f32,
    pub visibility: f32,
    pub precipitation_rate: f32,
}

impl WeatherConditions {
    /// Create clear weather conditions
    pub fn clear() -> Self {
        Self {
            weather_type: WeatherType::Clear,
            intensity: WeatherIntensity::None,
            temperature: 20.0,
            humidity: 0.5,
            wind_speed: 5.0,
            wind_direction: 0.0,
            visibility: 1.0,
            precipitation_rate: 0.0,
        }
    }
    
    /// Create rainy weather
    pub fn rain(intensity: WeatherIntensity) -> Self {
        let (precip_rate, visibility) = match intensity {
            WeatherIntensity::Light => (0.1, 0.9),
            WeatherIntensity::Moderate => (0.3, 0.7),
            WeatherIntensity::Heavy => (0.6, 0.5),
            WeatherIntensity::Extreme => (1.0, 0.3),
            _ => (0.0, 1.0),
        };
        
        Self {
            weather_type: WeatherType::Rain,
            intensity,
            temperature: 15.0,
            humidity: 0.9,
            wind_speed: 15.0,
            wind_direction: 180.0,
            visibility,
            precipitation_rate: precip_rate,
        }
    }
    
    /// Create snowy weather
    pub fn snow(intensity: WeatherIntensity) -> Self {
        let (precip_rate, visibility) = match intensity {
            WeatherIntensity::Light => (0.05, 0.8),
            WeatherIntensity::Moderate => (0.15, 0.6),
            WeatherIntensity::Heavy => (0.3, 0.4),
            WeatherIntensity::Extreme => (0.5, 0.2),
            _ => (0.0, 1.0),
        };
        
        Self {
            weather_type: WeatherType::Snow,
            intensity,
            temperature: -5.0,
            humidity: 0.7,
            wind_speed: 10.0,
            wind_direction: 270.0,
            visibility,
            precipitation_rate: precip_rate,
        }
    }
    
    /// Create foggy weather
    pub fn fog(intensity: WeatherIntensity) -> Self {
        let visibility = match intensity {
            WeatherIntensity::Light => 0.7,
            WeatherIntensity::Moderate => 0.5,
            WeatherIntensity::Heavy => 0.3,
            WeatherIntensity::Extreme => 0.1,
            _ => 1.0,
        };
        
        Self {
            weather_type: WeatherType::Fog,
            intensity,
            temperature: 10.0,
            humidity: 0.95,
            wind_speed: 2.0,
            wind_direction: 90.0,
            visibility,
            precipitation_rate: 0.0,
        }
    }
    
    /// Create thunderstorm weather
    pub fn thunderstorm(intensity: WeatherIntensity) -> Self {
        let (precip_rate, visibility) = match intensity {
            WeatherIntensity::Light => (0.2, 0.8),
            WeatherIntensity::Moderate => (0.5, 0.6),
            WeatherIntensity::Heavy => (0.8, 0.4),
            WeatherIntensity::Extreme => (1.5, 0.2),
            _ => (0.0, 1.0),
        };
        
        Self {
            weather_type: WeatherType::Thunderstorm,
            intensity,
            temperature: 18.0,
            humidity: 0.95,
            wind_speed: 25.0,
            wind_direction: 225.0,
            visibility,
            precipitation_rate: precip_rate,
        }
    }
    
    /// Interpolate between two weather conditions
    pub fn interpolate(&self, other: &WeatherConditions, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        
        Self {
            weather_type: if t < 0.5 { self.weather_type } else { other.weather_type },
            intensity: if t < 0.5 { self.intensity } else { other.intensity },
            temperature: self.temperature + (other.temperature - self.temperature) * t,
            humidity: self.humidity + (other.humidity - self.humidity) * t,
            wind_speed: self.wind_speed + (other.wind_speed - self.wind_speed) * t,
            wind_direction: lerp_angle(self.wind_direction, other.wind_direction, t),
            visibility: self.visibility + (other.visibility - self.visibility) * t,
            precipitation_rate: self.precipitation_rate + (other.precipitation_rate - self.precipitation_rate) * t,
        }
    }
    
    /// Check if it's precipitating
    pub fn is_precipitating(&self) -> bool {
        matches!(self.weather_type, WeatherType::Rain | WeatherType::Snow | WeatherType::Thunderstorm)
            && self.precipitation_rate > 0.0
    }
    
    /// Get the appropriate particle type for this weather
    pub fn get_particle_type(&self) -> Option<ParticleType> {
        match self.weather_type {
            WeatherType::Rain | WeatherType::Thunderstorm => Some(ParticleType::Rain),
            WeatherType::Snow => Some(ParticleType::Snow),
            WeatherType::Sandstorm => Some(ParticleType::Sand),
            _ => None,
        }
    }
}

/// Particle types for weather effects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleType {
    Rain,
    Snow,
    Sand,
}

/// Lerp between two angles (handles wrapping)
fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut diff = b - a;
    
    // Wrap the difference to [-180, 180]
    while diff > 180.0 {
        diff -= 360.0;
    }
    while diff < -180.0 {
        diff += 360.0;
    }
    
    let result = a + diff * t;
    
    // Wrap result to [0, 360]
    if result < 0.0 {
        result + 360.0
    } else if result >= 360.0 {
        result - 360.0
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_weather_conditions() {
        let clear = WeatherConditions::clear();
        assert_eq!(clear.weather_type, WeatherType::Clear);
        assert_eq!(clear.precipitation_rate, 0.0);
        
        let rain = WeatherConditions::rain(WeatherIntensity::Heavy);
        assert_eq!(rain.weather_type, WeatherType::Rain);
        assert!(rain.precipitation_rate > 0.0);
        assert!(rain.is_precipitating());
    }
    
    #[test]
    fn test_weather_interpolation() {
        let clear = WeatherConditions::clear();
        let rain = WeatherConditions::rain(WeatherIntensity::Moderate);
        
        let mid = clear.interpolate(&rain, 0.5);
        // Clear weather is warmer than rain
        assert!(mid.temperature < clear.temperature && mid.temperature > rain.temperature);
        assert!(mid.humidity > clear.humidity && mid.humidity < rain.humidity);
    }
}