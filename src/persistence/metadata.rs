use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use glam::Vec3;

/// Current save version
pub const SAVE_VERSION: SaveVersion = SaveVersion {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Version information for save files
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SaveVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl SaveVersion {
    /// Create a new version
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch }
    }
    
    /// Check if this version is compatible with another
    pub fn is_compatible_with(&self, other: &SaveVersion) -> bool {
        // Major version must match
        if self.major != other.major {
            return false;
        }
        
        // Can load older minor versions
        if self.minor < other.minor {
            return false;
        }
        
        true
    }
    
    /// Check if migration is needed
    pub fn needs_migration(&self, other: &SaveVersion) -> bool {
        self.major != other.major || self.minor != other.minor
    }
}

impl std::fmt::Display for SaveVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// World metadata containing information about the save
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMetadata {
    /// Save format version
    pub version: SaveVersion,
    /// World name
    pub name: String,
    /// World seed
    pub seed: u64,
    /// Creation timestamp
    pub created_at: u64,
    /// Last modified timestamp
    pub modified_at: u64,
    /// Total play time in seconds
    pub play_time: u64,
    /// Game version that created the world
    pub game_version: String,
    /// World spawn point
    pub spawn_point: Vec3,
    /// World bounds (if limited)
    pub world_bounds: Option<WorldBounds>,
    /// Game rules
    pub game_rules: GameRules,
    /// World statistics
    pub statistics: WorldStatistics,
    /// Custom properties
    pub custom_properties: HashMap<String, String>,
}

/// World boundary limits
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorldBounds {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub min_z: i32,
    pub max_z: i32,
}

/// Game rules for the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRules {
    /// PvP enabled
    pub pvp_enabled: bool,
    /// Keep inventory on death
    pub keep_inventory: bool,
    /// Natural mob spawning
    pub mob_spawning: bool,
    /// Fire spread
    pub fire_spread: bool,
    /// TNT explodes
    pub tnt_enabled: bool,
    /// Day/night cycle
    pub day_night_cycle: bool,
    /// Weather changes
    pub weather_cycle: bool,
    /// Difficulty level
    pub difficulty: Difficulty,
    /// Maximum players
    pub max_players: u32,
    /// View distance in chunks
    pub view_distance: u32,
    /// Simulation distance in chunks
    pub simulation_distance: u32,
}

/// Game difficulty
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

/// World statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldStatistics {
    /// Total chunks generated
    pub chunks_generated: u64,
    /// Total blocks placed
    pub blocks_placed: u64,
    /// Total blocks broken
    pub blocks_broken: u64,
    /// Total entities spawned
    pub entities_spawned: u64,
    /// Peak player count
    pub peak_player_count: u32,
    /// Total player joins
    pub total_joins: u64,
}

impl WorldMetadata {
    /// Create new world metadata
    pub fn new(name: String, seed: u64, game_version: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            version: SAVE_VERSION,
            name,
            seed,
            created_at: now,
            modified_at: now,
            play_time: 0,
            game_version,
            spawn_point: Vec3::new(0.0, 100.0, 0.0),
            world_bounds: None,
            game_rules: GameRules::default(),
            statistics: WorldStatistics::default(),
            custom_properties: HashMap::new(),
        }
    }
    
    /// Update modified timestamp
    pub fn touch(&mut self) {
        self.modified_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// Add play time
    pub fn add_play_time(&mut self, seconds: u64) {
        self.play_time += seconds;
    }
    
    /// Set custom property
    pub fn set_property(&mut self, key: String, value: String) {
        self.custom_properties.insert(key, value);
    }
    
    /// Get custom property
    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.custom_properties.get(key)
    }
    
    /// Check if world is compatible with current version
    pub fn is_compatible(&self) -> bool {
        SAVE_VERSION.is_compatible_with(&self.version)
    }
    
    /// Check if world needs migration
    pub fn needs_migration(&self) -> bool {
        SAVE_VERSION.needs_migration(&self.version)
    }
    
    /// Get world age in seconds
    pub fn age(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now.saturating_sub(self.created_at)
    }
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            pvp_enabled: true,
            keep_inventory: false,
            mob_spawning: true,
            fire_spread: true,
            tnt_enabled: true,
            day_night_cycle: true,
            weather_cycle: true,
            difficulty: Difficulty::Normal,
            max_players: 20,
            view_distance: 8,
            simulation_distance: 8,
        }
    }
}

/// Extended metadata for server worlds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetadata {
    /// Base world metadata
    pub world: WorldMetadata,
    /// Server version
    pub server_version: String,
    /// Server MOTD
    pub motd: String,
    /// Whitelist enabled
    pub whitelist_enabled: bool,
    /// Online mode (authentication)
    pub online_mode: bool,
    /// Resource pack URL
    pub resource_pack_url: Option<String>,
    /// Resource pack hash
    pub resource_pack_hash: Option<String>,
    /// Banned players
    pub banned_players: Vec<BannedPlayer>,
    /// Whitelisted players
    pub whitelisted_players: Vec<String>,
}

/// Banned player entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannedPlayer {
    pub uuid: String,
    pub username: String,
    pub reason: String,
    pub banned_by: String,
    pub banned_at: u64,
    pub expires_at: Option<u64>,
}

/// Metadata validation
pub fn validate_metadata(metadata: &WorldMetadata) -> Result<(), String> {
    // Validate name
    if metadata.name.is_empty() {
        return Err("World name cannot be empty".to_string());
    }
    
    if metadata.name.len() > 255 {
        return Err("World name too long".to_string());
    }
    
    // Validate game rules
    if metadata.game_rules.max_players == 0 {
        return Err("Max players must be at least 1".to_string());
    }
    
    if metadata.game_rules.view_distance > 32 {
        return Err("View distance too large".to_string());
    }
    
    // Validate spawn point
    if !metadata.spawn_point.is_finite() {
        return Err("Invalid spawn point".to_string());
    }
    
    // Validate world bounds if present
    if let Some(bounds) = &metadata.world_bounds {
        if bounds.min_x >= bounds.max_x || 
           bounds.min_y >= bounds.max_y || 
           bounds.min_z >= bounds.max_z {
            return Err("Invalid world bounds".to_string());
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_save_version_compatibility() {
        let v1 = SaveVersion::new(1, 0, 0);
        let v2 = SaveVersion::new(1, 0, 5);
        let v3 = SaveVersion::new(1, 1, 0);
        let v4 = SaveVersion::new(2, 0, 0);
        
        assert!(v1.is_compatible_with(&v1));
        assert!(v1.is_compatible_with(&v2)); // Same major.minor
        assert!(!v1.is_compatible_with(&v3)); // Newer minor
        assert!(!v1.is_compatible_with(&v4)); // Different major
    }
    
    #[test]
    fn test_world_metadata_creation() {
        let metadata = WorldMetadata::new(
            "Test World".to_string(),
            12345,
            "1.0.0".to_string(),
        );
        
        assert_eq!(metadata.name, "Test World");
        assert_eq!(metadata.seed, 12345);
        assert_eq!(metadata.play_time, 0);
        assert!(metadata.is_compatible());
    }
    
    #[test]
    fn test_metadata_validation() {
        let mut metadata = WorldMetadata::new(
            "Valid World".to_string(),
            42,
            "1.0.0".to_string(),
        );
        
        assert!(validate_metadata(&metadata).is_ok());
        
        // Test invalid cases
        metadata.name = "".to_string();
        assert!(validate_metadata(&metadata).is_err());
        
        metadata.name = "Valid".to_string();
        metadata.game_rules.max_players = 0;
        assert!(validate_metadata(&metadata).is_err());
    }
}