//! World interface implementations

use super::{capabilities, QueryType, UnifiedInterface};
use crate::world::{
    core::{BlockId, ChunkPos, Ray, RaycastHit, VoxelPos},
    management::UnifiedWorldManager,
};
use parking_lot;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Universal world interface that works across GPU and CPU backends
pub trait WorldInterface: UnifiedInterface {
    /// Get a block at the specified position
    fn get_block(&self, pos: VoxelPos) -> BlockId;

    /// Set a block at the specified position
    fn set_block(&mut self, pos: VoxelPos, block_id: BlockId) -> Result<(), WorldError>;

    /// Get surface height at world coordinates
    fn get_surface_height(&self, x: f64, z: f64) -> i32;

    /// Check if a chunk is loaded
    fn is_chunk_loaded(&self, chunk_pos: ChunkPos) -> bool;

    /// Load a chunk
    fn load_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), WorldError>;

    /// Unload a chunk
    fn unload_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), WorldError>;

    /// Perform a raycast
    fn raycast(&self, ray: Ray, max_distance: f32) -> Option<RaycastHit>;

    /// Perform a general query
    fn query(&self, query: WorldQuery) -> Result<QueryResult, WorldError>;

    /// Get loaded chunks in radius
    fn get_chunks_in_radius(&self, center: ChunkPos, radius: u32) -> Vec<ChunkPos>;

    /// Batch operations for efficiency
    fn batch_operation(
        &mut self,
        operations: Vec<WorldOperation>,
    ) -> Result<Vec<OperationResult>, WorldError>;

    /// Get light emission level for a block type
    /// This is a helper method that queries the block registry
    fn get_block_light_emission(&self, block_id: BlockId) -> u8 {
        // Default implementation - blocks don't emit light unless overridden
        match block_id {
            BlockId(6) => 15,  // Glowstone emits maximum light
            BlockId(19) => 14, // Torch emits bright light
            BlockId(21) => 15, // Lava emits maximum light
            _ => 0,
        }
    }

    /// Check if a block is transparent
    fn is_block_transparent(&self, block_id: BlockId) -> bool {
        match block_id {
            BlockId::AIR => true,
            BlockId::WATER => true,
            BlockId::GLASS => true,
            _ => false,
        }
    }

    /// Update skylight for a vertical column
    /// This is typically called after block changes
    fn update_skylight_column(&mut self, x: i32, y: i32, z: i32) {
        // Default implementation does nothing
        // GPU implementations would update skylight propagation
        let _ = (x, y, z);
    }

    /// Ensure the chunk containing the camera position is loaded
    /// Returns true if the chunk is loaded, false if still being generated
    fn ensure_camera_chunk_loaded(&mut self, camera_pos: cgmath::Point3<f32>) -> bool {
        let chunk_size = self.chunk_size() as i32;
        let chunk_pos = ChunkPos {
            x: (camera_pos.x as i32) / chunk_size,
            y: (camera_pos.y as i32) / chunk_size,
            z: (camera_pos.z as i32) / chunk_size,
        };
        self.is_chunk_loaded(chunk_pos)
    }

    /// Update loaded chunks based on camera position
    fn update_loaded_chunks(&mut self, camera_pos: cgmath::Point3<f32>) {
        // Default implementation - derived implementations should override
        let _ = camera_pos;
    }

    /// Check if a block position is in bounds (for infinite worlds, always true)
    fn is_block_in_bounds(&self, _pos: VoxelPos) -> bool {
        true // Default implementation for infinite worlds
    }

    /// Get the world buffer for GPU operations
    fn get_world_buffer(
        &self,
    ) -> Option<Arc<std::sync::Mutex<crate::world::storage::WorldBuffer>>> {
        // Default implementation returns None
        // GPU implementations should override this
        None
    }

    /// Get sky light level at position  
    fn get_sky_light(&self, pos: VoxelPos) -> u8 {
        // Default sky light implementation
        15 // Full sunlight by default
    }

    /// Set sky light level at position
    fn set_sky_light(&mut self, pos: VoxelPos, level: u8) {
        // Default implementation does nothing
        let _ = (pos, level);
    }

    /// Get block light level at position
    fn get_block_light(&self, pos: VoxelPos) -> u8 {
        // Get light emission from the block itself
        let block_id = self.get_block(pos);
        self.get_block_light_emission(block_id)
    }

    /// Set block light level at position
    fn set_block_light(&mut self, pos: VoxelPos, level: u8) {
        // Default implementation does nothing
        let _ = (pos, level);
    }

    /// Take dirty chunks that need remeshing
    fn take_dirty_chunks(&mut self) -> std::collections::HashSet<ChunkPos> {
        // Default implementation returns empty set
        std::collections::HashSet::new()
    }

    /// Get chunk size from configuration
    fn chunk_size(&self) -> u32 {
        50 // Default chunk size (50 for danger-money)
    }

    /// Get an iterator over loaded chunks
    fn iter_loaded_chunks(&self) -> Box<dyn Iterator<Item = (ChunkPos, bool)> + '_> {
        // Default implementation returns empty iterator
        Box::new(std::iter::empty())
    }

    /// Get configuration object
    fn config(&self) -> WorldConfig {
        WorldConfig::default()
    }

    /// Get chunk manager reference for direct access
    fn chunk_manager(&self) -> &dyn ChunkManager {
        // Default implementation - should be overridden by concrete types
        static DEFAULT_MANAGER: DefaultChunkManager = DefaultChunkManager;
        &DEFAULT_MANAGER
    }

    /// Get a chunk for meshing operations
    fn get_chunk_for_meshing(
        &self,
        chunk_pos: ChunkPos,
    ) -> Option<Arc<parking_lot::RwLock<dyn ChunkData>>> {
        // Default implementation returns None
        let _ = chunk_pos;
        None
    }
}

