//! Network validation system for world consistency
//!
//! This module provides validation functionality to ensure that save/load operations
//! maintain world consistency across network operations. It validates data integrity,
//! checks for conflicts, and ensures synchronization between multiple clients.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::network::{ChunkSaveStatus, LoadStatus, SaveStatus};
use crate::persistence::{PersistenceError, PersistenceResult};
use crate::{BlockId, ChunkPos, VoxelPos};

/// World consistency validation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationResult {
    Valid,
    Invalid(ValidationError),
    Warning(ValidationWarning),
}

/// Validation errors that indicate data corruption or inconsistency
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationError {
    ChunkChecksumMismatch {
        chunk_pos: ChunkPos,
        expected: u64,
        actual: u64,
    },
    PlayerPositionOutOfBounds {
        player_id: u32,
        position: Vec3,
    },
    BlockDataCorrupted {
        position: VoxelPos,
        block_id: BlockId,
    },
    SaveVersionMismatch {
        expected: u32,
        actual: u32,
    },
    NetworkStateMismatch {
        chunk_pos: ChunkPos,
        network_state: ChunkSaveStatus,
        disk_state: ChunkSaveStatus,
    },
    TimestampInconsistency {
        operation: String,
        timestamp: u64,
        current_time: u64,
    },
    DuplicateOperation {
        operation_id: u32,
        timestamp: u64,
    },
}

/// Validation warnings that indicate potential issues but not corruption
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationWarning {
    LargeTimestampGap {
        gap_seconds: u64,
        operation: String,
    },
    UnusualPlayerMovement {
        player_id: u32,
        distance: f32,
        time_delta_seconds: u64,
    },
    HighChunkChangeRate {
        chunk_pos: ChunkPos,
        changes_per_second: f32,
    },
    NetworkLatencyHigh {
        latency_ms: u64,
        threshold_ms: u64,
    },
}

/// Chunk validation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkValidationData {
    pub chunk_pos: ChunkPos,
    pub checksum: u64,
    pub last_modified: u64,
    pub block_count: u32,
    pub save_state: ChunkSaveStatus,
    pub network_timestamp: u64,
    pub validation_timestamp: u64,
}

/// Player validation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerValidationData {
    pub player_id: u32,
    pub uuid: String,
    pub last_position: Vec3,
    pub last_update: u64,
    pub movement_history: Vec<(Vec3, u64)>,
    pub save_state: SaveStatus,
}

/// World validation state
#[derive(Debug, Clone)]
pub struct WorldValidationState {
    pub chunks: HashMap<ChunkPos, ChunkValidationData>,
    pub players: HashMap<u32, PlayerValidationData>,
    pub operation_history: Vec<ValidationOperation>,
    pub last_full_validation: u64,
    pub validation_errors: Vec<ValidationError>,
    pub validation_warnings: Vec<ValidationWarning>,
}

/// Validation operation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOperation {
    pub operation_id: u32,
    pub operation_type: ValidationType,
    pub timestamp: u64,
    pub result: ValidationResult,
    pub duration_ms: u64,
}

/// Types of validation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    ChunkSave,
    ChunkLoad,
    PlayerSave,
    PlayerLoad,
    WorldSave,
    WorldLoad,
    NetworkSync,
    FullValidation,
}

/// Network validator for world consistency
#[derive(Debug)]
pub struct NetworkValidator {
    state: Arc<Mutex<WorldValidationState>>,
    config: ValidationConfig,
    next_operation_id: Arc<Mutex<u32>>,
}

/// Validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Enable chunk checksum validation
    pub enable_chunk_checksums: bool,
    /// Enable player movement validation
    pub enable_movement_validation: bool,
    /// Maximum allowed player movement speed (m/s)
    pub max_player_speed: f32,
    /// Maximum allowed network latency before warning
    pub max_network_latency: Duration,
    /// Interval for full world validation
    pub full_validation_interval: Duration,
    /// Maximum number of validation errors to store
    pub max_stored_errors: usize,
    /// Maximum number of validation warnings to store
    pub max_stored_warnings: usize,
    /// Enable timestamp validation
    pub enable_timestamp_validation: bool,
    /// Maximum allowed timestamp drift
    pub max_timestamp_drift: Duration,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enable_chunk_checksums: true,
            enable_movement_validation: true,
            max_player_speed: 50.0, // 50 m/s - generous for creative mode
            max_network_latency: Duration::from_millis(500),
            full_validation_interval: Duration::from_secs(300), // 5 minutes
            max_stored_errors: 100,
            max_stored_warnings: 200,
            enable_timestamp_validation: true,
            max_timestamp_drift: Duration::from_secs(60), // 1 minute
        }
    }
}

