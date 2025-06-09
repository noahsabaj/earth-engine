use glam::Vec3;
use serde::{Serialize, Deserialize};
use crate::biome::BiomeType;
use crate::world::BlockId;

/// Properties that define a biome's characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeProperties {
    /// Climate of the biome
    pub climate: BiomeClimate,
    /// Surface block (e.g., grass)
    pub surface_block: BlockId,
    /// Subsurface block (e.g., dirt)
    pub subsurface_block: BlockId,
    /// Stone variant for this biome
    pub stone_block: BlockId,
    /// Water color
    pub water_color: Vec3,
    /// Fog color
    pub fog_color: Vec3,
    /// Sky color modifier
    pub sky_color: Vec3,
    /// Grass color
    pub grass_color: Vec3,
    /// Foliage color
    pub foliage_color: Vec3,
    /// Precipitation type
    pub precipitation: PrecipitationType,
    /// Tree density (trees per chunk)
    pub tree_density: f32,
    /// Grass density
    pub grass_density: f32,
    /// Flower density
    pub flower_density: f32,
    /// Mob spawn info
    pub mob_spawns: MobSpawnInfo,
}

/// Climate information for a biome
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BiomeClimate {
    /// Temperature (0.0 = freezing, 1.0 = hot)
    pub temperature: f32,
    /// Humidity (0.0 = dry, 1.0 = wet)
    pub humidity: f32,
    /// Whether water freezes in this biome
    pub freezes_water: bool,
    /// Whether it snows instead of rains
    pub has_snow: bool,
}

/// Type of precipitation in the biome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecipitationType {
    None,
    Rain,
    Snow,
}

/// Information about mob spawning in the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobSpawnInfo {
    /// Hostile mob spawn weight
    pub hostile_weight: f32,
    /// Passive mob spawn weight
    pub passive_weight: f32,
    /// Ambient mob spawn weight (bats, etc.)
    pub ambient_weight: f32,
    /// Water mob spawn weight
    pub water_weight: f32,
}

impl BiomeProperties {
    /// Get properties for a biome type
    pub fn from_biome_type(biome: BiomeType) -> Self {
        match biome {
            BiomeType::Plains => Self::plains(),
            BiomeType::Forest => Self::forest(),
            BiomeType::Desert => Self::desert(),
            BiomeType::Taiga => Self::taiga(),
            BiomeType::Swamp => Self::swamp(),
            BiomeType::Mountains => Self::mountains(),
            BiomeType::Ocean => Self::ocean(),
            BiomeType::Jungle => Self::jungle(),
            BiomeType::IcePlains => Self::ice_plains(),
            BiomeType::Beach => Self::beach(),
            BiomeType::River => Self::river(),
            BiomeType::SnowyTaiga => Self::snowy_taiga(),
            BiomeType::Badlands => Self::badlands(),
            _ => Self::default(),
        }
    }
    
