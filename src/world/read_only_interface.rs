/// Read-Only World Interface
/// 
/// Provides safe, concurrent read access to world data for renderer, physics,
/// and other systems that only need to query world state without modifications.
/// 
/// This reduces contention on the main WorldInterface by allowing multiple
/// systems to read world data simultaneously without blocking each other
/// or the main world update thread.

use crate::world::{BlockId, VoxelPos, ChunkPos, Block};
use cgmath::Point3;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Read-only snapshot of world state
pub trait ReadOnlyWorldInterface: Send + Sync {
    /// Get a block at the given position (immutable)
    fn get_block(&self, pos: VoxelPos) -> BlockId;
    
    /// Get chunk size
    fn chunk_size(&self) -> u32;
    
    /// Check if a block position is in bounds
    fn is_block_in_bounds(&self, pos: VoxelPos) -> bool;
    
    /// Get sky light level at position (immutable)
    fn get_sky_light(&self, pos: VoxelPos) -> u8;
    
    /// Get block light level at position (immutable)
    fn get_block_light(&self, pos: VoxelPos) -> u8;
    
    /// Check if a chunk is loaded (immutable)
    fn is_chunk_loaded(&self, pos: ChunkPos) -> bool;
    
    /// Get the surface height at the given world coordinates (immutable)
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32;
    
    /// Check if a block is transparent (immutable)
    fn is_block_transparent(&self, pos: VoxelPos) -> bool;
    
    /// Get loaded chunks list (immutable)
    fn get_loaded_chunks(&self) -> Vec<ChunkPos>;
    
    /// Get blocks in a region (batch query for efficiency)
    fn get_blocks_in_region(&self, min: VoxelPos, max: VoxelPos) -> Vec<(VoxelPos, BlockId)>;
    
    /// Get timestamp when this snapshot was created
    fn snapshot_timestamp(&self) -> Instant;
    
    /// Get version number for change detection
    fn version(&self) -> u64;
}

/// Immutable world snapshot that can be safely shared across threads
#[derive(Clone)]
pub struct WorldSnapshot {
    /// Snapshot of chunk data at time of creation
    chunk_data: Arc<HashMap<ChunkPos, ChunkSnapshot>>,
    
    /// Loaded chunks at time of snapshot
    loaded_chunks: Arc<HashSet<ChunkPos>>,
    
    /// World configuration
    chunk_size: u32,
    
    /// When this snapshot was created
    timestamp: Instant,
    
    /// Version number for change detection
    version: u64,
    
    /// Cached surface heights for performance
    surface_height_cache: Arc<HashMap<(i32, i32), i32>>,
}

/// Snapshot of a single chunk's data
#[derive(Debug, Clone)]
pub struct ChunkSnapshot {
    /// Block data (flattened 3D array)
    blocks: Vec<BlockId>,
    
    /// Sky light data
    sky_light: Vec<u8>,
    
    /// Block light data  
    block_light: Vec<u8>,
    
    /// Chunk position
    position: ChunkPos,
    
    /// Chunk size
    size: u32,
    
    /// Is this chunk fully generated?
    is_generated: bool,
    
    /// Cached block transparency for lighting calculations
    transparency_cache: Vec<bool>,
}

/// Builder for creating world snapshots
pub struct WorldSnapshotBuilder {
    chunk_data: HashMap<ChunkPos, ChunkSnapshot>,
    loaded_chunks: HashSet<ChunkPos>,
    chunk_size: u32,
    version: u64,
    surface_height_cache: HashMap<(i32, i32), i32>,
}

impl WorldSnapshot {
    /// Create a new builder
    pub fn builder(chunk_size: u32, version: u64) -> WorldSnapshotBuilder {
        WorldSnapshotBuilder {
            chunk_data: HashMap::new(),
            loaded_chunks: HashSet::new(),
            chunk_size,
            version,
            surface_height_cache: HashMap::new(),
        }
    }
    
    /// Get chunk containing the given voxel position
    fn get_chunk_for_voxel(&self, pos: VoxelPos) -> Option<&ChunkSnapshot> {
        let chunk_pos = self.voxel_to_chunk_pos(pos);
        self.chunk_data.get(&chunk_pos)
    }
    
    /// Convert voxel position to chunk position
    fn voxel_to_chunk_pos(&self, pos: VoxelPos) -> ChunkPos {
        ChunkPos {
            x: pos.x.div_euclid(self.chunk_size as i32),
            y: pos.y.div_euclid(self.chunk_size as i32),
            z: pos.z.div_euclid(self.chunk_size as i32),
        }
    }
    
