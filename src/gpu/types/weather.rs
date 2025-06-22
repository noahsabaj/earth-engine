//! GPU-compatible weather types for WGSL
//!
//! WGSL doesn't support i16/u16, so we use u32 for all fields

use crate::gpu::automation::auto_layout::AutoLayout;
use crate::gpu::automation::auto_wgsl::AutoWgsl;
use bytemuck::{Pod, Zeroable};
use encase::ShaderType;

/// GPU-compatible weather data (WGSL version)
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug)]
pub struct WeatherDataGpu {
    /// Packed weather type and intensity (0-7 type, 8-15 intensity)
    pub weather_type_intensity: u32,
    /// Temperature in Celsius * 10 (as u32)
    pub temperature: u32,
    /// Humidity percentage * 100 (0-10000 for 0-100%)
    pub humidity: u32,
    /// Wind speed in m/s * 10
    pub wind_speed: u32,
    /// Wind direction in degrees (0-359)
    pub wind_direction: u32,
    /// Visibility factor * 1000 (0-1000 for 0.0-1.0)
    pub visibility: u32,
    /// Precipitation rate * 1000
    pub precipitation_rate: u32,
    /// Padding for alignment
    pub _padding: u32,
}

// Implement AutoWgsl
crate::auto_wgsl!(
    WeatherDataGpu,
    name = "WeatherData",
    fields = [
        weather_type_intensity: "u32",
        temperature: "u32",
        humidity: "u32",
        wind_speed: "u32",
        wind_direction: "u32",
        visibility: "u32",
        precipitation_rate: "u32",
        _padding: "u32",
    ]
);

// Implement AutoLayout
crate::impl_auto_layout!(
    WeatherDataGpu,
    fields = [
        weather_type_intensity: u32 = "weather_type_intensity",
        temperature: u32 = "temperature",
        humidity: u32 = "humidity",
        wind_speed: u32 = "wind_speed",
        wind_direction: u32 = "wind_direction",
        visibility: u32 = "visibility",
        precipitation_rate: u32 = "precipitation_rate",
        _padding: u32 = "_padding"
    ]
);

/// GPU-compatible precipitation particle
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug)]
pub struct PrecipitationParticleGpu {
    /// World position
    pub position: [f32; 3],
    /// Particle type (0=rain, 1=snow, 2=hail, etc.)
    pub particle_type: u32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Lifetime (0.0-1.0)
    pub lifetime: f32,
    /// Particle size
    pub size: f32,
    /// Padding for alignment
    pub _padding: [f32; 3],
}

// Implement AutoWgsl
crate::auto_wgsl!(
    PrecipitationParticleGpu,
    name = "PrecipitationParticle",
    fields = [
        position: "f32"[3],
        particle_type: "u32",
        velocity: "f32"[3],
        lifetime: "f32",
        size: "f32",
        _padding: "f32"[3],
    ]
);

// Implement AutoLayout
crate::impl_auto_layout!(
    PrecipitationParticleGpu,
    fields = [
        position: [f32; 3] = "position",
        particle_type: u32 = "particle_type",
        velocity: [f32; 3] = "velocity",
        lifetime: f32 = "lifetime",
        size: f32 = "size",
        _padding: [f32; 3] = "_padding"
    ]
);

/// GPU-compatible weather transition
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug)]
pub struct WeatherTransitionGpu {
    /// Current weather data
    pub current: WeatherDataGpu,
    /// Target weather data
    pub target_weather: WeatherDataGpu,
    /// Transition progress (0-65535 for 0.0-1.0)
    pub progress: u32,
    /// Transition speed per frame
    pub speed: u32,
    /// Time until next weather change (in frames)
    pub change_timer: u32,
    /// Biome type for weather generation
    pub biome_type: u32,
}

// Implement AutoWgsl
crate::auto_wgsl!(
    WeatherTransitionGpu,
    name = "WeatherTransition",
    fields = [
        current: "WeatherData",
        target_weather: "WeatherData",
        progress: "u32",
        speed: "u32",
        change_timer: "u32",
        biome_type: "u32",
    ]
);

// Implement AutoLayout
crate::impl_auto_layout!(
    WeatherTransitionGpu,
    fields = [
        current: WeatherDataGpu = "current",
        target_weather: WeatherDataGpu = "target_weather",
        progress: u32 = "progress",
        speed: u32 = "speed",
        change_timer: u32 = "change_timer",
        biome_type: u32 = "biome_type"
    ]
);

/// Weather configuration uniform
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug)]
pub struct WeatherConfigGpu {
    /// Current frame number
    pub frame_number: u32,
    /// Delta time in milliseconds
    pub delta_time_ms: u32,
    /// Player position for particle spawning
    pub player_position: [f32; 3],
    /// Precipitation spawn radius
    pub precipitation_radius: f32,
    /// Maximum particles
    pub max_particles: u32,
    /// Current particle count
    pub particle_count: u32,
    /// Random seed
    pub random_seed: u32,
    /// Configuration flags
    pub flags: u32,
}

// Implement AutoWgsl
crate::auto_wgsl!(
    WeatherConfigGpu,
    name = "WeatherConfig",
    fields = [
        frame_number: "u32",
        delta_time_ms: "u32",
        player_position: "f32"[3],
        precipitation_radius: "f32",
        max_particles: "u32",
        particle_count: "u32",
        random_seed: "u32",
        flags: "u32",
    ]
);

// Implement AutoLayout
crate::impl_auto_layout!(
    WeatherConfigGpu,
    fields = [
        frame_number: u32 = "frame_number",
        delta_time_ms: u32 = "delta_time_ms",
        player_position: [f32; 3] = "player_position",
        precipitation_radius: f32 = "precipitation_radius",
        max_particles: u32 = "max_particles",
        particle_count: u32 = "particle_count",
        random_seed: u32 = "random_seed",
        flags: u32 = "flags"
    ]
);

// Conversion helpers
impl From<&crate::world::WeatherData> for WeatherDataGpu {
    fn from(data: &crate::world::WeatherData) -> Self {
        Self {
            weather_type_intensity: data.weather_type_intensity,
            temperature: data.temperature as u32,
            humidity: data.humidity as u32,
            wind_speed: data.wind_speed as u32,
            wind_direction: data.wind_direction as u32,
            visibility: data.visibility as u32,
            precipitation_rate: data.precipitation_rate as u32,
            _padding: 0,
        }
    }
}
