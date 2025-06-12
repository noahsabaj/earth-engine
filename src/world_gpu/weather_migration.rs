/// Migration helper for converting from CPU weather system to GPU weather system

use crate::weather::{BiomeType, WeatherType, WeatherIntensity};
use crate::world_gpu::weather_gpu::{WeatherData, WeatherTransition, WeatherConfig};

/// Convert legacy weather types to GPU format
pub fn convert_weather_type(weather_type: WeatherType, intensity: WeatherIntensity) -> u32 {
    let type_bits = match weather_type {
        WeatherType::Clear => 0,
        WeatherType::Cloudy => 1,
        WeatherType::Rain => 2,
        WeatherType::Snow => 3,
        WeatherType::Thunderstorm => 4,
        WeatherType::Fog => 5,
        WeatherType::Sandstorm => 6,
    };
    
    let intensity_bits = match intensity {
        WeatherIntensity::None => 0,
        WeatherIntensity::Light => 1,
        WeatherIntensity::Moderate => 2,
        WeatherIntensity::Heavy => 3,
        WeatherIntensity::Extreme => 4,
    };
    
    type_bits | (intensity_bits << 8)
}

/// Convert biome type to GPU format
pub fn convert_biome_type(biome: BiomeType) -> u32 {
    biome.to_u32()
}

/// Initialize weather data for a region
pub fn init_weather_for_biome(biome: BiomeType) -> WeatherData {
    match biome {
        BiomeType::Plains => WeatherData {
            weather_type_intensity: 0, // Clear
            temperature: 200, // 20°C
            humidity: 5000, // 50%
            wind_speed: 50, // 5 m/s
            wind_direction: 0,
            visibility: 1000,
            precipitation_rate: 0,
        },
        BiomeType::Desert => WeatherData {
            weather_type_intensity: 0, // Clear
            temperature: 350, // 35°C
            humidity: 1000, // 10%
            wind_speed: 150, // 15 m/s
            wind_direction: 45,
            visibility: 1000,
            precipitation_rate: 0,
        },
        BiomeType::Tundra => WeatherData {
            weather_type_intensity: convert_weather_type(WeatherType::Snow, WeatherIntensity::Light),
            temperature: -50, // -5°C
            humidity: 7000, // 70%
            wind_speed: 100, // 10 m/s
            wind_direction: 270,
            visibility: 800,
            precipitation_rate: 50,
        },
        BiomeType::Forest => WeatherData {
            weather_type_intensity: convert_weather_type(WeatherType::Rain, WeatherIntensity::Light),
            temperature: 180, // 18°C
            humidity: 6000, // 60%
            wind_speed: 50, // 5 m/s
            wind_direction: 180,
            visibility: 900,
            precipitation_rate: 100,
        },
        BiomeType::Mountain => WeatherData {
            weather_type_intensity: convert_weather_type(WeatherType::Fog, WeatherIntensity::Moderate),
            temperature: 100, // 10°C
            humidity: 9500, // 95%
            wind_speed: 20, // 2 m/s
            wind_direction: 90,
            visibility: 500,
            precipitation_rate: 0,
        },
        BiomeType::Swamp => WeatherData {
            weather_type_intensity: convert_weather_type(WeatherType::Fog, WeatherIntensity::Heavy),
            temperature: 150, // 15°C
            humidity: 9500, // 95%
            wind_speed: 20, // 2 m/s
            wind_direction: 90,
            visibility: 300,
            precipitation_rate: 0,
        },
        BiomeType::Ocean => WeatherData {
            weather_type_intensity: convert_weather_type(WeatherType::Cloudy, WeatherIntensity::Light),
            temperature: 180, // 18°C
            humidity: 8000, // 80%
            wind_speed: 200, // 20 m/s
            wind_direction: 270,
            visibility: 900,
            precipitation_rate: 0,
        },
    }
}

/// Create initial weather transition for a region
pub fn create_weather_transition(biome: BiomeType) -> WeatherTransition {
    let weather = init_weather_for_biome(biome);
    WeatherTransition {
        current: weather,
        target: weather,
        progress: 65535, // Fully transitioned
        speed: 10,
        change_timer: 18000, // 5 minutes at 60 FPS
        biome_type: convert_biome_type(biome),
    }
}

/// Create default weather config
pub fn create_default_weather_config(player_pos: [f32; 3]) -> WeatherConfig {
    WeatherConfig {
        frame_number: 0,
        delta_time_ms: 16, // 60 FPS
        player_position: player_pos,
        precipitation_radius: 200.0,
        max_particles: 100000,
        particle_count: 0,
        random_seed: 42,
        flags: 0xFFFFFFFF, // All features enabled
    }
}