use glam::Vec3;
use rand::Rng;
use serde::{Serialize, Deserialize};
use std::time::Duration;

use crate::weather::{
    WeatherType, WeatherIntensity, WeatherConditions,
    PrecipitationSystem, PrecipitationType,
    FogSettings, FogDensity,
    WindSystem, WindDirection, WindStrength,
};

/// Weather system update event
#[derive(Debug, Clone)]
pub struct WeatherUpdate {
    pub conditions: WeatherConditions,
    pub transition_progress: f32,
}

/// Main weather system that manages all weather effects
pub struct WeatherSystem {
    /// Current weather conditions
    current_conditions: WeatherConditions,
    /// Target weather conditions (for transitions)
    target_conditions: WeatherConditions,
    /// Transition progress (0-1)
    transition_progress: f32,
    /// Transition duration
    transition_duration: f32,
    /// Time until next weather change
    time_until_change: f32,
    /// Weather change frequency
    change_frequency: (f32, f32), // min, max seconds
    /// Precipitation system
    precipitation: PrecipitationSystem,
    /// Wind system
    wind: WindSystem,
    /// Current biome (affects weather patterns)
    current_biome: BiomeType,
    /// Thunder system
    thunder_timer: f32,
    thunder_active: bool,
}

/// Biome types that affect weather
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiomeType {
    Plains,
    Forest,
    Desert,
    Tundra,
    Mountain,
    Swamp,
    Ocean,
}

impl WeatherSystem {
    /// Create a new weather system
    pub fn new(initial_biome: BiomeType) -> Self {
        let initial_conditions = Self::get_biome_weather(initial_biome);
        
        Self {
            current_conditions: initial_conditions.clone(),
            target_conditions: initial_conditions,
            transition_progress: 1.0,
            transition_duration: 60.0, // 1 minute transitions
            time_until_change: 300.0, // 5 minutes initial
            change_frequency: (180.0, 600.0), // 3-10 minutes
            precipitation: PrecipitationSystem::new(Vec3::ZERO, 200.0, (0.0, 300.0)),
            wind: WindSystem::new(),
            current_biome: initial_biome,
            thunder_timer: 0.0,
            thunder_active: false,
        }
    }
    
    /// Update the weather system
    pub fn update(&mut self, dt: f32, player_pos: Vec3) -> WeatherUpdate {
        // Update transition
        if self.transition_progress < 1.0 {
            self.transition_progress += dt / self.transition_duration;
            self.transition_progress = self.transition_progress.min(1.0);
            
            // Interpolate conditions
            self.current_conditions = self.current_conditions.interpolate(
                &self.target_conditions,
                self.transition_progress
            );
        }
        
        // Check for weather change
        self.time_until_change -= dt;
        if self.time_until_change <= 0.0 {
            self.change_weather();
        }
        
        // Update wind
        let wind_dir = WindDirection::from_angle(self.current_conditions.wind_direction);
        let wind_strength = WindStrength::from_speed(self.current_conditions.wind_speed);
        self.wind.set_wind(wind_dir, wind_strength);
        self.wind.update(dt);
        
        // Update precipitation
        if self.current_conditions.is_precipitating() {
            let precip_type = match self.current_conditions.weather_type {
                WeatherType::Snow => PrecipitationType::Snow,
                WeatherType::Thunderstorm => PrecipitationType::Rain,
                _ => PrecipitationType::Rain,
            };
            
            self.precipitation.update_bounds(player_pos, 200.0);
            self.precipitation.set_wind(self.wind.get_wind_velocity(player_pos));
            self.precipitation.update(dt, precip_type, self.current_conditions.precipitation_rate);
        } else {
            self.precipitation.clear();
        }
        
        // Update thunder
        if self.current_conditions.weather_type == WeatherType::Thunderstorm {
            self.update_thunder(dt);
        } else {
            self.thunder_active = false;
        }
        
        WeatherUpdate {
            conditions: self.current_conditions.clone(),
            transition_progress: self.transition_progress,
        }
    }
    
