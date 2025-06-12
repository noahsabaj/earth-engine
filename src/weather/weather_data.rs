/// Weather data structures used by the GPU weather system
/// This file only contains data definitions - all logic is on GPU

use serde::{Serialize, Deserialize};

/// Weather system update event
#[derive(Debug, Clone)]
pub struct WeatherUpdate {
    pub region_id: u32,
    pub weather_type: u32,
    pub intensity: u32,
    pub transition_progress: f32,
}

/// Biome types that affect weather patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiomeType {
    Plains = 0,
    Forest = 1,
    Desert = 2,
    Tundra = 3,
    Mountain = 4,
    Swamp = 5,
    Ocean = 6,
}

impl BiomeType {
    pub fn to_u32(&self) -> u32 {
        *self as u32
    }
    
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(BiomeType::Plains),
            1 => Some(BiomeType::Forest),
            2 => Some(BiomeType::Desert),
            3 => Some(BiomeType::Tundra),
            4 => Some(BiomeType::Mountain),
            5 => Some(BiomeType::Swamp),
            6 => Some(BiomeType::Ocean),
            _ => None,
        }
    }
}

/// Weather region data for CPU-side queries
#[derive(Debug, Clone)]
pub struct WeatherRegion {
    pub biome: BiomeType,
    pub position: [f32; 3],
    pub size: f32,
}

/// Thunder event for audio/visual effects
#[derive(Debug, Clone)]
pub struct ThunderEvent {
    pub position: [f32; 3],
    pub intensity: f32,
    pub duration: f32,
}