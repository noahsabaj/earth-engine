use serde::{Serialize, Deserialize};

/// Unique identifier for a biome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BiomeId(pub u32);

/// Different types of biomes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiomeType {
    // Temperate biomes
    Plains,
    Forest,
    BirchForest,
    DarkForest,
    Swamp,
    River,
    Beach,
    
    // Cold biomes
    Taiga,
    SnowyTaiga,
    IcePlains,
    FrozenRiver,
    FrozenOcean,
    
    // Warm biomes
    Desert,
    Savanna,
    Jungle,
    Badlands,
    
    // Mountain biomes
    Mountains,
    MountainEdge,
    SnowyMountains,
    WoodedMountains,
    GravellyMountains,
    
    // Ocean biomes
    Ocean,
    DeepOcean,
    WarmOcean,
    LukewarmOcean,
    ColdOcean,
    
    // Special biomes
    MushroomIsland,
    TheVoid,
    
    // Underground biomes
    Cave,
    DeepCave,
    LushCave,
    DripstoneCase,
}

impl BiomeType {
    /// Get the biome ID
    pub fn id(&self) -> BiomeId {
        BiomeId(match self {
            BiomeType::Plains => 1,
            BiomeType::Forest => 2,
            BiomeType::BirchForest => 3,
            BiomeType::DarkForest => 4,
            BiomeType::Swamp => 5,
            BiomeType::River => 6,
            BiomeType::Beach => 7,
            BiomeType::Taiga => 8,
            BiomeType::SnowyTaiga => 9,
            BiomeType::IcePlains => 10,
            BiomeType::FrozenRiver => 11,
            BiomeType::FrozenOcean => 12,
            BiomeType::Desert => 13,
            BiomeType::Savanna => 14,
            BiomeType::Jungle => 15,
            BiomeType::Badlands => 16,
            BiomeType::Mountains => 17,
            BiomeType::MountainEdge => 18,
            BiomeType::SnowyMountains => 19,
            BiomeType::WoodedMountains => 20,
            BiomeType::GravellyMountains => 21,
            BiomeType::Ocean => 22,
            BiomeType::DeepOcean => 23,
            BiomeType::WarmOcean => 24,
            BiomeType::LukewarmOcean => 25,
            BiomeType::ColdOcean => 26,
            BiomeType::MushroomIsland => 27,
            BiomeType::TheVoid => 28,
            BiomeType::Cave => 29,
            BiomeType::DeepCave => 30,
            BiomeType::LushCave => 31,
            BiomeType::DripstoneCase => 32,
        })
    }
    
