use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use glam::{Vec3, Quat};

use crate::inventory::{
    PlayerInventoryData, ItemStackData, create_item_stack, 
    create_empty_slot, create_slot_with_item, SlotType
};
use crate::persistence::{PersistenceResult, PersistenceError};

/// Player data that needs to be persisted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    /// Player UUID
    pub uuid: String,
    /// Player username
    pub username: String,
    /// Last known position
    pub position: Vec3,
    /// Last known rotation
    pub rotation: Quat,
    /// Player health
    pub health: f32,
    /// Player hunger/food level
    pub hunger: f32,
    /// Experience points
    pub experience: u32,
    /// Experience level
    pub level: u32,
    /// Game mode
    pub game_mode: GameMode,
    /// Spawn position
    pub spawn_position: Option<Vec3>,
    /// Last login timestamp
    pub last_login: u64,
    /// Total play time in seconds
    pub play_time: u64,
    /// Player statistics
    pub stats: PlayerStats,
}

/// Player save data including inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSaveData {
    /// Core player data
    pub player_data: PlayerData,
    /// Inventory contents
    pub inventory: InventoryData,
    /// Active potion effects
    pub effects: Vec<PotionEffect>,
    /// Unlocked achievements
    pub achievements: Vec<String>,
    /// Custom player tags
    pub tags: Vec<String>,
}

/// Serializable inventory data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryData {
    /// Main inventory slots
    pub main_slots: Vec<Option<ItemStackData>>,
    /// Hotbar slots (indices into main inventory)
    pub hotbar_indices: [usize; 9],
    /// Armor slots
    pub armor_slots: [Option<ItemStackData>; 4],
    /// Offhand slot
    pub offhand_slot: Option<ItemStackData>,
    /// Currently selected hotbar slot
    pub selected_slot: usize,
}

/// Player game mode
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

/// Potion effect data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotionEffect {
    pub effect_type: String,
    pub amplifier: u8,
    pub duration: f32,
}

/// Player statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerStats {
    pub blocks_broken: u64,
    pub blocks_placed: u64,
    pub distance_walked: f64,
    pub distance_sprinted: f64,
    pub distance_fallen: f64,
    pub distance_climbed: f64,
    pub distance_flown: f64,
    pub jumps: u64,
    pub deaths: u32,
    pub mob_kills: u32,
    pub player_kills: u32,
    pub damage_dealt: f64,
    pub damage_taken: f64,
    pub play_time: u64,
}

impl PlayerData {
    /// Create new player data with defaults
    pub fn new(uuid: String, username: String) -> Self {
        Self {
            uuid,
            username,
            position: Vec3::new(0.0, 100.0, 0.0),
            rotation: Quat::IDENTITY,
            health: 20.0,
            hunger: 20.0,
            experience: 0,
            level: 0,
            game_mode: GameMode::Survival,
            spawn_position: None,
            last_login: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
            play_time: 0,
            stats: PlayerStats::default(),
        }
    }
    
    /// Update play time based on session duration
    pub fn update_play_time(&mut self, session_seconds: u64) {
        self.play_time += session_seconds;
        self.stats.play_time = self.play_time;
    }
}

impl PlayerSaveData {
    /// Create save data from player components
    pub fn from_player(
        player_data: PlayerData,
        inventory: &PlayerInventoryData,
    ) -> Self {
        Self {
            player_data,
            inventory: InventoryData::from_inventory(inventory),
            effects: Vec::new(),
            achievements: Vec::new(),
            tags: Vec::new(),
        }
    }
    
    /// Save player data to file
    pub fn save<P: AsRef<Path>>(&self, save_dir: P) -> PersistenceResult<()> {
        let save_dir = save_dir.as_ref();
        let player_dir = save_dir.join("players");
        
        // Ensure directory exists
        fs::create_dir_all(&player_dir)?;
        
        // Save to UUID-based file
        let file_path = player_dir.join(format!("{}.player", self.player_data.uuid));
        let data = bincode::serialize(self)?;
        
        // Write to temporary file first
        let temp_path = file_path.with_extension("tmp");
        fs::write(&temp_path, data)?;
        
        // Atomic rename
        fs::rename(temp_path, file_path)?;
        
        Ok(())
    }
    
    /// Load player data from file
    pub fn load<P: AsRef<Path>>(save_dir: P, uuid: &str) -> PersistenceResult<Self> {
        let save_dir = save_dir.as_ref();
        let file_path = save_dir.join("players").join(format!("{}.player", uuid));
        
        if !file_path.exists() {
            return Err(PersistenceError::IoError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "Player file not found")
            ));
        }
        
        let data = fs::read(file_path)?;
        let player_data: Self = bincode::deserialize(&data)?;
        
        // Validate data
        if player_data.player_data.uuid != uuid {
            return Err(PersistenceError::CorruptedData(
                "UUID mismatch in player data".to_string()
            ));
        }
        
        Ok(player_data)
    }
    
    /// Delete player data
    pub fn delete<P: AsRef<Path>>(save_dir: P, uuid: &str) -> PersistenceResult<()> {
        let save_dir = save_dir.as_ref();
        let file_path = save_dir.join("players").join(format!("{}.player", uuid));
        
        if file_path.exists() {
            fs::remove_file(file_path)?;
        }
        
        Ok(())
    }
    
    /// List all saved players
    pub fn list_players<P: AsRef<Path>>(save_dir: P) -> PersistenceResult<Vec<PlayerInfo>> {
        let save_dir = save_dir.as_ref();
        let player_dir = save_dir.join("players");
        
        if !player_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut players = Vec::new();
        
        for entry in fs::read_dir(player_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension() == Some("player".as_ref()) {
                if let Ok(data) = fs::read(&path) {
                    if let Ok(player_data) = bincode::deserialize::<PlayerSaveData>(&data) {
                        players.push(PlayerInfo {
                            uuid: player_data.player_data.uuid.clone(),
                            username: player_data.player_data.username.clone(),
                            last_login: player_data.player_data.last_login,
                            play_time: player_data.player_data.play_time,
                        });
                    }
                }
            }
        }
        
        Ok(players)
    }
}

