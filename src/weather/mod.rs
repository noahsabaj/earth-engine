/// Weather module - data structures only
/// All weather logic runs on GPU via world_gpu::weather_gpu

pub mod weather_types;
pub mod weather_data;

// Re-export commonly used types
pub use weather_types::{WeatherType, WeatherIntensity, WeatherConditions, ParticleType};
pub use weather_data::{WeatherUpdate, BiomeType, WeatherRegion, ThunderEvent};

// Legacy compatibility exports (these will be removed in future)
pub use weather_types::WeatherConditions as LegacyWeatherConditions;