/// Read-only world interface for queries that don't modify state
pub trait ReadOnlyWorldInterface: UnifiedInterface {
    /// Get a block at the specified position
    fn get_block(&self, pos: VoxelPos) -> BlockId;

    /// Get surface height at world coordinates
    fn get_surface_height(&self, x: f64, z: f64) -> i32;

    /// Check if a chunk is loaded
    fn is_chunk_loaded(&self, chunk_pos: ChunkPos) -> bool;

    /// Perform a raycast
    fn raycast(&self, ray: Ray, max_distance: f32) -> Option<RaycastHit>;

    /// Perform a general query
    fn query(&self, query: WorldQuery) -> Result<QueryResult, WorldError>;
}

/// Unified world interface implementation
pub struct UnifiedWorldInterface {
    manager: Arc<Mutex<UnifiedWorldManager>>,
}

impl UnifiedWorldInterface {
    /// Create a new unified world interface
    pub fn new(manager: Arc<Mutex<UnifiedWorldManager>>) -> Self {
        Self { manager }
    }
}

impl UnifiedInterface for UnifiedWorldInterface {
    fn backend_type(&self) -> &str {
        // Lock briefly to check backend type
        if let Ok(manager) = self.manager.try_lock() {
            if manager.is_gpu() {
                "GPU"
            } else {
                "CPU"
            }
        } else {
            "Unknown"
        }
    }

    fn supports_capability(&self, capability: &str) -> bool {
        if let Ok(manager) = self.manager.try_lock() {
            match capability {
                capabilities::GPU_ACCELERATION => manager.is_gpu(),
                capabilities::REAL_TIME_GENERATION => true,
                capabilities::BATCH_OPERATIONS => true,
                capabilities::INFINITE_WORLDS => true,
                capabilities::MULTI_THREADING => true,
                capabilities::LIGHTING_CALCULATION => manager.is_gpu(),
                capabilities::PHYSICS_SIMULATION => manager.is_gpu(),
                capabilities::WEATHER_EFFECTS => manager.is_gpu(),
                capabilities::MEMORY_STREAMING => true,
                capabilities::LOD_SUPPORT => manager.is_gpu(),
                _ => false,
            }
        } else {
            false
        }
    }

    fn performance_metrics(&self) -> Option<HashMap<String, f64>> {
        // TODO: Implement performance metrics collection
        None
    }
}

