use std::collections::HashMap;
use crate::biome::{BiomeType, BiomeProperties};

/// Information about a biome at a specific location
#[derive(Debug, Clone)]
pub struct BiomeInfo {
    /// Primary biome type
    pub biome_type: BiomeType,
    /// Biome blend factor (for smooth transitions)
    pub blend_factor: f32,
    /// Neighboring biomes for blending
    pub neighbors: Vec<(BiomeType, f32)>,
}

/// Map that stores biome data for the world
pub struct BiomeMap {
    /// Cached biome data
    cache: HashMap<(i32, i32), BiomeType>,
    /// Biome scale (how large biomes are)
    biome_scale: f32,
    /// Temperature noise scale
    temperature_scale: f32,
    /// Humidity noise scale
    humidity_scale: f32,
    /// Seed for generation
    seed: u64,
}

impl BiomeMap {
    /// Create a new biome map
    pub fn new(seed: u64) -> Self {
        Self {
            cache: HashMap::new(),
            biome_scale: 256.0,
            temperature_scale: 512.0,
            humidity_scale: 512.0,
            seed,
        }
    }
    
    /// Get biome at a specific world position
    pub fn get_biome(&mut self, x: f64, z: f64) -> BiomeType {
        let chunk_x = (x / 16.0).floor() as i32;
        let chunk_z = (z / 16.0).floor() as i32;
        
        // Check cache first
        if let Some(&biome) = self.cache.get(&(chunk_x, chunk_z)) {
            return biome;
        }
        
        // Generate biome
        let biome = self.generate_biome(x, z);
        self.cache.insert((chunk_x, chunk_z), biome);
        biome
    }
    
    /// Get biome info with blending information
    pub fn get_biome_info(&mut self, x: f64, z: f64) -> BiomeInfo {
        let primary = self.get_biome(x, z);
        
        // Sample neighboring points for blending
        let sample_dist = 16.0;
        let neighbors = vec![
            self.get_biome(x + sample_dist, z),
            self.get_biome(x - sample_dist, z),
            self.get_biome(x, z + sample_dist),
            self.get_biome(x, z - sample_dist),
        ];
        
        // Calculate blend factors based on distance to biome boundaries
        let mut neighbor_map: HashMap<BiomeType, f32> = HashMap::new();
        for neighbor in neighbors {
            if neighbor != primary {
                *neighbor_map.entry(neighbor).or_insert(0.0) += 0.25;
            }
        }
        
        let neighbors: Vec<(BiomeType, f32)> = neighbor_map.into_iter().collect();
        let blend_factor = neighbors.iter().map(|(_, f)| f).sum::<f32>();
        
        BiomeInfo {
            biome_type: primary,
            blend_factor,
            neighbors,
        }
    }
    
    /// Generate biome based on temperature and humidity
    fn generate_biome(&self, x: f64, z: f64) -> BiomeType {
        // Get temperature and humidity at this position
        let temp = self.get_temperature(x, z);
        let humidity = self.get_humidity(x, z);
        
        // Get elevation (simplified - in real implementation would use height map)
        let elevation = self.get_elevation(x, z);
        
        // Special cases
        if elevation < -0.5 {
            return BiomeType::Ocean;
        } else if elevation < -0.1 {
            return BiomeType::River;
        } else if elevation < 0.0 {
            return BiomeType::Beach;
        } else if elevation > 0.8 {
            if temp < 0.2 {
                return BiomeType::SnowyMountains;
            } else {
                return BiomeType::Mountains;
            }
        }
        
        // Temperature/humidity based selection
        match (temp, humidity) {
            // Cold biomes
            (t, h) if t < 0.2 => {
                if h < 0.3 {
                    BiomeType::IcePlains
                } else if h < 0.6 {
                    BiomeType::SnowyTaiga
                } else {
                    BiomeType::Taiga
                }
            },
            // Temperate biomes
            (t, h) if t < 0.6 => {
                if h < 0.3 {
                    BiomeType::Plains
                } else if h < 0.6 {
                    BiomeType::Forest
                } else {
                    BiomeType::DarkForest
                }
            },
            // Warm biomes
            (t, h) if t < 0.8 => {
                if h < 0.3 {
                    BiomeType::Savanna
                } else if h < 0.7 {
                    BiomeType::Plains
                } else {
                    BiomeType::Swamp
                }
            },
            // Hot biomes
            _ => {
                if humidity < 0.3 {
                    BiomeType::Desert
                } else if humidity < 0.5 {
                    BiomeType::Badlands
                } else {
                    BiomeType::Jungle
                }
            },
        }
    }
    