    /// Convert voxel position to local chunk coordinates
    fn voxel_to_local_pos(&self, pos: VoxelPos) -> (u32, u32, u32) {
        let local_x = pos.x.rem_euclid(self.chunk_size as i32) as u32;
        let local_z = pos.z.rem_euclid(self.chunk_size as i32) as u32;
        let local_y = pos.y as u32;
        (local_x, local_y, local_z)
    }
    
    /// Check if this snapshot is newer than the given version
    pub fn is_newer_than(&self, other_version: u64) -> bool {
        self.version > other_version
    }
    
    /// Get statistics about this snapshot
    pub fn get_stats(&self) -> SnapshotStats {
        let total_blocks: usize = self.chunk_data.values()
            .map(|chunk| chunk.blocks.len())
            .sum();
            
        let generated_chunks = self.chunk_data.values()
            .filter(|chunk| chunk.is_generated)
            .count();
        
        SnapshotStats {
            chunk_count: self.chunk_data.len(),
            generated_chunk_count: generated_chunks,
            total_block_count: total_blocks,
            snapshot_size_bytes: self.estimate_memory_usage(),
            creation_timestamp: self.timestamp,
            version: self.version,
        }
    }
    
    /// Estimate memory usage of this snapshot
    fn estimate_memory_usage(&self) -> usize {
        let chunk_overhead = std::mem::size_of::<ChunkSnapshot>() * self.chunk_data.len();
        let block_data: usize = self.chunk_data.values()
            .map(|chunk| chunk.blocks.len() * std::mem::size_of::<BlockId>())
            .sum();
        let light_data: usize = self.chunk_data.values()
            .map(|chunk| (chunk.sky_light.len() + chunk.block_light.len()) * std::mem::size_of::<u8>())
            .sum();
        let transparency_data: usize = self.chunk_data.values()
            .map(|chunk| chunk.transparency_cache.len() * std::mem::size_of::<bool>())
            .sum();
        
        chunk_overhead + block_data + light_data + transparency_data
    }
}

impl ReadOnlyWorldInterface for WorldSnapshot {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        if let Some(chunk) = self.get_chunk_for_voxel(pos) {
            chunk.get_block_local(self.voxel_to_local_pos(pos))
        } else {
            BlockId::AIR // Return air for unloaded chunks
        }
    }
    
    fn chunk_size(&self) -> u32 {
        self.chunk_size
    }
    
    fn is_block_in_bounds(&self, pos: VoxelPos) -> bool {
        // For infinite worlds, check if chunk is loaded
        self.is_chunk_loaded(self.voxel_to_chunk_pos(pos))
    }
    
    fn get_sky_light(&self, pos: VoxelPos) -> u8 {
        if let Some(chunk) = self.get_chunk_for_voxel(pos) {
            chunk.get_sky_light_local(self.voxel_to_local_pos(pos))
        } else {
            15 // Full skylight for unloaded chunks
        }
    }
    
    fn get_block_light(&self, pos: VoxelPos) -> u8 {
        if let Some(chunk) = self.get_chunk_for_voxel(pos) {
            chunk.get_block_light_local(self.voxel_to_local_pos(pos))
        } else {
            0 // No block light for unloaded chunks
        }
    }
    
    fn is_chunk_loaded(&self, pos: ChunkPos) -> bool {
        self.loaded_chunks.contains(&pos)
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        let grid_x = world_x.floor() as i32;
        let grid_z = world_z.floor() as i32;
        
        // Check cache first
        if let Some(&height) = self.surface_height_cache.get(&(grid_x, grid_z)) {
            return height;
        }
        
        // Calculate surface height by scanning from top down
        for y in (0..256).rev() {
            let pos = VoxelPos { x: grid_x, y, z: grid_z };
            let block = self.get_block(pos);
            if block != BlockId::AIR {
                return y;
            }
        }
        
        0 // Default to sea level if no blocks found
    }
    
    fn is_block_transparent(&self, pos: VoxelPos) -> bool {
        if let Some(chunk) = self.get_chunk_for_voxel(pos) {
            chunk.is_block_transparent_local(self.voxel_to_local_pos(pos))
        } else {
            true // Unloaded chunks are considered transparent
        }
    }
    
    fn get_loaded_chunks(&self) -> Vec<ChunkPos> {
        self.loaded_chunks.iter().copied().collect()
    }
    
    fn get_blocks_in_region(&self, min: VoxelPos, max: VoxelPos) -> Vec<(VoxelPos, BlockId)> {
        let mut blocks = Vec::new();
        
        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let pos = VoxelPos { x, y, z };
                    let block = self.get_block(pos);
                    if block != BlockId::AIR {
                        blocks.push((pos, block));
                    }
                }
            }
        }
        
        blocks
    }
    
    fn snapshot_timestamp(&self) -> Instant {
        self.timestamp
    }
    
    fn version(&self) -> u64 {
        self.version
    }
}