impl NetworkValidator {
    /// Create a new network validator
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(WorldValidationState {
                chunks: HashMap::new(),
                players: HashMap::new(),
                operation_history: Vec::new(),
                last_full_validation: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                validation_errors: Vec::new(),
                validation_warnings: Vec::new(),
            })),
            config,
            next_operation_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Validate chunk save operation
    pub fn validate_chunk_save(
        &self,
        chunk_pos: ChunkPos,
        checksum: u64,
        save_state: ChunkSaveStatus,
    ) -> PersistenceResult<ValidationResult> {
        let operation_id = self.get_next_operation_id()?;
        let start_time = std::time::SystemTime::now();

        let result = self.validate_chunk_internal(chunk_pos, checksum, save_state)?;

        // Record operation
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        self.record_operation(ValidationOperation {
            operation_id,
            operation_type: ValidationType::ChunkSave,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            result: result.clone(),
            duration_ms: duration,
        })?;

        Ok(result)
    }

    /// Validate chunk load operation  
    pub fn validate_chunk_load(
        &self,
        chunk_pos: ChunkPos,
        checksum: u64,
    ) -> PersistenceResult<ValidationResult> {
        let operation_id = self.get_next_operation_id()?;
        let start_time = std::time::SystemTime::now();

        let result = self.validate_chunk_internal(chunk_pos, checksum, ChunkSaveStatus::Loaded)?;

        // Record operation
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        self.record_operation(ValidationOperation {
            operation_id,
            operation_type: ValidationType::ChunkLoad,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            result: result.clone(),
            duration_ms: duration,
        })?;

        Ok(result)
    }

    /// Validate player save operation
    pub fn validate_player_save(
        &self,
        player_id: u32,
        position: Vec3,
        timestamp: u64,
    ) -> PersistenceResult<ValidationResult> {
        let operation_id = self.get_next_operation_id()?;
        let start_time = std::time::SystemTime::now();

        let result = self.validate_player_internal(player_id, position, timestamp)?;

        // Record operation
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        self.record_operation(ValidationOperation {
            operation_id,
            operation_type: ValidationType::PlayerSave,
            timestamp,
            result: result.clone(),
            duration_ms: duration,
        })?;

        Ok(result)
    }

    /// Validate network synchronization state
    pub fn validate_network_sync(
        &self,
        chunk_states: &HashMap<ChunkPos, ChunkSaveStatus>,
    ) -> PersistenceResult<Vec<ValidationResult>> {
        let mut results = Vec::new();
        let mut state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        for (chunk_pos, network_state) in chunk_states {
            if let Some(chunk_data) = state.chunks.get(chunk_pos) {
                if chunk_data.save_state != *network_state {
                    let error = ValidationError::NetworkStateMismatch {
                        chunk_pos: *chunk_pos,
                        network_state: *network_state,
                        disk_state: chunk_data.save_state,
                    };
                    results.push(ValidationResult::Invalid(error.clone()));

                    // Store error
                    state.validation_errors.push(error);
                    if state.validation_errors.len() > self.config.max_stored_errors {
                        state.validation_errors.remove(0);
                    }
                }
            }
        }

        if results.is_empty() {
            results.push(ValidationResult::Valid);
        }

        Ok(results)
    }

    /// Perform full world validation
    pub fn validate_full_world(&self) -> PersistenceResult<ValidationResult> {
        let operation_id = self.get_next_operation_id()?;
        let start_time = std::time::SystemTime::now();

        let mut state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate all chunks
        for (chunk_pos, chunk_data) in &state.chunks {
            if self.config.enable_chunk_checksums {
                // Check for corrupted checksums (placeholder logic)
                if chunk_data.checksum == 0 {
                    errors.push(ValidationError::ChunkChecksumMismatch {
                        chunk_pos: *chunk_pos,
                        expected: 1, // Placeholder
                        actual: 0,
                    });
                }
            }
        }

        // Validate all players
        for (player_id, player_data) in &state.players {
            if self.config.enable_movement_validation {
                self.validate_player_movement(*player_id, player_data, &mut warnings);
            }

            self.validate_player_position(*player_id, player_data.last_position, &mut errors);
        }

        // Update validation time
        state.last_full_validation = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Store results
        state.validation_errors.extend(errors.clone());
        state.validation_warnings.extend(warnings.clone());

        // Trim stored results
        let error_len = state.validation_errors.len();
        if error_len > self.config.max_stored_errors {
            state
                .validation_errors
                .drain(0..error_len - self.config.max_stored_errors);
        }
        let warning_len = state.validation_warnings.len();
        if warning_len > self.config.max_stored_warnings {
            state
                .validation_warnings
                .drain(0..warning_len - self.config.max_stored_warnings);
        }

        let result = if !errors.is_empty() {
            ValidationResult::Invalid(errors[0].clone())
        } else if !warnings.is_empty() {
            ValidationResult::Warning(warnings[0].clone())
        } else {
            ValidationResult::Valid
        };

        // Record operation
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        drop(state); // Release lock before recording
        self.record_operation(ValidationOperation {
            operation_id,
            operation_type: ValidationType::FullValidation,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            result: result.clone(),
            duration_ms: duration,
        })?;

        Ok(result)
    }

    /// Register chunk data for validation
    pub fn register_chunk(
        &self,
        chunk_pos: ChunkPos,
        checksum: u64,
        save_state: ChunkSaveStatus,
    ) -> PersistenceResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        let chunk_data = ChunkValidationData {
            chunk_pos,
            checksum,
            last_modified: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            block_count: 0, // TODO: Calculate actual block count
            save_state,
            network_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            validation_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        state.chunks.insert(chunk_pos, chunk_data);
        Ok(())
    }

    /// Register player data for validation
    pub fn register_player(
        &self,
        player_id: u32,
        uuid: String,
        position: Vec3,
    ) -> PersistenceResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let player_data = PlayerValidationData {
            player_id,
            uuid,
            last_position: position,
            last_update: current_time,
            movement_history: vec![(position, current_time)],
            save_state: SaveStatus::Completed,
        };

        state.players.insert(player_id, player_data);
        Ok(())
    }

    /// Get validation statistics
    pub fn get_validation_stats(&self) -> PersistenceResult<ValidationStats> {
        let state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        Ok(ValidationStats {
            total_chunks: state.chunks.len(),
            total_players: state.players.len(),
            total_operations: state.operation_history.len(),
            total_errors: state.validation_errors.len(),
            total_warnings: state.validation_warnings.len(),
            last_full_validation: state.last_full_validation,
        })
    }

    /// Get recent validation errors
    pub fn get_recent_errors(&self, limit: usize) -> PersistenceResult<Vec<ValidationError>> {
        let state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        let start = if state.validation_errors.len() > limit {
            state.validation_errors.len() - limit
        } else {
            0
        };

        Ok(state.validation_errors[start..].to_vec())
    }

    /// Get recent validation warnings
    pub fn get_recent_warnings(&self, limit: usize) -> PersistenceResult<Vec<ValidationWarning>> {
        let state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        let start = if state.validation_warnings.len() > limit {
            state.validation_warnings.len() - limit
        } else {
            0
        };

        Ok(state.validation_warnings[start..].to_vec())
    }

    // Internal validation methods
    fn validate_chunk_internal(
        &self,
        chunk_pos: ChunkPos,
        checksum: u64,
        save_state: ChunkSaveStatus,
    ) -> PersistenceResult<ValidationResult> {
        let mut state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        if let Some(existing_data) = state.chunks.get(&chunk_pos) {
            if self.config.enable_chunk_checksums && existing_data.checksum != checksum {
                let error = ValidationError::ChunkChecksumMismatch {
                    chunk_pos,
                    expected: existing_data.checksum,
                    actual: checksum,
                };
                state.validation_errors.push(error.clone());
                return Ok(ValidationResult::Invalid(error));
            }
        }

        // Update chunk data
        let chunk_data = ChunkValidationData {
            chunk_pos,
            checksum,
            last_modified: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            block_count: 0, // TODO: Calculate actual block count
            save_state,
            network_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            validation_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        state.chunks.insert(chunk_pos, chunk_data);
        Ok(ValidationResult::Valid)
    }

    fn validate_player_internal(
        &self,
        player_id: u32,
        position: Vec3,
        timestamp: u64,
    ) -> PersistenceResult<ValidationResult> {
        let mut state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        // Check position bounds
        if self.is_position_out_of_bounds(position) {
            let error = ValidationError::PlayerPositionOutOfBounds {
                player_id,
                position,
            };
            state.validation_errors.push(error.clone());
            return Ok(ValidationResult::Invalid(error));
        }

        // Check timestamp validity
        if self.config.enable_timestamp_validation {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let time_diff = if current_time > timestamp {
                Duration::from_secs(current_time - timestamp)
            } else {
                Duration::from_secs(timestamp - current_time)
            };

            if time_diff > self.config.max_timestamp_drift {
                let error = ValidationError::TimestampInconsistency {
                    operation: format!("player_save_{}", player_id),
                    timestamp,
                    current_time,
                };
                state.validation_errors.push(error.clone());
                return Ok(ValidationResult::Invalid(error));
            }
        }

        // Update player data
        if let Some(player_data) = state.players.get_mut(&player_id) {
            player_data.last_position = position;
            player_data.last_update = timestamp;
            player_data.movement_history.push((position, timestamp));

            // Keep movement history bounded
            if player_data.movement_history.len() > 100 {
                player_data.movement_history.remove(0);
            }
        }

        Ok(ValidationResult::Valid)
    }

    fn validate_player_movement(
        &self,
        player_id: u32,
        player_data: &PlayerValidationData,
        warnings: &mut Vec<ValidationWarning>,
    ) {
        if player_data.movement_history.len() < 2 {
            return;
        }

        let history = &player_data.movement_history;
        for i in 1..history.len() {
            let (prev_pos, prev_time) = history[i - 1];
            let (curr_pos, curr_time) = history[i];

            if curr_time > prev_time {
                let distance = (curr_pos - prev_pos).length();
                let time_delta = Duration::from_secs(curr_time - prev_time);
                let speed = distance / time_delta.as_secs_f32();

                if speed > self.config.max_player_speed {
                    warnings.push(ValidationWarning::UnusualPlayerMovement {
                        player_id,
                        distance,
                        time_delta_seconds: time_delta.as_secs(),
                    });
                }
            }
        }
    }

    fn validate_player_position(
        &self,
        player_id: u32,
        position: Vec3,
        errors: &mut Vec<ValidationError>,
    ) {
        if self.is_position_out_of_bounds(position) {
            errors.push(ValidationError::PlayerPositionOutOfBounds {
                player_id,
                position,
            });
        }
    }

    fn is_position_out_of_bounds(&self, position: Vec3) -> bool {
        // Define reasonable world bounds (can be configured)
        const MIN_COORD: f32 = -30_000_000.0;
        const MAX_COORD: f32 = 30_000_000.0;
        const MIN_Y: f32 = -64.0;
        const MAX_Y: f32 = 320.0;

        position.x < MIN_COORD
            || position.x > MAX_COORD
            || position.z < MIN_COORD
            || position.z > MAX_COORD
            || position.y < MIN_Y
            || position.y > MAX_Y
    }

    fn get_next_operation_id(&self) -> PersistenceResult<u32> {
        let mut id = self.next_operation_id.lock().map_err(|_| {
            PersistenceError::LockPoisoned("operation_id lock poisoned".to_string())
        })?;
        *id += 1;
        Ok(*id)
    }

    fn record_operation(&self, operation: ValidationOperation) -> PersistenceResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            PersistenceError::LockPoisoned("validator state lock poisoned".to_string())
        })?;

        state.operation_history.push(operation);

        // Keep history bounded
        if state.operation_history.len() > 1000 {
            state.operation_history.remove(0);
        }

        Ok(())
    }
}

