//! State validation system for persistence integrity
//!
//! This module validates state consistency between network-replicated data
//! and persisted data to prevent corruption and ensure data integrity.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::time::{Duration, Instant};

use crate::persistence::{PersistenceResult, PersistenceError};
use crate::world::{ChunkPos, World};

/// Types of validation that can be performed
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationType {
    /// Validate chunk data consistency
    ChunkData,
    /// Validate player data consistency
    PlayerData,
    /// Validate world metadata consistency
    WorldMetadata,
    /// Full validation of all data
    Full,
}

/// Validation result for a single check
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub validation_type: ValidationType,
    pub target_id: String,
    pub success: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub checked_at: Instant,
    pub duration: Duration,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub error_type: ValidationErrorType,
    pub description: String,
    pub field_path: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

/// Types of validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    /// Data corruption detected
    DataCorruption,
    /// Checksum mismatch
    ChecksumMismatch,
    /// Version mismatch
    VersionMismatch,
    /// Missing required data
    MissingData,
    /// Invalid data format
    InvalidFormat,
    /// State inconsistency between network and persistence
    StateInconsistency,
    /// Timestamp inconsistency
    TimestampInconsistency,
}

/// Validation warning (non-critical issues)
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub warning_type: ValidationWarningType,
    pub description: String,
    pub field_path: Option<String>,
}

/// Types of validation warnings
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarningType {
    /// Data appears outdated
    OutdatedData,
    /// Performance impact detected
    PerformanceImpact,
    /// Suboptimal data structure
    SuboptimalStructure,
    /// Potential future issue
    PotentialIssue,
}

/// State snapshot for validation
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub chunk_states: HashMap<ChunkPos, ChunkState>,
    pub player_states: HashMap<String, PlayerState>,
    pub world_metadata: WorldMetadataState,
    pub created_at: Instant,
    pub checksum: u64,
}

/// Chunk state information
#[derive(Debug, Clone)]
pub struct ChunkState {
    pub position: ChunkPos,
    pub blocks_checksum: u64,
    pub entities_checksum: u64,
    pub modified_at: Instant,
    pub network_version: u32,
    pub persistence_version: u32,
    pub block_count: usize,
    pub entity_count: usize,
}

/// Player state information
#[derive(Debug, Clone)]
pub struct PlayerState {
    pub uuid: String,
    pub position: (f64, f64, f64),
    pub inventory_checksum: u64,
    pub health: f32,
    pub network_timestamp: Instant,
    pub persistence_timestamp: Instant,
    pub network_version: u32,
    pub persistence_version: u32,
}

/// World metadata state
#[derive(Debug, Clone)]
pub struct WorldMetadataState {
    pub world_name: String,
    pub world_version: u32,
    pub seed: i64,
    pub spawn_position: (i32, i32, i32),
    pub total_chunks: usize,
    pub total_players: usize,
    pub created_at: Instant,
    pub last_saved_at: Instant,
}

/// Configuration for state validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Enable automatic validation
    pub auto_validate: bool,
    /// Validation interval for auto-validation
    pub validation_interval: Duration,
    /// Maximum age for snapshots before they're considered stale
    pub max_snapshot_age: Duration,
    /// Enable checksum validation
    pub enable_checksums: bool,
    /// Enable deep validation (slower but more thorough)
    pub enable_deep_validation: bool,
    /// Maximum time to spend on validation
    pub max_validation_time: Duration,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            auto_validate: true,
            validation_interval: Duration::from_secs(60),
            max_snapshot_age: Duration::from_secs(300),
            enable_checksums: true,
            enable_deep_validation: false,
            max_validation_time: Duration::from_secs(10),
        }
    }
}

/// State validator for ensuring persistence integrity
#[derive(Debug)]
pub struct StateValidator {
    config: ValidationConfig,
    network_snapshots: HashMap<String, StateSnapshot>,
    persistence_snapshots: HashMap<String, StateSnapshot>,
    validation_history: Vec<ValidationResult>,
    last_validation: Option<Instant>,
}

