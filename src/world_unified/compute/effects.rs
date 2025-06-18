//! GPU effects and lighting systems

// Re-export effects from world_gpu
pub use crate::world_gpu::{
    GpuLighting, GpuLightPropagator, GpuBlockProvider,
    WeatherGpu, WeatherData, WeatherTransition, WeatherConfig,
    PrecipitationParticle, WeatherGpuDescriptor
};