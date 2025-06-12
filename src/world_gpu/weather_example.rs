/// Example usage of the GPU weather system

use std::sync::Arc;
use wgpu::{Device, Queue};
use crate::weather::BiomeType;
use crate::world_gpu::{
    WeatherGpu, WeatherGpuDescriptor, WeatherConfig,
    weather_migration::{create_weather_transition, create_default_weather_config},
};

/// Example of setting up and using the GPU weather system
pub fn setup_weather_system(
    device: Arc<Device>,
    queue: &Queue,
) -> WeatherGpu {
    // Create weather GPU system
    let weather_desc = WeatherGpuDescriptor {
        region_count: 4096, // 16x16x16 regions
        max_particles: 100000,
        enable_particles: true,
    };
    
    let weather_gpu = WeatherGpu::new(device, weather_desc);
    
    // Initialize weather for different biomes
    let biomes = [
        (0, BiomeType::Plains),
        (1, BiomeType::Forest),
        (2, BiomeType::Desert),
        (3, BiomeType::Tundra),
        (4, BiomeType::Mountain),
        (5, BiomeType::Swamp),
        (6, BiomeType::Ocean),
    ];
    
    // Create transition data for each region
    let mut transitions = Vec::new();
    for region_id in 0..4096 {
        // Simple biome assignment based on region position
        let biome_idx = (region_id / 512) % 7;
        let biome = biomes[biome_idx as usize].1;
        transitions.push(create_weather_transition(biome));
    }
    
    // Upload transition data to GPU
    // (In real usage, this would be done through the weather GPU API)
    
    // Set up initial configuration
    let config = create_default_weather_config([0.0, 100.0, 0.0]);
    weather_gpu.update_config(queue, &config);
    
    weather_gpu
}

/// Example frame update
pub fn update_weather_frame(
    weather_gpu: &WeatherGpu,
    queue: &Queue,
    encoder: &mut wgpu::CommandEncoder,
    player_pos: [f32; 3],
    frame_number: u32,
    delta_ms: u32,
) {
    // Update configuration for this frame
    let config = WeatherConfig {
        frame_number,
        delta_time_ms: delta_ms,
        player_position: player_pos,
        precipitation_radius: 200.0,
        max_particles: 100000,
        particle_count: 50000, // Current active particles
        random_seed: frame_number * 1337,
        flags: 0xFFFFFFFF,
    };
    
    weather_gpu.update_config(queue, &config);
    
    // Run weather compute pass
    weather_gpu.update(encoder);
}

/// Example of querying weather data (would need readback in real usage)
pub struct WeatherQuery {
    pub region_id: u32,
    pub weather_type: u32,
    pub intensity: u32,
    pub temperature: f32,
    pub wind_speed: f32,
    pub precipitation_rate: f32,
}

impl WeatherQuery {
    /// Convert from GPU format to CPU-friendly format
    pub fn from_gpu_data(data: &crate::world_gpu::WeatherData) -> Self {
        Self {
            region_id: 0, // Would be set based on query
            weather_type: data.weather_type_intensity & 0xFF,
            intensity: (data.weather_type_intensity >> 8) & 0xFF,
            temperature: data.temperature as f32 / 10.0,
            wind_speed: data.wind_speed as f32 / 10.0,
            precipitation_rate: data.precipitation_rate as f32 / 1000.0,
        }
    }
}