#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_gpu::weather_gpu::{WeatherData, WeatherTransition, PrecipitationParticle};
    use crate::world_gpu::weather_migration::init_weather_for_biome;
    use crate::weather::BiomeType;
    
    #[test]
    fn test_weather_data_size() {
        // Ensure our weather data structures are properly sized and aligned
        assert_eq!(std::mem::size_of::<WeatherData>(), 16);
        assert_eq!(std::mem::align_of::<WeatherData>(), 4);
        
        assert_eq!(std::mem::size_of::<WeatherTransition>(), 40);
        assert_eq!(std::mem::align_of::<WeatherTransition>(), 4);
        
        assert_eq!(std::mem::size_of::<PrecipitationParticle>(), 48);
        assert_eq!(std::mem::align_of::<PrecipitationParticle>(), 4);
    }
    
    #[test]
    fn test_weather_initialization() {
        // Test that each biome gets appropriate weather
        let desert = init_weather_for_biome(BiomeType::Desert);
        assert_eq!(desert.temperature, 350); // 35°C
        assert_eq!(desert.humidity, 1000); // 10%
        assert_eq!(desert.precipitation_rate, 0);
        
        let tundra = init_weather_for_biome(BiomeType::Tundra);
        assert_eq!(tundra.temperature, -50); // -5°C
        assert!(tundra.humidity > 5000); // > 50%
        assert!(tundra.precipitation_rate > 0);
    }
    
    #[test]
    fn test_weather_type_packing() {
        use crate::world_gpu::weather_migration::convert_weather_type;
        use crate::weather::{WeatherType, WeatherIntensity};
        
        let packed = convert_weather_type(WeatherType::Rain, WeatherIntensity::Heavy);
        assert_eq!(packed & 0xFF, 2); // Rain = 2
        assert_eq!((packed >> 8) & 0xFF, 3); // Heavy = 3
        
        let packed = convert_weather_type(WeatherType::Thunderstorm, WeatherIntensity::Extreme);
        assert_eq!(packed & 0xFF, 4); // Thunderstorm = 4
        assert_eq!((packed >> 8) & 0xFF, 4); // Extreme = 4
    }
    
    #[test]
    fn test_clear_weather() {
        let clear = WeatherData::clear();
        assert_eq!(clear.weather_type_intensity, 0);
        assert_eq!(clear.temperature, 200); // 20°C
        assert_eq!(clear.visibility, 1000); // Full visibility
        assert_eq!(clear.precipitation_rate, 0);
    }
}