impl ChunkSnapshot {
    /// Create a new chunk snapshot
    pub fn new(position: ChunkPos, size: u32) -> Self {
        let volume = (size * size * 256) as usize; // Assume 256 height
        
        Self {
            blocks: vec![BlockId::AIR; volume],
            sky_light: vec![15; volume], // Full skylight by default
            block_light: vec![0; volume],
            position,
            size,
            is_generated: false,
            transparency_cache: vec![true; volume], // Air is transparent
        }
    }
    
    /// Get block at local chunk coordinates
    pub fn get_block_local(&self, (x, y, z): (u32, u32, u32)) -> BlockId {
        if let Some(index) = self.get_index(x, y, z) {
            self.blocks[index]
        } else {
            BlockId::AIR
        }
    }
    
    /// Get sky light at local chunk coordinates
    pub fn get_sky_light_local(&self, (x, y, z): (u32, u32, u32)) -> u8 {
        if let Some(index) = self.get_index(x, y, z) {
            self.sky_light[index]
        } else {
            15
        }
    }
    
    /// Get block light at local chunk coordinates
    pub fn get_block_light_local(&self, (x, y, z): (u32, u32, u32)) -> u8 {
        if let Some(index) = self.get_index(x, y, z) {
            self.block_light[index]
        } else {
            0
        }
    }
    
    /// Check if block is transparent at local coordinates
    pub fn is_block_transparent_local(&self, (x, y, z): (u32, u32, u32)) -> bool {
        if let Some(index) = self.get_index(x, y, z) {
            self.transparency_cache[index]
        } else {
            true
        }
    }
    
    /// Set block at local coordinates (for snapshot building)
    pub fn set_block_local(&mut self, (x, y, z): (u32, u32, u32), block: BlockId, is_transparent: bool) {
        if let Some(index) = self.get_index(x, y, z) {
            self.blocks[index] = block;
            self.transparency_cache[index] = is_transparent;
        }
    }
    
    /// Set lighting at local coordinates (for snapshot building)
    pub fn set_lighting_local(&mut self, (x, y, z): (u32, u32, u32), sky_light: u8, block_light: u8) {
        if let Some(index) = self.get_index(x, y, z) {
            self.sky_light[index] = sky_light;
            self.block_light[index] = block_light;
        }
    }
    
    /// Convert local coordinates to array index
    fn get_index(&self, x: u32, y: u32, z: u32) -> Option<usize> {
        if x >= self.size || z >= self.size || y >= 256 {
            return None;
        }
        Some(((y * self.size + z) * self.size + x) as usize)
    }
    
    /// Mark chunk as fully generated
    pub fn mark_generated(&mut self) {
        self.is_generated = true;
    }
}

impl WorldSnapshotBuilder {
    /// Add a chunk to the snapshot
    pub fn add_chunk(&mut self, chunk: ChunkSnapshot) {
        self.loaded_chunks.insert(chunk.position);
        self.chunk_data.insert(chunk.position, chunk);
    }
    
    /// Add surface height to cache
    pub fn add_surface_height(&mut self, x: i32, z: i32, height: i32) {
        self.surface_height_cache.insert((x, z), height);
    }
    
    /// Build the final snapshot
    pub fn build(self) -> WorldSnapshot {
        WorldSnapshot {
            chunk_data: Arc::new(self.chunk_data),
            loaded_chunks: Arc::new(self.loaded_chunks),
            chunk_size: self.chunk_size,
            timestamp: Instant::now(),
            version: self.version,
            surface_height_cache: Arc::new(self.surface_height_cache),
        }
    }
}

/// Statistics about a world snapshot
#[derive(Debug, Clone)]
pub struct SnapshotStats {
    pub chunk_count: usize,
    pub generated_chunk_count: usize,
    pub total_block_count: usize,
    pub snapshot_size_bytes: usize,
    pub creation_timestamp: Instant,
    pub version: u64,
}