    /// Get biome from ID
    pub fn from_id(id: BiomeId) -> Option<Self> {
        match id.0 {
            1 => Some(BiomeType::Plains),
            2 => Some(BiomeType::Forest),
            3 => Some(BiomeType::BirchForest),
            4 => Some(BiomeType::DarkForest),
            5 => Some(BiomeType::Swamp),
            6 => Some(BiomeType::River),
            7 => Some(BiomeType::Beach),
            8 => Some(BiomeType::Taiga),
            9 => Some(BiomeType::SnowyTaiga),
            10 => Some(BiomeType::IcePlains),
            11 => Some(BiomeType::FrozenRiver),
            12 => Some(BiomeType::FrozenOcean),
            13 => Some(BiomeType::Desert),
            14 => Some(BiomeType::Savanna),
            15 => Some(BiomeType::Jungle),
            16 => Some(BiomeType::Badlands),
            17 => Some(BiomeType::Mountains),
            18 => Some(BiomeType::MountainEdge),
            19 => Some(BiomeType::SnowyMountains),
            20 => Some(BiomeType::WoodedMountains),
            21 => Some(BiomeType::GravellyMountains),
            22 => Some(BiomeType::Ocean),
            23 => Some(BiomeType::DeepOcean),
            24 => Some(BiomeType::WarmOcean),
            25 => Some(BiomeType::LukewarmOcean),
            26 => Some(BiomeType::ColdOcean),
            27 => Some(BiomeType::MushroomIsland),
            28 => Some(BiomeType::TheVoid),
            29 => Some(BiomeType::Cave),
            30 => Some(BiomeType::DeepCave),
            31 => Some(BiomeType::LushCave),
            32 => Some(BiomeType::DripstoneCase),
            _ => None,
        }
    }
    
    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            BiomeType::Plains => "Plains",
            BiomeType::Forest => "Forest",
            BiomeType::BirchForest => "Birch Forest",
            BiomeType::DarkForest => "Dark Forest",
            BiomeType::Swamp => "Swamp",
            BiomeType::River => "River",
            BiomeType::Beach => "Beach",
            BiomeType::Taiga => "Taiga",
            BiomeType::SnowyTaiga => "Snowy Taiga",
            BiomeType::IcePlains => "Ice Plains",
            BiomeType::FrozenRiver => "Frozen River",
            BiomeType::FrozenOcean => "Frozen Ocean",
            BiomeType::Desert => "Desert",
            BiomeType::Savanna => "Savanna",
            BiomeType::Jungle => "Jungle",
            BiomeType::Badlands => "Badlands",
            BiomeType::Mountains => "Mountains",
            BiomeType::MountainEdge => "Mountain Edge",
            BiomeType::SnowyMountains => "Snowy Mountains",
            BiomeType::WoodedMountains => "Wooded Mountains",
            BiomeType::GravellyMountains => "Gravelly Mountains",
            BiomeType::Ocean => "Ocean",
            BiomeType::DeepOcean => "Deep Ocean",
            BiomeType::WarmOcean => "Warm Ocean",
            BiomeType::LukewarmOcean => "Lukewarm Ocean",
            BiomeType::ColdOcean => "Cold Ocean",
            BiomeType::MushroomIsland => "Mushroom Island",
            BiomeType::TheVoid => "The Void",
            BiomeType::Cave => "Cave",
            BiomeType::DeepCave => "Deep Cave",
            BiomeType::LushCave => "Lush Cave",
            BiomeType::DripstoneCase => "Dripstone Cave",
        }
    }
    
    /// Check if this is a water biome
    pub fn is_water(&self) -> bool {
        matches!(self,
            BiomeType::River |
            BiomeType::FrozenRiver |
            BiomeType::Ocean |
            BiomeType::DeepOcean |
            BiomeType::WarmOcean |
            BiomeType::LukewarmOcean |
            BiomeType::ColdOcean |
            BiomeType::FrozenOcean
        )
    }
    
    /// Check if this is a cold biome
    pub fn is_cold(&self) -> bool {
        matches!(self,
            BiomeType::SnowyTaiga |
            BiomeType::IcePlains |
            BiomeType::FrozenRiver |
            BiomeType::FrozenOcean |
            BiomeType::SnowyMountains |
            BiomeType::ColdOcean
        )
    }
    
    /// Check if this is a hot biome
    pub fn is_hot(&self) -> bool {
        matches!(self,
            BiomeType::Desert |
            BiomeType::Savanna |
            BiomeType::Badlands |
            BiomeType::WarmOcean
        )
    }
    
    /// Check if this is a mountain biome
    pub fn is_mountain(&self) -> bool {
        matches!(self,
            BiomeType::Mountains |
            BiomeType::MountainEdge |
            BiomeType::SnowyMountains |
            BiomeType::WoodedMountains |
            BiomeType::GravellyMountains
        )
    }
    
    /// Check if this is an underground biome
    pub fn is_underground(&self) -> bool {
        matches!(self,
            BiomeType::Cave |
            BiomeType::DeepCave |
            BiomeType::LushCave |
            BiomeType::DripstoneCase
        )
    }
    
    /// Get the base height for this biome
    pub fn base_height(&self) -> f32 {
        match self {
            // Water biomes
            BiomeType::Ocean => -1.0,
            BiomeType::DeepOcean => -1.8,
            BiomeType::River => -0.5,
            BiomeType::FrozenRiver => -0.5,
            BiomeType::Beach => 0.0,
            
            // Flat biomes
            BiomeType::Plains => 0.125,
            BiomeType::IcePlains => 0.125,
            BiomeType::Desert => 0.125,
            BiomeType::TheVoid => 0.0,
            
            // Medium height biomes
            BiomeType::Forest => 0.1,
            BiomeType::BirchForest => 0.1,
            BiomeType::Taiga => 0.2,
            BiomeType::SnowyTaiga => 0.2,
            BiomeType::Savanna => 0.125,
            BiomeType::Swamp => -0.2,
            BiomeType::MushroomIsland => 0.2,
            
            // High biomes
            BiomeType::DarkForest => 0.2,
            BiomeType::Jungle => 0.1,
            BiomeType::Badlands => 1.5,
            
            // Mountain biomes
            BiomeType::Mountains => 1.0,
            BiomeType::MountainEdge => 0.8,
            BiomeType::SnowyMountains => 1.2,
            BiomeType::WoodedMountains => 1.0,
            BiomeType::GravellyMountains => 1.0,
            
            // Underground (not used for surface generation)
            _ => 0.0,
        }
    }
    
    /// Get the height variation for this biome
    pub fn height_variation(&self) -> f32 {
        match self {
            // Flat biomes
            BiomeType::Ocean => 0.1,
            BiomeType::DeepOcean => 0.1,
            BiomeType::Plains => 0.05,
            BiomeType::IcePlains => 0.05,
            BiomeType::Desert => 0.05,
            BiomeType::Beach => 0.025,
            BiomeType::TheVoid => 0.0,
            
            // Medium variation
            BiomeType::Forest => 0.2,
            BiomeType::BirchForest => 0.2,
            BiomeType::Taiga => 0.2,
            BiomeType::SnowyTaiga => 0.2,
            BiomeType::Savanna => 0.15,
            BiomeType::River => 0.0,
            BiomeType::FrozenRiver => 0.0,
            
            // High variation
            BiomeType::DarkForest => 0.4,
            BiomeType::Jungle => 0.45,
            BiomeType::Swamp => 0.25,
            BiomeType::MushroomIsland => 0.3,
            BiomeType::Badlands => 0.3,
            
            // Very high variation
            BiomeType::Mountains => 0.5,
            BiomeType::MountainEdge => 0.3,
            BiomeType::SnowyMountains => 0.45,
            BiomeType::WoodedMountains => 0.5,
            BiomeType::GravellyMountains => 0.5,
            
            // Underground
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_biome_properties() {
        assert!(BiomeType::Ocean.is_water());
        assert!(!BiomeType::Plains.is_water());
        
        assert!(BiomeType::IcePlains.is_cold());
        assert!(!BiomeType::Desert.is_cold());
        
        assert!(BiomeType::Desert.is_hot());
        assert!(!BiomeType::Taiga.is_hot());
        
        assert!(BiomeType::Mountains.is_mountain());
        assert!(!BiomeType::Plains.is_mountain());
    }
    
    #[test]
    fn test_biome_id_conversion() {
        let biome = BiomeType::Forest;
        let id = biome.id();
        let converted = BiomeType::from_id(id);
        assert_eq!(Some(biome), converted);
    }
}