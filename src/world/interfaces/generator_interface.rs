//! Generator interface implementations

use super::{capabilities, UnifiedInterface};
use crate::world::{
    core::{ChunkPos, VoxelPos},
    generation::{TerrainParams, UnifiedGenerator, WorldGenerator},
    storage::TempChunk,
};
use crate::constants::core::CHUNK_SIZE;
use crate::constants::monitoring::PROFILING_SESSION_TIMEOUT_MS;
use std::collections::HashMap;
use std::sync::Arc;

/// Universal generator interface
pub trait GeneratorInterface: UnifiedInterface {
    /// Generate a chunk
    fn generate_chunk(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationResult, GeneratorError>;

    /// Generate multiple chunks in batch
    fn generate_batch(
        &self,
        requests: Vec<GenerationRequest>,
    ) -> Result<Vec<GenerationResult>, GeneratorError>;

    /// Get surface height at coordinates
    fn get_surface_height(&self, x: f64, z: f64) -> i32;

    /// Find safe spawn location
    fn find_spawn_location(&self, hint: VoxelPos) -> VoxelPos;

    /// Check if generator can handle request
    fn can_generate(&self, request: &GenerationRequest) -> bool;

    /// Get generation capabilities
    fn capabilities(&self) -> Vec<String>;
}

/// Unified generator interface implementation
pub struct UnifiedGeneratorInterface {
    generator: Arc<UnifiedGenerator>,
}

impl UnifiedGeneratorInterface {
    /// Create a new unified generator interface
    pub fn new(generator: Arc<UnifiedGenerator>) -> Self {
        Self { generator }
    }
}

impl UnifiedInterface for UnifiedGeneratorInterface {
    fn backend_type(&self) -> &str {
        if self.generator.is_gpu() {
            "GPU"
        } else {
            "CPU"
        }
    }

    fn supports_capability(&self, capability: &str) -> bool {
        match capability {
            capabilities::GPU_ACCELERATION => self.generator.is_gpu(),
            capabilities::REAL_TIME_GENERATION => true,
            capabilities::BATCH_OPERATIONS => self.generator.is_gpu(),
            capabilities::INFINITE_WORLDS => true,
            capabilities::MULTI_THREADING => true,
            capabilities::LOD_SUPPORT => self.generator.is_gpu(),
            _ => false,
        }
    }

    fn performance_metrics(&self) -> Option<HashMap<String, f64>> {
        // TODO: Implement performance metrics for generator
        Some(HashMap::from([
            ("chunks_generated".to_string(), 0.0),
            ("avg_generation_time_ms".to_string(), 0.0),
        ]))
    }
}

impl GeneratorInterface for UnifiedGeneratorInterface {
    fn generate_chunk(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationResult, GeneratorError> {
        let chunk = self
            .generator
            .generate_chunk(request.chunk_pos, request.chunk_size);

        Ok(GenerationResult {
            chunk_pos: request.chunk_pos,
            chunk: Some(chunk),
            generation_time_ms: 0.0, // TODO: Measure actual time
            metadata: HashMap::new(),
        })
    }

    fn generate_batch(
        &self,
        requests: Vec<GenerationRequest>,
    ) -> Result<Vec<GenerationResult>, GeneratorError> {
        let mut results = Vec::with_capacity(requests.len());

        for request in requests {
            let result = self.generate_chunk(request)?;
            results.push(result);
        }

        Ok(results)
    }

    fn get_surface_height(&self, x: f64, z: f64) -> i32 {
        self.generator.get_surface_height(x, z)
    }

    fn find_spawn_location(&self, hint: VoxelPos) -> VoxelPos {
        let spawn_height = self
            .generator
            .find_safe_spawn_height(hint.x as f64, hint.z as f64);
        VoxelPos {
            x: hint.x,
            y: spawn_height as i32,
            z: hint.z,
        }
    }

    fn can_generate(&self, request: &GenerationRequest) -> bool {
        // Basic validation
        request.chunk_size > 0 && request.chunk_size <= 256
    }

    fn capabilities(&self) -> Vec<String> {
        let mut caps = vec![
            capabilities::REAL_TIME_GENERATION.to_string(),
            capabilities::INFINITE_WORLDS.to_string(),
            capabilities::MULTI_THREADING.to_string(),
        ];

        if self.generator.is_gpu() {
            caps.extend([
                capabilities::GPU_ACCELERATION.to_string(),
                capabilities::BATCH_OPERATIONS.to_string(),
                capabilities::LOD_SUPPORT.to_string(),
            ]);
        }

        caps
    }
}

/// Generation request structure
#[derive(Debug, Clone)]
pub struct GenerationRequest {
    pub chunk_pos: ChunkPos,
    pub chunk_size: u32,
    pub terrain_params: Option<TerrainParams>,
    pub priority: GenerationPriority,
    pub timeout_ms: Option<u32>,
}

impl GenerationRequest {
    /// Create a basic generation request
    pub fn new(chunk_pos: ChunkPos, chunk_size: u32) -> Self {
        Self {
            chunk_pos,
            chunk_size,
            terrain_params: None,
            priority: GenerationPriority::Normal,
            timeout_ms: None,
        }
    }

    /// Set terrain parameters
    pub fn with_terrain_params(mut self, params: TerrainParams) -> Self {
        self.terrain_params = Some(params);
        self
    }

    /// Set generation priority
    pub fn with_priority(mut self, priority: GenerationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// Generation result structure
#[derive(Debug)]
pub struct GenerationResult {
    pub chunk_pos: ChunkPos,
    pub chunk: Option<TempChunk>,
    pub generation_time_ms: f64,
    pub metadata: HashMap<String, f64>,
}

/// Generation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GenerationPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for GenerationPriority {
    fn default() -> Self {
        GenerationPriority::Normal
    }
}

/// Generator errors
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("Generation failed for chunk {x}, {y}, {z}: {message}")]
    GenerationFailed {
        x: i32,
        y: i32,
        z: i32,
        message: String,
    },

    #[error("Invalid request: {field}")]
    InvalidRequest { field: String },

    #[error("Backend not available: {backend}")]
    BackendNotAvailable { backend: String },

    #[error("Generation timeout after {timeout_ms}ms")]
    GenerationTimeout { timeout_ms: u32 },

    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_request_creation() {
        let chunk_pos = ChunkPos { x: 0, y: 0, z: 0 };
        let request = GenerationRequest::new(chunk_pos, CHUNK_SIZE);

        assert_eq!(request.chunk_pos, chunk_pos);
        assert_eq!(request.chunk_size, CHUNK_SIZE);
        assert_eq!(request.priority, GenerationPriority::Normal);
    }

    #[test]
    fn test_generation_priority_ordering() {
        assert!(GenerationPriority::Critical > GenerationPriority::High);
        assert!(GenerationPriority::High > GenerationPriority::Normal);
        assert!(GenerationPriority::Normal > GenerationPriority::Low);
    }

    #[test]
    fn test_request_builder_pattern() {
        let chunk_pos = ChunkPos { x: 1, y: 2, z: 3 };
        let params = TerrainParams::default();

        let request = GenerationRequest::new(chunk_pos, CHUNK_SIZE)
            .with_terrain_params(params)
            .with_priority(GenerationPriority::High)
            .with_timeout(PROFILING_SESSION_TIMEOUT_MS as u32);

        assert!(request.terrain_params.is_some());
        assert_eq!(request.priority, GenerationPriority::High);
        assert_eq!(request.timeout_ms, Some(PROFILING_SESSION_TIMEOUT_MS as u32));
    }
}