/// Manager for creating and caching world snapshots
pub struct SnapshotManager {
    /// Current snapshot
    current_snapshot: RwLock<Option<Arc<WorldSnapshot>>>,
    
    /// Snapshot cache (keyed by version)
    snapshot_cache: RwLock<HashMap<u64, Arc<WorldSnapshot>>>,
    
    /// Maximum cache size
    max_cache_size: usize,
    
    /// Next version number
    next_version: std::sync::atomic::AtomicU64,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            current_snapshot: RwLock::new(None),
            snapshot_cache: RwLock::new(HashMap::new()),
            max_cache_size,
            next_version: std::sync::atomic::AtomicU64::new(1),
        }
    }
    
    /// Update the current snapshot
    pub fn update_snapshot(&self, snapshot: WorldSnapshot) {
        let snapshot = Arc::new(snapshot);
        
        // Update current
        *self.current_snapshot.write() = Some(snapshot.clone());
        
        // Add to cache
        let mut cache = self.snapshot_cache.write();
        cache.insert(snapshot.version(), snapshot);
        
        // Cleanup old snapshots if cache is too large
        if cache.len() > self.max_cache_size {
            let oldest_versions: Vec<_> = {
                let mut versions: Vec<_> = cache.keys().copied().collect();
                versions.sort();
                versions.into_iter().take(cache.len() - self.max_cache_size).collect()
            };
            
            for version in oldest_versions {
                cache.remove(&version);
            }
        }
    }
    
    /// Get the current snapshot
    pub fn get_current_snapshot(&self) -> Option<Arc<WorldSnapshot>> {
        self.current_snapshot.read().clone()
    }
    
    /// Get a specific snapshot version
    pub fn get_snapshot_version(&self, version: u64) -> Option<Arc<WorldSnapshot>> {
        self.snapshot_cache.read().get(&version).cloned()
    }
    
    /// Get next version number
    pub fn next_version(&self) -> u64 {
        self.next_version.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
    
    /// Clear all cached snapshots
    pub fn clear_cache(&self) {
        self.snapshot_cache.write().clear();
        *self.current_snapshot.write() = None;
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        let cache = self.snapshot_cache.read();
        let total_memory: usize = cache.values()
            .map(|snapshot| snapshot.estimate_memory_usage())
            .sum();
        
        CacheStats {
            cached_snapshot_count: cache.len(),
            total_memory_bytes: total_memory,
            oldest_version: cache.keys().min().copied(),
            newest_version: cache.keys().max().copied(),
        }
    }
}

/// Statistics about the snapshot cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub cached_snapshot_count: usize,
    pub total_memory_bytes: usize,
    pub oldest_version: Option<u64>,
    pub newest_version: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunk_snapshot_creation() {
        let chunk = ChunkSnapshot::new(ChunkPos { x: 0, y: 0, z: 0 }, 32);
        assert_eq!(chunk.size, 32);
        assert_eq!(chunk.blocks.len(), 32 * 32 * 256);
        assert!(!chunk.is_generated);
    }
    
    #[test]
    fn test_world_snapshot_builder() {
        let mut builder = WorldSnapshot::builder(32, 1);
        
        let mut chunk = ChunkSnapshot::new(ChunkPos { x: 0, y: 0, z: 0 }, 32);
        chunk.mark_generated();
        builder.add_chunk(chunk);
        
        let snapshot = builder.build();
        assert_eq!(snapshot.chunk_size(), 32);
        assert_eq!(snapshot.version(), 1);
        assert!(snapshot.is_chunk_loaded(ChunkPos { x: 0, y: 0, z: 0 }));
    }
    
    #[test]
    fn test_snapshot_manager() {
        let manager = SnapshotManager::new(3);
        
        let snapshot1 = WorldSnapshot::builder(32, 1).build();
        manager.update_snapshot(snapshot1);
        
        assert!(manager.get_current_snapshot().is_some());
        assert_eq!(manager.get_current_snapshot().unwrap().version(), 1);
    }
    
    #[test]
    fn test_block_access() {
        let mut builder = WorldSnapshot::builder(32, 1);
        
        let mut chunk = ChunkSnapshot::new(ChunkPos { x: 0, y: 0, z: 0 }, 32);
        chunk.set_block_local((1, 10, 1), BlockId(5), false);
        builder.add_chunk(chunk);
        
        let snapshot = builder.build();
        let block = snapshot.get_block(VoxelPos { x: 1, y: 10, z: 1 });
        assert_eq!(block, BlockId(5));
    }
}