impl StateValidator {
    /// Create a new state validator
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            network_snapshots: HashMap::new(),
            persistence_snapshots: HashMap::new(),
            validation_history: Vec::new(),
            last_validation: None,
        }
    }
    
    /// Take a snapshot of the current network state
    pub fn take_network_snapshot(&mut self, world: &World, snapshot_id: String) -> PersistenceResult<()> {
        let snapshot = self.create_state_snapshot(world, true)?;
        self.network_snapshots.insert(snapshot_id, snapshot);
        Ok(())
    }
    
    /// Take a snapshot of the current persistence state
    pub fn take_persistence_snapshot(&mut self, world: &World, snapshot_id: String) -> PersistenceResult<()> {
        let snapshot = self.create_state_snapshot(world, false)?;
        self.persistence_snapshots.insert(snapshot_id, snapshot);
        Ok(())
    }
    
    /// Validate consistency between network and persistence states
    pub fn validate_consistency(&mut self, snapshot_id: &str) -> PersistenceResult<ValidationResult> {
        let start_time = Instant::now();
        
        let network_snapshot = self.network_snapshots.get(snapshot_id)
            .ok_or_else(|| PersistenceError::CorruptedData(
                format!("Network snapshot '{}' not found", snapshot_id)))?;
        
        let persistence_snapshot = self.persistence_snapshots.get(snapshot_id)
            .ok_or_else(|| PersistenceError::CorruptedData(
                format!("Persistence snapshot '{}' not found", snapshot_id)))?;
        
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Validate chunk consistency
        self.validate_chunk_consistency(&network_snapshot.chunk_states, 
                                      &persistence_snapshot.chunk_states, 
                                      &mut errors, &mut warnings);
        
        // Validate player consistency
        self.validate_player_consistency(&network_snapshot.player_states,
                                       &persistence_snapshot.player_states,
                                       &mut errors, &mut warnings);
        
        // Validate world metadata consistency
        self.validate_metadata_consistency(&network_snapshot.world_metadata,
                                         &persistence_snapshot.world_metadata,
                                         &mut errors, &mut warnings);
        
        // Check overall checksums
        if self.config.enable_checksums {
            if network_snapshot.checksum != persistence_snapshot.checksum {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::ChecksumMismatch,
                    description: "Overall state checksum mismatch".to_string(),
                    field_path: Some("checksum".to_string()),
                    expected: Some(network_snapshot.checksum.to_string()),
                    actual: Some(persistence_snapshot.checksum.to_string()),
                });
            }
        }
        
        let duration = start_time.elapsed();
        let result = ValidationResult {
            validation_type: ValidationType::Full,
            target_id: snapshot_id.to_string(),
            success: errors.is_empty(),
            errors,
            warnings,
            checked_at: start_time,
            duration,
        };
        
        self.validation_history.push(result.clone());
        self.last_validation = Some(start_time);
        
        Ok(result)
    }
    
    /// Validate a specific chunk's consistency
    pub fn validate_chunk(&mut self, chunk_pos: ChunkPos, world: &World) -> PersistenceResult<ValidationResult> {
        let start_time = Instant::now();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Get network chunk state
        let network_state = self.get_chunk_state_from_world(chunk_pos, world, true)?;
        
        // Get persistence chunk state (simulated for now)
        let persistence_state = self.get_chunk_state_from_world(chunk_pos, world, false)?;
        
        // Compare states
        if network_state.blocks_checksum != persistence_state.blocks_checksum {
            errors.push(ValidationError {
                error_type: ValidationErrorType::ChecksumMismatch,
                description: format!("Block checksum mismatch for chunk {:?}", chunk_pos),
                field_path: Some("blocks_checksum".to_string()),
                expected: Some(network_state.blocks_checksum.to_string()),
                actual: Some(persistence_state.blocks_checksum.to_string()),
            });
        }
        
        if network_state.entities_checksum != persistence_state.entities_checksum {
            errors.push(ValidationError {
                error_type: ValidationErrorType::ChecksumMismatch,
                description: format!("Entity checksum mismatch for chunk {:?}", chunk_pos),
                field_path: Some("entities_checksum".to_string()),
                expected: Some(network_state.entities_checksum.to_string()),
                actual: Some(persistence_state.entities_checksum.to_string()),
            });
        }
        
        // Check version consistency
        if network_state.network_version != persistence_state.persistence_version {
            warnings.push(ValidationWarning {
                warning_type: ValidationWarningType::OutdatedData,
                description: format!("Version mismatch for chunk {:?}", chunk_pos),
                field_path: Some("version".to_string()),
            });
        }
        
        let duration = start_time.elapsed();
        let result = ValidationResult {
            validation_type: ValidationType::ChunkData,
            target_id: format!("chunk_{}_{}_{}", chunk_pos.x, chunk_pos.y, chunk_pos.z),
            success: errors.is_empty(),
            errors,
            warnings,
            checked_at: start_time,
            duration,
        };
        
        self.validation_history.push(result.clone());
        
        Ok(result)
    }
    
    /// Auto-validate if needed
    pub fn auto_validate_if_needed(&mut self, world: &World) -> PersistenceResult<Option<ValidationResult>> {
        if !self.config.auto_validate {
            return Ok(None);
        }
        
        let should_validate = match self.last_validation {
            Some(last) => Instant::now().duration_since(last) >= self.config.validation_interval,
            None => true,
        };
        
        if should_validate {
            let snapshot_id = format!("auto_{}", Instant::now().elapsed().as_millis());
            self.take_network_snapshot(world, snapshot_id.clone())?;
            self.take_persistence_snapshot(world, snapshot_id.clone())?;
            let result = self.validate_consistency(&snapshot_id)?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
    
    /// Create a state snapshot
    fn create_state_snapshot(&self, world: &World, is_network_state: bool) -> PersistenceResult<StateSnapshot> {
        let mut chunk_states = HashMap::new();
        let mut player_states = HashMap::new();
        
        // Get chunk positions (dummy implementation)
        let chunk_positions = vec![
            ChunkPos { x: 0, y: 0, z: 0 },
            ChunkPos { x: 1, y: 0, z: 0 },
            ChunkPos { x: 0, y: 0, z: 1 },
        ];
        
        // Create chunk states
        for pos in chunk_positions {
            let chunk_state = self.get_chunk_state_from_world(pos, world, is_network_state)?;
            chunk_states.insert(pos, chunk_state);
        }
        
        // Create dummy player states
        for i in 0..3 {
            let uuid = format!("player_{}", i);
            let player_state = PlayerState {
                uuid: uuid.clone(),
                position: (i as f64 * 10.0, 64.0, 0.0),
                inventory_checksum: self.calculate_hash(&format!("inventory_{}", i)),
                health: 20.0,
                network_timestamp: Instant::now(),
                persistence_timestamp: Instant::now(),
                network_version: if is_network_state { 1 } else { 1 },
                persistence_version: 1,
            };
            player_states.insert(uuid, player_state);
        }
        
        // Create world metadata
        let world_metadata = WorldMetadataState {
            world_name: "test_world".to_string(),
            world_version: 1,
            seed: 12345,
            spawn_position: (0, 64, 0),
            total_chunks: chunk_states.len(),
            total_players: player_states.len(),
            created_at: Instant::now(),
            last_saved_at: Instant::now(),
        };
        
        // Calculate overall checksum
        let checksum = self.calculate_snapshot_checksum(&chunk_states, &player_states, &world_metadata);
        
        Ok(StateSnapshot {
            chunk_states,
            player_states,
            world_metadata,
            created_at: Instant::now(),
            checksum,
        })
    }
    
    /// Get chunk state from world
    fn get_chunk_state_from_world(&self, pos: ChunkPos, _world: &World, is_network: bool) -> PersistenceResult<ChunkState> {
        // Dummy implementation - in real code would extract actual chunk data
        Ok(ChunkState {
            position: pos,
            blocks_checksum: self.calculate_hash(&format!("blocks_{}_{}_{}", pos.x, pos.y, pos.z)),
            entities_checksum: self.calculate_hash(&format!("entities_{}_{}_{}", pos.x, pos.y, pos.z)),
            modified_at: Instant::now(),
            network_version: if is_network { 1 } else { 1 },
            persistence_version: 1,
            block_count: 4096, // 16x16x16 chunk
            entity_count: 5,
        })
    }
    
    /// Calculate hash for data
    fn calculate_hash(&self, data: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Calculate snapshot checksum
    fn calculate_snapshot_checksum(
        &self,
        chunks: &HashMap<ChunkPos, ChunkState>,
        players: &HashMap<String, PlayerState>,
        metadata: &WorldMetadataState,
    ) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        // Hash chunk data
        for (pos, state) in chunks {
            pos.hash(&mut hasher);
            state.blocks_checksum.hash(&mut hasher);
            state.entities_checksum.hash(&mut hasher);
        }
        
        // Hash player data
        for (uuid, state) in players {
            uuid.hash(&mut hasher);
            state.inventory_checksum.hash(&mut hasher);
            (state.health as u32).hash(&mut hasher);
        }
        
        // Hash metadata
        metadata.world_name.hash(&mut hasher);
        metadata.seed.hash(&mut hasher);
        metadata.world_version.hash(&mut hasher);
        
        hasher.finish()
    }
    
    /// Validate chunk consistency
    fn validate_chunk_consistency(
        &self,
        network_chunks: &HashMap<ChunkPos, ChunkState>,
        persistence_chunks: &HashMap<ChunkPos, ChunkState>,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        // Check for missing chunks
        for pos in network_chunks.keys() {
            if !persistence_chunks.contains_key(pos) {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::MissingData,
                    description: format!("Chunk {:?} missing from persistence", pos),
                    field_path: Some(format!("chunks.{:?}", pos)),
                    expected: Some("present".to_string()),
                    actual: Some("missing".to_string()),
                });
            }
        }
        
        // Check for extra chunks in persistence
        for pos in persistence_chunks.keys() {
            if !network_chunks.contains_key(pos) {
                warnings.push(ValidationWarning {
                    warning_type: ValidationWarningType::OutdatedData,
                    description: format!("Chunk {:?} in persistence but not in network state", pos),
                    field_path: Some(format!("chunks.{:?}", pos)),
                });
            }
        }
        
        // Compare existing chunks
        for (pos, network_chunk) in network_chunks {
            if let Some(persistence_chunk) = persistence_chunks.get(pos) {
                if network_chunk.blocks_checksum != persistence_chunk.blocks_checksum {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::StateInconsistency,
                        description: format!("Block data inconsistency in chunk {:?}", pos),
                        field_path: Some(format!("chunks.{:?}.blocks", pos)),
                        expected: Some(network_chunk.blocks_checksum.to_string()),
                        actual: Some(persistence_chunk.blocks_checksum.to_string()),
                    });
                }
                
                if network_chunk.entities_checksum != persistence_chunk.entities_checksum {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::StateInconsistency,
                        description: format!("Entity data inconsistency in chunk {:?}", pos),
                        field_path: Some(format!("chunks.{:?}.entities", pos)),
                        expected: Some(network_chunk.entities_checksum.to_string()),
                        actual: Some(persistence_chunk.entities_checksum.to_string()),
                    });
                }
            }
        }
    }
    
    /// Validate player consistency
    fn validate_player_consistency(
        &self,
        network_players: &HashMap<String, PlayerState>,
        persistence_players: &HashMap<String, PlayerState>,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) {
        for (uuid, network_player) in network_players {
            if let Some(persistence_player) = persistence_players.get(uuid) {
                // Check inventory consistency
                if network_player.inventory_checksum != persistence_player.inventory_checksum {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::StateInconsistency,
                        description: format!("Inventory inconsistency for player {}", uuid),
                        field_path: Some(format!("players.{}.inventory", uuid)),
                        expected: Some(network_player.inventory_checksum.to_string()),
                        actual: Some(persistence_player.inventory_checksum.to_string()),
                    });
                }
                
                // Check health consistency (with tolerance)
                let health_diff = (network_player.health - persistence_player.health).abs();
                if health_diff > 0.1 {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::StateInconsistency,
                        description: format!("Health inconsistency for player {}", uuid),
                        field_path: Some(format!("players.{}.health", uuid)),
                        expected: Some(network_player.health.to_string()),
                        actual: Some(persistence_player.health.to_string()),
                    });
                }
            }
        }
    }
    
    /// Validate metadata consistency
    fn validate_metadata_consistency(
        &self,
        network_metadata: &WorldMetadataState,
        persistence_metadata: &WorldMetadataState,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        if network_metadata.world_version != persistence_metadata.world_version {
            warnings.push(ValidationWarning {
                warning_type: ValidationWarningType::OutdatedData,
                description: "World version mismatch".to_string(),
                field_path: Some("metadata.world_version".to_string()),
            });
        }
        
        if network_metadata.seed != persistence_metadata.seed {
            errors.push(ValidationError {
                error_type: ValidationErrorType::StateInconsistency,
                description: "World seed mismatch".to_string(),
                field_path: Some("metadata.seed".to_string()),
                expected: Some(network_metadata.seed.to_string()),
                actual: Some(persistence_metadata.seed.to_string()),
            });
        }
    }
    
    /// Get validation statistics
    pub fn get_validation_stats(&self) -> ValidationStats {
        let total_validations = self.validation_history.len();
        let successful_validations = self.validation_history.iter()
            .filter(|r| r.success)
            .count();
        
        let average_duration = if total_validations > 0 {
            let total_time: Duration = self.validation_history.iter()
                .map(|r| r.duration)
                .sum();
            total_time / total_validations as u32
        } else {
            Duration::from_millis(0)
        };
        
        ValidationStats {
            total_validations,
            successful_validations,
            failed_validations: total_validations - successful_validations,
            average_duration,
            last_validation: self.last_validation,
            network_snapshots: self.network_snapshots.len(),
            persistence_snapshots: self.persistence_snapshots.len(),
        }
    }
    
    /// Clean up old snapshots
    pub fn cleanup_old_snapshots(&mut self) {
        let cutoff_time = Instant::now() - self.config.max_snapshot_age;
        
        self.network_snapshots.retain(|_, snapshot| {
            snapshot.created_at > cutoff_time
        });
        
        self.persistence_snapshots.retain(|_, snapshot| {
            snapshot.created_at > cutoff_time
        });
    }
}