    fn plains() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.8,
                humidity: 0.4,
                freezes_water: false,
                has_snow: false,
            },
            surface_block: BlockId::GRASS,
            subsurface_block: BlockId::DIRT,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.3, 0.5, 0.8),
            fog_color: Vec3::new(0.7, 0.8, 0.9),
            sky_color: Vec3::new(0.5, 0.7, 1.0),
            grass_color: Vec3::new(0.5, 0.8, 0.3),
            foliage_color: Vec3::new(0.4, 0.7, 0.2),
            precipitation: PrecipitationType::Rain,
            tree_density: 0.1,
            grass_density: 0.8,
            flower_density: 0.3,
            mob_spawns: MobSpawnInfo::default(),
        }
    }
    
    fn forest() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.7,
                humidity: 0.8,
                freezes_water: false,
                has_snow: false,
            },
            surface_block: BlockId::GRASS,
            subsurface_block: BlockId::DIRT,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.3, 0.5, 0.8),
            fog_color: Vec3::new(0.6, 0.7, 0.8),
            sky_color: Vec3::new(0.5, 0.7, 1.0),
            grass_color: Vec3::new(0.3, 0.6, 0.1),
            foliage_color: Vec3::new(0.2, 0.5, 0.1),
            precipitation: PrecipitationType::Rain,
            tree_density: 10.0,
            grass_density: 0.4,
            flower_density: 0.2,
            mob_spawns: MobSpawnInfo::default(),
        }
    }
    
    fn desert() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 2.0,
                humidity: 0.0,
                freezes_water: false,
                has_snow: false,
            },
            surface_block: BlockId::SAND,
            subsurface_block: BlockId::SAND,
            stone_block: BlockId::SANDSTONE,
            water_color: Vec3::new(0.3, 0.5, 0.8),
            fog_color: Vec3::new(0.9, 0.8, 0.6),
            sky_color: Vec3::new(0.7, 0.8, 0.9),
            grass_color: Vec3::new(0.7, 0.7, 0.3),
            foliage_color: Vec3::new(0.6, 0.6, 0.2),
            precipitation: PrecipitationType::None,
            tree_density: 0.0,
            grass_density: 0.05,
            flower_density: 0.0,
            mob_spawns: MobSpawnInfo {
                hostile_weight: 1.5,
                passive_weight: 0.2,
                ambient_weight: 0.1,
                water_weight: 0.0,
            },
        }
    }
    
    fn taiga() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.25,
                humidity: 0.8,
                freezes_water: true,
                has_snow: false,
            },
            surface_block: BlockId::GRASS,
            subsurface_block: BlockId::DIRT,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.2, 0.4, 0.7),
            fog_color: Vec3::new(0.6, 0.7, 0.8),
            sky_color: Vec3::new(0.5, 0.7, 1.0),
            grass_color: Vec3::new(0.3, 0.5, 0.3),
            foliage_color: Vec3::new(0.2, 0.4, 0.2),
            precipitation: PrecipitationType::Rain,
            tree_density: 8.0,
            grass_density: 0.3,
            flower_density: 0.1,
            mob_spawns: MobSpawnInfo::default(),
        }
    }
    
    fn swamp() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.8,
                humidity: 0.9,
                freezes_water: false,
                has_snow: false,
            },
            surface_block: BlockId::GRASS,
            subsurface_block: BlockId::DIRT,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.2, 0.3, 0.3),
            fog_color: Vec3::new(0.5, 0.5, 0.5),
            sky_color: Vec3::new(0.5, 0.6, 0.7),
            grass_color: Vec3::new(0.4, 0.5, 0.2),
            foliage_color: Vec3::new(0.3, 0.4, 0.1),
            precipitation: PrecipitationType::Rain,
            tree_density: 6.0,
            grass_density: 0.6,
            flower_density: 0.4,
            mob_spawns: MobSpawnInfo {
                hostile_weight: 1.2,
                passive_weight: 0.8,
                ambient_weight: 1.5,
                water_weight: 0.5,
            },
        }
    }
    
    fn mountains() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.2,
                humidity: 0.3,
                freezes_water: true,
                has_snow: true,
            },
            surface_block: BlockId::STONE,
            subsurface_block: BlockId::STONE,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.3, 0.5, 0.8),
            fog_color: Vec3::new(0.7, 0.8, 0.9),
            sky_color: Vec3::new(0.5, 0.7, 1.0),
            grass_color: Vec3::new(0.3, 0.5, 0.3),
            foliage_color: Vec3::new(0.2, 0.4, 0.2),
            precipitation: PrecipitationType::Snow,
            tree_density: 0.5,
            grass_density: 0.1,
            flower_density: 0.05,
            mob_spawns: MobSpawnInfo {
                hostile_weight: 0.8,
                passive_weight: 0.5,
                ambient_weight: 0.3,
                water_weight: 0.0,
            },
        }
    }
    
    fn ocean() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.5,
                humidity: 0.5,
                freezes_water: false,
                has_snow: false,
            },
            surface_block: BlockId::WATER,
            subsurface_block: BlockId::SAND,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.2, 0.4, 0.7),
            fog_color: Vec3::new(0.6, 0.7, 0.8),
            sky_color: Vec3::new(0.5, 0.7, 1.0),
            grass_color: Vec3::new(0.5, 0.8, 0.3),
            foliage_color: Vec3::new(0.4, 0.7, 0.2),
            precipitation: PrecipitationType::Rain,
            tree_density: 0.0,
            grass_density: 0.0,
            flower_density: 0.0,
            mob_spawns: MobSpawnInfo {
                hostile_weight: 0.1,
                passive_weight: 0.0,
                ambient_weight: 0.0,
                water_weight: 3.0,
            },
        }
    }
    
    fn jungle() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.95,
                humidity: 0.9,
                freezes_water: false,
                has_snow: false,
            },
            surface_block: BlockId::GRASS,
            subsurface_block: BlockId::DIRT,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.2, 0.5, 0.7),
            fog_color: Vec3::new(0.5, 0.6, 0.5),
            sky_color: Vec3::new(0.5, 0.7, 1.0),
            grass_color: Vec3::new(0.2, 0.9, 0.1),
            foliage_color: Vec3::new(0.1, 0.8, 0.0),
            precipitation: PrecipitationType::Rain,
            tree_density: 20.0,
            grass_density: 0.9,
            flower_density: 0.5,
            mob_spawns: MobSpawnInfo {
                hostile_weight: 0.8,
                passive_weight: 1.5,
                ambient_weight: 2.0,
                water_weight: 0.2,
            },
        }
    }
    
    fn ice_plains() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.0,
                humidity: 0.5,
                freezes_water: true,
                has_snow: true,
            },
            surface_block: BlockId::GRASS,
            subsurface_block: BlockId::DIRT,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.3, 0.4, 0.6),
            fog_color: Vec3::new(0.8, 0.8, 0.9),
            sky_color: Vec3::new(0.6, 0.7, 0.9),
            grass_color: Vec3::new(0.4, 0.4, 0.5),
            foliage_color: Vec3::new(0.3, 0.3, 0.4),
            precipitation: PrecipitationType::Snow,
            tree_density: 0.1,
            grass_density: 0.2,
            flower_density: 0.05,
            mob_spawns: MobSpawnInfo {
                hostile_weight: 0.8,
                passive_weight: 0.3,
                ambient_weight: 0.2,
                water_weight: 0.0,
            },
        }
    }
    
    fn beach() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.8,
                humidity: 0.4,
                freezes_water: false,
                has_snow: false,
            },
            surface_block: BlockId::SAND,
            subsurface_block: BlockId::SAND,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.3, 0.5, 0.8),
            fog_color: Vec3::new(0.7, 0.8, 0.9),
            sky_color: Vec3::new(0.5, 0.7, 1.0),
            grass_color: Vec3::new(0.5, 0.8, 0.3),
            foliage_color: Vec3::new(0.4, 0.7, 0.2),
            precipitation: PrecipitationType::Rain,
            tree_density: 0.0,
            grass_density: 0.1,
            flower_density: 0.0,
            mob_spawns: MobSpawnInfo::default(),
        }
    }
    
    fn river() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 0.5,
                humidity: 0.5,
                freezes_water: false,
                has_snow: false,
            },
            surface_block: BlockId::WATER,
            subsurface_block: BlockId::SAND,
            stone_block: BlockId::STONE,
            water_color: Vec3::new(0.2, 0.4, 0.7),
            fog_color: Vec3::new(0.6, 0.7, 0.8),
            sky_color: Vec3::new(0.5, 0.7, 1.0),
            grass_color: Vec3::new(0.5, 0.8, 0.3),
            foliage_color: Vec3::new(0.4, 0.7, 0.2),
            precipitation: PrecipitationType::Rain,
            tree_density: 0.0,
            grass_density: 0.3,
            flower_density: 0.1,
            mob_spawns: MobSpawnInfo {
                hostile_weight: 0.1,
                passive_weight: 0.2,
                ambient_weight: 0.1,
                water_weight: 1.0,
            },
        }
    }
    
    fn snowy_taiga() -> Self {
        let mut taiga = Self::taiga();
        taiga.climate.temperature = 0.0;
        taiga.climate.has_snow = true;
        taiga.precipitation = PrecipitationType::Snow;
        taiga
    }
    
    fn badlands() -> Self {
        Self {
            climate: BiomeClimate {
                temperature: 2.0,
                humidity: 0.0,
                freezes_water: false,
                has_snow: false,
            },
            surface_block: BlockId::RED_SAND,
            subsurface_block: BlockId::RED_SANDSTONE,
            stone_block: BlockId::RED_SANDSTONE,
            water_color: Vec3::new(0.3, 0.5, 0.8),
            fog_color: Vec3::new(0.9, 0.7, 0.5),
            sky_color: Vec3::new(0.8, 0.7, 0.6),
            grass_color: Vec3::new(0.7, 0.6, 0.3),
            foliage_color: Vec3::new(0.6, 0.5, 0.2),
            precipitation: PrecipitationType::None,
            tree_density: 0.0,
            grass_density: 0.0,
            flower_density: 0.0,
            mob_spawns: MobSpawnInfo {
                hostile_weight: 1.2,
                passive_weight: 0.1,
                ambient_weight: 0.1,
                water_weight: 0.0,
            },
        }
    }
}

impl Default for BiomeProperties {
    fn default() -> Self {
        Self::plains()
    }
}

impl Default for MobSpawnInfo {
    fn default() -> Self {
        Self {
            hostile_weight: 1.0,
            passive_weight: 1.0,
            ambient_weight: 0.5,
            water_weight: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_biome_properties() {
        let desert = BiomeProperties::from_biome_type(BiomeType::Desert);
        assert_eq!(desert.surface_block, BlockId::SAND);
        assert_eq!(desert.precipitation, PrecipitationType::None);
        assert!(desert.climate.temperature > 1.5);
        
        let taiga = BiomeProperties::from_biome_type(BiomeType::Taiga);
        assert!(taiga.climate.freezes_water);
        assert_eq!(taiga.surface_block, BlockId::GRASS);
    }
}