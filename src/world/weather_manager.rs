//! Weather management system for world generation
//!
//! This module provides weather integration for terrain generation,
//! managing weather zones, transitions, and biome-based weather patterns.

use crate::gpu::types::terrain::TerrainParams;
use crate::weather::*;
use crate::world::core::ChunkPos;

/// Weather zone information
#[derive(Debug, Clone, Copy)]
pub struct WeatherZone {
    /// Center position of the weather zone
    pub center: ChunkPos,
    /// Radius in chunks
    pub radius: u32,
    /// Weather type
    pub weather_type: u32,
    /// Weather intensity
    pub intensity: u32,
    /// Base temperature for this zone
    pub temperature: f32,
}

/// Weather manager for world generation
pub struct WeatherManager {
    /// Current global weather
    pub global_weather: u32,
    pub global_intensity: u32,
    pub base_temperature: f32,
    /// Active weather zones
    pub zones: Vec<WeatherZone>,
}

impl WeatherManager {
    /// Create a new weather manager with default clear weather
    pub fn new() -> Self {
        Self {
            global_weather: WEATHER_CLEAR,
            global_intensity: INTENSITY_NONE,
            base_temperature: 20.0, // 20°C default
            zones: Vec::new(),
        }
    }

    /// Set global weather conditions
    pub fn set_global_weather(&mut self, weather_type: u32, intensity: u32, temperature: f32) {
        self.global_weather = weather_type;
        self.global_intensity = intensity;
        self.base_temperature = temperature;
    }

    /// Add a localized weather zone
    pub fn add_zone(&mut self, zone: WeatherZone) {
        self.zones.push(zone);
    }

    /// Get weather parameters for a specific chunk position
    pub fn get_weather_at(&self, pos: ChunkPos) -> (u32, u32, f32) {
        // Check if position is within any weather zone
        for zone in &self.zones {
            let distance = Self::chunk_distance(pos, zone.center);
            if distance <= zone.radius as f32 {
                // Blend with global weather based on distance from center
                let blend_factor = 1.0 - (distance / zone.radius as f32);
                let temperature = self.base_temperature
                    + (zone.temperature - self.base_temperature) * blend_factor;

                return (zone.weather_type, zone.intensity, temperature);
            }
        }

        // Use global weather with altitude adjustment
        let altitude_temp_adjustment = -0.65 * (pos.y as f32 * 5.0); // -0.65°C per 100m
        let temperature = self.base_temperature + altitude_temp_adjustment;

        (self.global_weather, self.global_intensity, temperature)
    }

    /// Calculate distance between chunks
    fn chunk_distance(a: ChunkPos, b: ChunkPos) -> f32 {
        let dx = (a.x - b.x) as f32;
        let dy = (a.y - b.y) as f32;
        let dz = (a.z - b.z) as f32;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Update terrain parameters with weather for a specific chunk
    pub fn apply_weather_to_params(&self, params: &mut TerrainParams, chunk_pos: ChunkPos) {
        let (weather_type, intensity, temperature) = self.get_weather_at(chunk_pos);
        params.set_weather(weather_type, intensity);
        params.set_temperature_celsius(temperature);

        // Add weather-specific block distributions
        self.add_weather_distributions(params, weather_type, intensity, temperature);
    }

    /// Add weather-specific block distributions
    fn add_weather_distributions(
        &self,
        params: &mut TerrainParams,
        weather_type: u32,
        intensity: u32,
        temperature: f32,
    ) {
        use crate::blocks::{FROST, ICE, SNOW};
        use crate::gpu::types::terrain::BlockDistribution;

        // Ice formations in cold weather
        if temperature <= 0.0 {
            params.add_distribution(BlockDistribution {
                block_id: ICE as u32,
                min_height: (params.sea_level as i32) - 5,
                max_height: params.sea_level as i32 + 2,
                probability: 0.7,
                noise_threshold: 0.4,
                _padding: [0; 3],
            });
        }

        // Snow layers in snow weather
        if weather_type == WEATHER_SNOW || weather_type == WEATHER_BLIZZARD {
            let snow_probability = match intensity {
                INTENSITY_LIGHT => 0.3,
                INTENSITY_MEDIUM => 0.5,
                INTENSITY_HEAVY => 0.7,
                INTENSITY_EXTREME => 0.9,
                _ => 0.1,
            };

            params.add_distribution(BlockDistribution {
                block_id: SNOW as u32,
                min_height: crate::weather::SNOW_HEIGHT_TYPICAL_LOW - (intensity as i32 * 100), // Lower with intensity
                max_height: i32::MAX,
                probability: snow_probability,
                noise_threshold: 0.3,
                _padding: [0; 3],
            });
        }

        // Frost formations in extreme cold
        if temperature <= -10.0 {
            params.add_distribution(BlockDistribution {
                block_id: FROST as u32,
                min_height: i32::MIN,
                max_height: i32::MAX,
                probability: 0.2,
                noise_threshold: 0.6,
                _padding: [0; 3],
            });
        }
    }

    /// Generate biome-appropriate weather
    pub fn generate_biome_weather(&mut self, biome_type: &str) {
        match biome_type {
            "tundra" => {
                self.set_global_weather(WEATHER_SNOW, INTENSITY_LIGHT, -15.0);
            }
            "desert" => {
                self.set_global_weather(WEATHER_CLEAR, INTENSITY_NONE, 35.0);
            }
            "rainforest" => {
                self.set_global_weather(WEATHER_RAIN, INTENSITY_MEDIUM, 25.0);
            }
            "taiga" => {
                self.set_global_weather(WEATHER_SNOW, INTENSITY_MEDIUM, -5.0);
            }
            "temperate" => {
                self.set_global_weather(WEATHER_CLEAR, INTENSITY_NONE, 15.0);
            }
            _ => {
                // Default temperate weather
                self.set_global_weather(WEATHER_CLEAR, INTENSITY_NONE, 20.0);
            }
        }
    }
}

impl Default for WeatherManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_manager() {
        let mut manager = WeatherManager::new();

        // Test global weather
        manager.set_global_weather(WEATHER_RAIN, INTENSITY_HEAVY, 15.0);
        let (weather, intensity, temp) = manager.get_weather_at(ChunkPos::new(0, 0, 0));
        assert_eq!(weather, WEATHER_RAIN);
        assert_eq!(intensity, INTENSITY_HEAVY);
        assert_eq!(temp, 15.0);

        // Test altitude temperature adjustment
        let (_, _, high_temp) = manager.get_weather_at(ChunkPos::new(0, 20, 0)); // 100m up
        assert!(high_temp < 15.0); // Should be colder at altitude

        // Test weather zones
        manager.add_zone(WeatherZone {
            center: ChunkPos::new(10, 0, 10),
            radius: 5,
            weather_type: WEATHER_SNOW,
            intensity: INTENSITY_EXTREME,
            temperature: -20.0,
        });

        let (zone_weather, zone_intensity, zone_temp) =
            manager.get_weather_at(ChunkPos::new(10, 0, 10));
        assert_eq!(zone_weather, WEATHER_SNOW);
        assert_eq!(zone_intensity, INTENSITY_EXTREME);
        assert_eq!(zone_temp, -20.0);
    }
}