/// Statistics for validation operations
#[derive(Debug, Clone)]
pub struct ValidationStats {
    pub total_validations: usize,
    pub successful_validations: usize,
    pub failed_validations: usize,
    pub average_duration: Duration,
    pub last_validation: Option<Instant>,
    pub network_snapshots: usize,
    pub persistence_snapshots: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_world() -> World {
        World::new(16) // 16x16 chunk size
    }
    
    #[test]
    fn test_state_validator_creation() {
        let config = ValidationConfig::default();
        let validator = StateValidator::new(config);
        
        let stats = validator.get_validation_stats();
        assert_eq!(stats.total_validations, 0);
        assert_eq!(stats.network_snapshots, 0);
        assert_eq!(stats.persistence_snapshots, 0);
    }
    
    #[test]
    fn test_take_snapshots() {
        let config = ValidationConfig::default();
        let mut validator = StateValidator::new(config);
        let world = create_test_world();
        
        // Take snapshots
        validator.take_network_snapshot(&world, "test_snapshot".to_string())
            .expect("Failed to take network snapshot");
        validator.take_persistence_snapshot(&world, "test_snapshot".to_string())
            .expect("Failed to take persistence snapshot");
        
        let stats = validator.get_validation_stats();
        assert_eq!(stats.network_snapshots, 1);
        assert_eq!(stats.persistence_snapshots, 1);
    }
    