impl InventoryData {
    /// Create from player inventory
    pub fn from_inventory(inventory: &PlayerInventoryData) -> Self {
        // Convert inventory slot data to serializable format
        let mut main_slots = Vec::with_capacity(36);
        for slot in &inventory.slots {
            if slot.has_item != 0 {
                main_slots.push(Some(slot.item));
            } else {
                main_slots.push(None);
            }
        }
        
        Self {
            main_slots,
            hotbar_indices: [0, 1, 2, 3, 4, 5, 6, 7, 8], // Fixed hotbar order
            armor_slots: [None, None, None, None], // TODO: Add armor support
            offhand_slot: None,
            selected_slot: inventory.selected_hotbar_slot as usize,
        }
    }
    
    /// Apply to player inventory
    pub fn apply_to_inventory(&self, inventory: &mut PlayerInventoryData) {
        // Clear all slots
        for slot in &mut inventory.slots {
            *slot = create_empty_slot(
                if slot.slot_type == 1 { 
                    SlotType::Hotbar 
                } else { 
                    SlotType::Normal 
                }
            );
        }
        
        // Restore items to slots
        for (i, item_option) in self.main_slots.iter().enumerate() {
            if i < inventory.slots.len() {
                if let Some(item_stack) = item_option {
                    inventory.slots[i] = create_slot_with_item(
                        if i < 9 { 
                            SlotType::Hotbar 
                        } else { 
                            SlotType::Normal 
                        },
                        *item_stack
                    );
                }
            }
        }
        
        // Restore selected slot
        inventory.selected_hotbar_slot = self.selected_slot as u32;
    }
}

/// Basic player information for listing
#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub uuid: String,
    pub username: String,
    pub last_login: u64,
    pub play_time: u64,
}

/// Player data manager for handling multiple players
pub struct PlayerDataManager {
    save_dir: PathBuf,
}

impl PlayerDataManager {
    pub fn new<P: AsRef<Path>>(save_dir: P) -> Self {
        Self {
            save_dir: save_dir.as_ref().to_path_buf(),
        }
    }
    
    /// Save player data with automatic backup
    pub fn save_player(&self, player_data: &PlayerSaveData) -> PersistenceResult<()> {
        // Create backup of existing data
        let backup_path = self.save_dir
            .join("players")
            .join(format!("{}.player.bak", player_data.player_data.uuid));
        
        let player_path = self.save_dir
            .join("players")
            .join(format!("{}.player", player_data.player_data.uuid));
        
        if player_path.exists() {
            fs::copy(&player_path, &backup_path)?;
        }
        
        // Save new data
        player_data.save(&self.save_dir)?;
        
        // Remove backup on success
        if backup_path.exists() {
            let _ = fs::remove_file(backup_path);
        }
        
        Ok(())
    }
    
    /// Load player with fallback to backup
    pub fn load_player(&self, uuid: &str) -> PersistenceResult<PlayerSaveData> {
        // Try to load main file
        match PlayerSaveData::load(&self.save_dir, uuid) {
            Ok(data) => Ok(data),
            Err(_) => {
                // Try backup
                let backup_path = self.save_dir
                    .join("players")
                    .join(format!("{}.player.bak", uuid));
                
                if backup_path.exists() {
                    let data = fs::read(backup_path)?;
                    Ok(bincode::deserialize(&data)?)
                } else {
                    Err(PersistenceError::IoError(
                        std::io::Error::new(std::io::ErrorKind::NotFound, "Player not found")
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::inventory::init_inventory;
    
    #[test]
    fn test_player_save_load() {
        let temp_dir = TempDir::new().unwrap();
        
        let player_data = PlayerData::new(
            "test-uuid-123".to_string(),
            "TestPlayer".to_string(),
        );
        
        let inventory = init_inventory();
        let save_data = PlayerSaveData::from_player(player_data, &inventory);
        
        // Save
        save_data.save(temp_dir.path()).unwrap();
        
        // Load
        let loaded = PlayerSaveData::load(temp_dir.path(), "test-uuid-123").unwrap();
        
        assert_eq!(loaded.player_data.uuid, "test-uuid-123");
        assert_eq!(loaded.player_data.username, "TestPlayer");
    }
    
    #[test]
    fn test_list_players() {
        let temp_dir = TempDir::new().unwrap();
        
        // Save multiple players
        for i in 0..3 {
            let player_data = PlayerData::new(
                format!("uuid-{}", i),
                format!("Player{}", i),
            );
            let save_data = PlayerSaveData::from_player(player_data, &init_inventory());
            save_data.save(temp_dir.path()).unwrap();
        }
        
        // List players
        let players = PlayerSaveData::list_players(temp_dir.path()).unwrap();
        
        assert_eq!(players.len(), 3);
    }
}