    /// Get temperature at position (0.0 = cold, 1.0 = hot)
    fn get_temperature(&self, x: f64, z: f64) -> f32 {
        let scale = self.temperature_scale as f64;
        let value = self.noise_2d(x / scale, z / scale, 0);
        (value + 1.0) * 0.5 // Convert from -1..1 to 0..1
    }
    
    /// Get humidity at position (0.0 = dry, 1.0 = wet)
    fn get_humidity(&self, x: f64, z: f64) -> f32 {
        let scale = self.humidity_scale as f64;
        let value = self.noise_2d(x / scale, z / scale, 1);
        (value + 1.0) * 0.5
    }
    
    /// Get elevation at position (-1.0 = deep ocean, 1.0 = mountain peak)
    fn get_elevation(&self, x: f64, z: f64) -> f32 {
        // Simplified elevation - would use actual terrain height in full implementation
        let scale = 256.0;
        let value = self.noise_2d(x / scale, z / scale, 2);
        value * 0.5 // Scale down
    }
    
    /// Simple 2D noise function (placeholder - would use proper noise in production)
    fn noise_2d(&self, x: f64, y: f64, offset: u64) -> f32 {
        // This is a placeholder - in production, use proper Perlin/Simplex noise
        let seed = self.seed.wrapping_add(offset);
        let hash = ((x * 12.9898 + y * 78.233 + seed as f64) * 43758.5453).sin();
        (hash.fract() * 2.0 - 1.0) as f32
    }
    
    /// Clear the biome cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Get biome transition distance
    pub fn get_transition_distance(&self, from: BiomeType, to: BiomeType) -> f32 {
        // Smooth transitions between similar biomes
        match (from, to) {
            // Same biome = no transition
            (a, b) if a == b => 0.0,
            
            // Water transitions
            (BiomeType::Ocean, BiomeType::Beach) |
            (BiomeType::Beach, BiomeType::Ocean) => 8.0,
            
            // Temperature transitions
            (a, b) if a.is_cold() && b.is_cold() => 16.0,
            (a, b) if a.is_hot() && b.is_hot() => 16.0,
            
            // Forest transitions
            (BiomeType::Forest, BiomeType::BirchForest) |
            (BiomeType::BirchForest, BiomeType::Forest) => 12.0,
            
            // Default transition
            _ => 32.0,
        }
    }
    
    /// Blend biome properties based on biome info
    pub fn blend_properties(&self, info: &BiomeInfo) -> BiomeProperties {
        let mut props = BiomeProperties::from_biome_type(info.biome_type);
        
        if info.neighbors.is_empty() || info.blend_factor == 0.0 {
            return props;
        }
        
        // Blend colors and values with neighbors
        for (neighbor_type, weight) in &info.neighbors {
            let neighbor_props = BiomeProperties::from_biome_type(*neighbor_type);
            
            // Blend colors
            props.water_color = props.water_color.lerp(neighbor_props.water_color, *weight);
            props.fog_color = props.fog_color.lerp(neighbor_props.fog_color, *weight);
            props.sky_color = props.sky_color.lerp(neighbor_props.sky_color, *weight);
            props.grass_color = props.grass_color.lerp(neighbor_props.grass_color, *weight);
            props.foliage_color = props.foliage_color.lerp(neighbor_props.foliage_color, *weight);
            
            // Blend densities
            props.tree_density = props.tree_density + (neighbor_props.tree_density - props.tree_density) * weight;
            props.grass_density = props.grass_density + (neighbor_props.grass_density - props.grass_density) * weight;
            props.flower_density = props.flower_density + (neighbor_props.flower_density - props.flower_density) * weight;
        }
        
        props
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_biome_map() {
        let mut map = BiomeMap::new(12345);
        
        // Test biome generation
        let biome1 = map.get_biome(0.0, 0.0);
        let biome2 = map.get_biome(1000.0, 1000.0);
        
        // Should get consistent results for same position
        let biome1_again = map.get_biome(0.0, 0.0);
        assert_eq!(biome1, biome1_again);
    }
    
    #[test]
    fn test_biome_blending() {
        let mut map = BiomeMap::new(12345);
        let info = map.get_biome_info(100.0, 100.0);
        
        let blended = map.blend_properties(&info);
        // Properties should be valid
        assert!(blended.climate.temperature >= 0.0);
        assert!(blended.climate.humidity >= 0.0);
    }
}