    #[test]
    fn test_validate_consistency() {
        let config = ValidationConfig::default();
        let mut validator = StateValidator::new(config);
        let world = create_test_world();
        
        // Take snapshots
        validator.take_network_snapshot(&world, "consistency_test".to_string())
            .expect("Failed to take network snapshot");
        validator.take_persistence_snapshot(&world, "consistency_test".to_string())
            .expect("Failed to take persistence snapshot");
        
        // Validate consistency
        let result = validator.validate_consistency("consistency_test")
            .expect("Failed to validate consistency");
        
        // Should succeed since we're using the same world for both snapshots
        assert!(result.success);
        assert_eq!(result.validation_type, ValidationType::Full);
    }
    
    #[test]
    fn test_chunk_validation() {
        let config = ValidationConfig::default();
        let mut validator = StateValidator::new(config);
        let world = create_test_world();
        
        let chunk_pos = ChunkPos { x: 0, y: 0, z: 0 };
        let result = validator.validate_chunk(chunk_pos, &world)
            .expect("Failed to validate chunk");
        
        assert_eq!(result.validation_type, ValidationType::ChunkData);
        assert!(result.target_id.contains("chunk_0_0_0"));
    }
    
    #[test]
    fn test_cleanup_old_snapshots() {
        let config = ValidationConfig {
            max_snapshot_age: Duration::from_millis(100),
            ..Default::default()
        };
        let mut validator = StateValidator::new(config);
        let world = create_test_world();
        
        // Take a snapshot
        validator.take_network_snapshot(&world, "old_snapshot".to_string())
            .expect("Failed to take network snapshot");
        
        // Wait for it to become old
        std::thread::sleep(Duration::from_millis(150));
        
        // Take another snapshot
        validator.take_network_snapshot(&world, "new_snapshot".to_string())
            .expect("Failed to take network snapshot");
        
        assert_eq!(validator.get_validation_stats().network_snapshots, 2);
        
        // Cleanup old snapshots
        validator.cleanup_old_snapshots();
        
        // Should only have the new snapshot
        assert_eq!(validator.get_validation_stats().network_snapshots, 1);
    }
}