    /// Force a weather change
    pub fn change_weather(&mut self) {
        let mut rng = rand::thread_rng();
        
        // Choose new weather based on biome
        let weather_options = Self::get_biome_weather_options(self.current_biome);
        let new_weather = weather_options[rng.gen_range(0..weather_options.len())].clone();
        
        // Set up transition
        self.target_conditions = new_weather;
        self.transition_progress = 0.0;
        
        // Schedule next change
        self.time_until_change = rng.gen_range(self.change_frequency.0..self.change_frequency.1);
    }
    
    /// Set the current biome
    pub fn set_biome(&mut self, biome: BiomeType) {
        if self.current_biome != biome {
            self.current_biome = biome;
            // Trigger weather change on biome transition
            self.time_until_change = self.time_until_change.min(30.0);
        }
    }
    
    /// Get default weather for a biome
    fn get_biome_weather(biome: BiomeType) -> WeatherConditions {
        match biome {
            BiomeType::Plains => WeatherConditions::clear(),
            BiomeType::Forest => WeatherConditions::rain(WeatherIntensity::Light),
            BiomeType::Desert => WeatherConditions {
                weather_type: WeatherType::Clear,
                intensity: WeatherIntensity::None,
                temperature: 35.0,
                humidity: 0.1,
                wind_speed: 15.0,
                wind_direction: 45.0,
                visibility: 1.0,
                precipitation_rate: 0.0,
            },
            BiomeType::Tundra => WeatherConditions::snow(WeatherIntensity::Light),
            BiomeType::Mountain => WeatherConditions::fog(WeatherIntensity::Moderate),
            BiomeType::Swamp => WeatherConditions::fog(WeatherIntensity::Heavy),
            BiomeType::Ocean => WeatherConditions {
                weather_type: WeatherType::Cloudy,
                intensity: WeatherIntensity::Light,
                temperature: 18.0,
                humidity: 0.8,
                wind_speed: 20.0,
                wind_direction: 270.0,
                visibility: 0.9,
                precipitation_rate: 0.0,
            },
        }
    }
    
    /// Get possible weather conditions for a biome
    fn get_biome_weather_options(biome: BiomeType) -> Vec<WeatherConditions> {
        match biome {
            BiomeType::Plains => vec![
                WeatherConditions::clear(),
                WeatherConditions::clear(),
                WeatherConditions::rain(WeatherIntensity::Light),
                WeatherConditions::rain(WeatherIntensity::Moderate),
                WeatherConditions::thunderstorm(WeatherIntensity::Moderate),
            ],
            BiomeType::Forest => vec![
                WeatherConditions::clear(),
                WeatherConditions::fog(WeatherIntensity::Light),
                WeatherConditions::rain(WeatherIntensity::Light),
                WeatherConditions::rain(WeatherIntensity::Moderate),
                WeatherConditions::thunderstorm(WeatherIntensity::Light),
            ],
            BiomeType::Desert => vec![
                WeatherConditions::clear(),
                WeatherConditions::clear(),
                WeatherConditions::clear(),
                WeatherConditions {
                    weather_type: WeatherType::Sandstorm,
                    intensity: WeatherIntensity::Moderate,
                    temperature: 40.0,
                    humidity: 0.05,
                    wind_speed: 30.0,
                    wind_direction: 90.0,
                    visibility: 0.3,
                    precipitation_rate: 0.0,
                },
            ],
            BiomeType::Tundra => vec![
                WeatherConditions::clear(),
                WeatherConditions::snow(WeatherIntensity::Light),
                WeatherConditions::snow(WeatherIntensity::Moderate),
                WeatherConditions::snow(WeatherIntensity::Heavy),
                WeatherConditions::fog(WeatherIntensity::Heavy),
            ],
            BiomeType::Mountain => vec![
                WeatherConditions::clear(),
                WeatherConditions::fog(WeatherIntensity::Moderate),
                WeatherConditions::fog(WeatherIntensity::Heavy),
                WeatherConditions::snow(WeatherIntensity::Light),
                WeatherConditions::snow(WeatherIntensity::Heavy),
            ],
            BiomeType::Swamp => vec![
                WeatherConditions::fog(WeatherIntensity::Moderate),
                WeatherConditions::fog(WeatherIntensity::Heavy),
                WeatherConditions::rain(WeatherIntensity::Light),
                WeatherConditions::thunderstorm(WeatherIntensity::Heavy),
            ],
            BiomeType::Ocean => vec![
                WeatherConditions::clear(),
                WeatherConditions::rain(WeatherIntensity::Light),
                WeatherConditions::rain(WeatherIntensity::Heavy),
                WeatherConditions::thunderstorm(WeatherIntensity::Moderate),
                WeatherConditions::thunderstorm(WeatherIntensity::Extreme),
            ],
        }
    }
    