impl WorldInterface for UnifiedWorldInterface {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        if let Ok(manager) = self.manager.lock() {
            manager.get_block(pos)
        } else {
            BlockId::AIR
        }
    }

    fn set_block(&mut self, pos: VoxelPos, block_id: BlockId) -> Result<(), WorldError> {
        if let Ok(mut manager) = self.manager.lock() {
            manager
                .set_block(pos, block_id)
                .map_err(|e| WorldError::OperationFailed {
                    message: e.to_string(),
                })
        } else {
            Err(WorldError::LockFailed)
        }
    }

    fn get_surface_height(&self, x: f64, z: f64) -> i32 {
        // TODO: Implement surface height query
        64
    }

    fn is_chunk_loaded(&self, chunk_pos: ChunkPos) -> bool {
        // TODO: Implement chunk loaded check
        false
    }

    fn load_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), WorldError> {
        if let Ok(mut manager) = self.manager.lock() {
            manager
                .load_chunk(chunk_pos)
                .map_err(|e| WorldError::OperationFailed {
                    message: e.to_string(),
                })
        } else {
            Err(WorldError::LockFailed)
        }
    }

    fn unload_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), WorldError> {
        // TODO: Implement chunk unloading
        Ok(())
    }

    fn raycast(&self, ray: Ray, max_distance: f32) -> Option<RaycastHit> {
        if let Ok(manager) = self.manager.lock() {
            // Manual raycast implementation
            let step = 0.1; // Step size
            let mut t = 0.0;

            while t <= max_distance {
                let point = cgmath::Point3::new(
                    ray.origin.x + ray.direction.x * t,
                    ray.origin.y + ray.direction.y * t,
                    ray.origin.z + ray.direction.z * t,
                );

                let voxel_pos = VoxelPos::new(
                    point.x.floor() as i32,
                    point.y.floor() as i32,
                    point.z.floor() as i32,
                );

                let block = manager.get_block(voxel_pos);
                if block != BlockId::AIR {
                    // Found a solid block
                    return Some(RaycastHit {
                        position: voxel_pos,
                        face: crate::world::core::BlockFace::Top, // TODO: Calculate proper face
                        distance: t,
                        block,
                    });
                }

                t += step;
            }
            None
        } else {
            None
        }
    }

    fn query(&self, query: WorldQuery) -> Result<QueryResult, WorldError> {
        match query.query_type {
            QueryType::GetBlock { pos } => {
                let block = self.get_block(pos);
                Ok(QueryResult::Block(block))
            }
            QueryType::GetSurfaceHeight { x, z } => {
                let height = self.get_surface_height(x, z);
                Ok(QueryResult::Height(height))
            }
            QueryType::IsChunkLoaded { pos } => {
                let loaded = self.is_chunk_loaded(pos);
                Ok(QueryResult::Boolean(loaded))
            }
            QueryType::GetChunksInRadius { center, radius } => {
                let chunks = self.get_chunks_in_radius(center, radius);
                Ok(QueryResult::ChunkList(chunks))
            }
            QueryType::Raycast {
                origin,
                direction,
                max_distance,
            } => {
                let ray = Ray::new(
                    cgmath::Point3::new(origin[0], origin[1], origin[2]),
                    cgmath::Vector3::new(direction[0], direction[1], direction[2]),
                );
                let hit = self.raycast(ray, max_distance);
                Ok(QueryResult::RaycastHit(hit))
            }
        }
    }

    fn get_chunks_in_radius(&self, center: ChunkPos, radius: u32) -> Vec<ChunkPos> {
        let mut chunks = Vec::new();
        let radius = radius as i32;

        for x in (center.x - radius)..=(center.x + radius) {
            for y in (center.y - radius)..=(center.y + radius) {
                for z in (center.z - radius)..=(center.z + radius) {
                    let chunk_pos = ChunkPos { x, y, z };
                    let distance_sq = (chunk_pos.x - center.x).pow(2)
                        + (chunk_pos.y - center.y).pow(2)
                        + (chunk_pos.z - center.z).pow(2);

                    if distance_sq <= radius.pow(2) {
                        chunks.push(chunk_pos);
                    }
                }
            }
        }

        chunks
    }

    fn batch_operation(
        &mut self,
        operations: Vec<WorldOperation>,
    ) -> Result<Vec<OperationResult>, WorldError> {
        let mut results = Vec::with_capacity(operations.len());

        for operation in operations {
            let result = match operation {
                WorldOperation::SetBlock { pos, block_id } => match self.set_block(pos, block_id) {
                    Ok(()) => OperationResult::Success,
                    Err(e) => OperationResult::Error(e.to_string()),
                },
                WorldOperation::LoadChunk { pos } => match self.load_chunk(pos) {
                    Ok(()) => OperationResult::Success,
                    Err(e) => OperationResult::Error(e.to_string()),
                },
                WorldOperation::UnloadChunk { pos } => match self.unload_chunk(pos) {
                    Ok(()) => OperationResult::Success,
                    Err(e) => OperationResult::Error(e.to_string()),
                },
            };
            results.push(result);
        }

        Ok(results)
    }
}