/// Validation statistics
#[derive(Debug, Clone)]
pub struct ValidationStats {
    pub total_chunks: usize,
    pub total_players: usize,
    pub total_operations: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub last_full_validation: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let config = ValidationConfig::default();
        let validator = NetworkValidator::new(config);

        let stats = validator
            .get_validation_stats()
            .expect("Failed to get validation stats");
        assert_eq!(stats.total_chunks, 0);
        assert_eq!(stats.total_players, 0);
        assert_eq!(stats.total_errors, 0);
    }

    #[test]
    fn test_chunk_registration() {
        let config = ValidationConfig::default();
        let validator = NetworkValidator::new(config);

        let chunk_pos = ChunkPos { x: 0, y: 0, z: 0 };
        validator
            .register_chunk(chunk_pos, 12345, ChunkSaveStatus::Saved)
            .expect("Failed to register chunk");

        let stats = validator
            .get_validation_stats()
            .expect("Failed to get validation stats");
        assert_eq!(stats.total_chunks, 1);
    }

    #[test]
    fn test_player_registration() {
        let config = ValidationConfig::default();
        let validator = NetworkValidator::new(config);

        validator
            .register_player(1, "test-uuid".to_string(), Vec3::new(0.0, 100.0, 0.0))
            .expect("Failed to register player");

        let stats = validator
            .get_validation_stats()
            .expect("Failed to get validation stats");
        assert_eq!(stats.total_players, 1);
    }

    #[test]
    fn test_chunk_validation() {
        let config = ValidationConfig::default();
        let validator = NetworkValidator::new(config);

        let chunk_pos = ChunkPos { x: 0, y: 0, z: 0 };

        // First save should be valid
        let result = validator
            .validate_chunk_save(chunk_pos, 12345, ChunkSaveStatus::Saved)
            .expect("Failed to validate chunk save");
        assert_eq!(result, ValidationResult::Valid);

        // Same checksum should be valid
        let result = validator
            .validate_chunk_save(chunk_pos, 12345, ChunkSaveStatus::Saved)
            .expect("Failed to validate chunk save");
        assert_eq!(result, ValidationResult::Valid);

        // Different checksum should be invalid
        let result = validator
            .validate_chunk_save(chunk_pos, 54321, ChunkSaveStatus::Saved)
            .expect("Failed to validate chunk save");
        if let ValidationResult::Invalid(ValidationError::ChunkChecksumMismatch { .. }) = result {
            // Expected
        } else {
            panic!("Expected checksum mismatch error, got: {:?}", result);
        }
    }

    #[test]
    fn test_player_position_validation() {
        let config = ValidationConfig::default();
        let validator = NetworkValidator::new(config);

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Valid position
        let result = validator
            .validate_player_save(1, Vec3::new(100.0, 64.0, 200.0), current_time)
            .expect("Failed to validate player save");
        assert_eq!(result, ValidationResult::Valid);

        // Out of bounds position
        let result = validator
            .validate_player_save(1, Vec3::new(50_000_000.0, 64.0, 200.0), current_time)
            .expect("Failed to validate player save");
        if let ValidationResult::Invalid(ValidationError::PlayerPositionOutOfBounds { .. }) = result
        {
            // Expected
        } else {
            panic!("Expected position out of bounds error, got: {:?}", result);
        }
    }

    #[test]
    fn test_full_world_validation() {
        let config = ValidationConfig::default();
        let validator = NetworkValidator::new(config);

        // Add some test data
        validator
            .register_chunk(ChunkPos { x: 0, y: 0, z: 0 }, 12345, ChunkSaveStatus::Saved)
            .expect("Failed to register chunk");
        validator
            .register_player(1, "test-uuid".to_string(), Vec3::new(0.0, 100.0, 0.0))
            .expect("Failed to register player");

        let result = validator
            .validate_full_world()
            .expect("Failed to validate full world");
        assert_eq!(result, ValidationResult::Valid);
    }
}