    /// Update thunder effects
    fn update_thunder(&mut self, dt: f32) {
        self.thunder_timer -= dt;
        
        if self.thunder_timer <= 0.0 {
            let mut rng = rand::thread_rng();
            
            // Chance of thunder based on intensity
            let thunder_chance = match self.current_conditions.intensity {
                WeatherIntensity::Light => 0.1,
                WeatherIntensity::Moderate => 0.3,
                WeatherIntensity::Heavy => 0.5,
                WeatherIntensity::Extreme => 0.8,
                _ => 0.0,
            };
            
            if rng.gen::<f32>() < thunder_chance {
                self.thunder_active = true;
                self.thunder_timer = rng.gen_range(5.0..20.0);
            } else {
                self.thunder_timer = rng.gen_range(1.0..5.0);
            }
        }
        
        // Thunder flash duration
        if self.thunder_active && self.thunder_timer < 19.8 {
            self.thunder_active = false;
        }
    }
    
    /// Get current fog settings
    pub fn get_fog_settings(&self) -> FogSettings {
        match self.current_conditions.weather_type {
            WeatherType::Fog => FogSettings::from_density(
                match self.current_conditions.intensity {
                    WeatherIntensity::Light => FogDensity::Light,
                    WeatherIntensity::Moderate => FogDensity::Medium,
                    WeatherIntensity::Heavy => FogDensity::Heavy,
                    WeatherIntensity::Extreme => FogDensity::VeryHeavy,
                    _ => FogDensity::None,
                }
            ),
            WeatherType::Rain | WeatherType::Thunderstorm => {
                // Rain reduces visibility
                let density = match self.current_conditions.intensity {
                    WeatherIntensity::Heavy | WeatherIntensity::Extreme => FogDensity::Light,
                    _ => FogDensity::None,
                };
                FogSettings::from_density(density)
            },
            _ => FogSettings::from_density(FogDensity::None),
        }
    }
    
    /// Get precipitation particles
    pub fn get_precipitation_particles(&self) -> &[crate::weather::PrecipitationParticle] {
        self.precipitation.get_particles()
    }
    
    /// Get wind velocity at a position
    pub fn get_wind_at(&self, pos: Vec3) -> Vec3 {
        self.wind.get_wind_velocity(pos)
    }
    
    /// Check if thunder is active
    pub fn is_thunder_active(&self) -> bool {
        self.thunder_active
    }
    
    /// Get current weather conditions
    pub fn get_conditions(&self) -> &WeatherConditions {
        &self.current_conditions
    }
    
    /// Set weather change frequency
    pub fn set_change_frequency(&mut self, min_seconds: f32, max_seconds: f32) {
        self.change_frequency = (min_seconds.max(10.0), max_seconds.max(min_seconds + 10.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_weather_system() {
        let mut weather = WeatherSystem::new(BiomeType::Plains);
        
        // Update should work without panicking
        let update = weather.update(0.1, Vec3::ZERO);
        assert_eq!(update.conditions.weather_type, WeatherType::Clear);
        
        // Force weather change
        weather.change_weather();
        assert_eq!(weather.transition_progress, 0.0);
        
        // Update during transition
        for _ in 0..100 {
            weather.update(1.0, Vec3::ZERO);
        }
        assert_eq!(weather.transition_progress, 1.0);
    }
    
    #[test]
    fn test_biome_weather() {
        let desert_weather = WeatherSystem::get_biome_weather(BiomeType::Desert);
        assert!(desert_weather.temperature > 30.0);
        assert!(desert_weather.humidity < 0.2);
        
        let tundra_weather = WeatherSystem::get_biome_weather(BiomeType::Tundra);
        assert!(tundra_weather.temperature < 0.0);
    }
}