/// World query structure
#[derive(Debug, Clone)]
pub struct WorldQuery {
    pub query_type: QueryType,
    pub timeout_ms: Option<u32>,
}

impl WorldQuery {
    pub fn new(query_type: QueryType) -> Self {
        Self {
            query_type,
            timeout_ms: None,
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// Query result types
#[derive(Debug, Clone)]
pub enum QueryResult {
    Block(BlockId),
    Height(i32),
    Boolean(bool),
    ChunkList(Vec<ChunkPos>),
    RaycastHit(Option<RaycastHit>),
}

/// World operations for batch processing
#[derive(Debug, Clone)]
pub enum WorldOperation {
    SetBlock { pos: VoxelPos, block_id: BlockId },
    LoadChunk { pos: ChunkPos },
    UnloadChunk { pos: ChunkPos },
}

/// Operation results
#[derive(Debug, Clone)]
pub enum OperationResult {
    Success,
    Error(String),
}

/// World interface errors
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("Operation failed: {message}")]
    OperationFailed { message: String },

    #[error("Lock acquisition failed")]
    LockFailed,

    #[error("Invalid position: {x}, {y}, {z}")]
    InvalidPosition { x: i32, y: i32, z: i32 },

    #[error("Backend not available: {backend}")]
    BackendNotAvailable { backend: String },

    #[error("Query timeout after {timeout_ms}ms")]
    QueryTimeout { timeout_ms: u32 },
}

/// World configuration
#[derive(Debug, Clone)]
pub struct WorldConfig {
    pub chunk_size: u32,
    pub render_distance: u32,
    pub max_loaded_chunks: usize,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            chunk_size: 32,
            render_distance: 8,
            max_loaded_chunks: 1000,
        }
    }
}

/// Chunk manager trait for managing chunk loading/unloading
pub trait ChunkManager: Send + Sync {
    fn loaded_chunk_count(&self) -> usize;
    fn is_chunk_loaded(&self, pos: ChunkPos) -> bool;
    fn get_loaded_chunks(&self) -> Vec<ChunkPos>;
}

/// Default chunk manager implementation
pub struct DefaultChunkManager;

impl ChunkManager for DefaultChunkManager {
    fn loaded_chunk_count(&self) -> usize {
        0
    }
    fn is_chunk_loaded(&self, _pos: ChunkPos) -> bool {
        false
    }
    fn get_loaded_chunks(&self) -> Vec<ChunkPos> {
        Vec::new()
    }
}

/// Chunk data trait for accessing chunk contents
pub trait ChunkData: Send + Sync + std::any::Any {
    fn position(&self) -> ChunkPos;
    fn get_block_at(&self, x: u32, y: u32, z: u32) -> BlockId;
    fn set_block_at(&mut self, x: u32, y: u32, z: u32, block: BlockId);
    fn is_dirty(&self) -> bool;
    fn mark_clean(&mut self);

    /// Helper method to downcast to concrete type
    fn as_any(&self) -> &dyn std::any::Any;
}
