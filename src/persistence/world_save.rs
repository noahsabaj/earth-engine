use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::CHUNK_SIZE;
use crate::persistence::{
    atomic_write, ChunkFormat, ChunkSerializer, PersistenceError, PersistenceResult, WorldMetadata,
};
use crate::{Chunk, ChunkData, ChunkPos, World};

/// World save structure
#[derive(Debug)]
pub struct WorldSave {
    /// Root directory for the save
    save_dir: PathBuf,
    /// Chunk serializer
    chunk_serializer: ChunkSerializer,
    /// Loaded chunks cache
    chunk_cache: HashMap<ChunkPos, Chunk>,
    /// Maximum chunks to keep in cache
    cache_size: usize,
    /// World name identifier
    pub world_name: String,
    /// World generation seed
    pub seed: u64,
    /// Default spawn position
    pub spawn_position: glam::Vec3,
    /// Game time in ticks
    pub game_time: u64,
}

/// Errors specific to world save operations
#[derive(Debug)]
pub enum WorldSaveError {
    DirectoryCreation(std::io::Error),
    ChunkSave(ChunkPos, PersistenceError),
    ChunkLoad(ChunkPos, PersistenceError),
    MetadataSave(PersistenceError),
    MetadataLoad(PersistenceError),
}

impl std::fmt::Display for WorldSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorldSaveError::DirectoryCreation(e) => write!(f, "Failed to create directory: {}", e),
            WorldSaveError::ChunkSave(pos, e) => write!(f, "Failed to save chunk {:?}: {}", pos, e),
            WorldSaveError::ChunkLoad(pos, e) => write!(f, "Failed to load chunk {:?}: {}", pos, e),
            WorldSaveError::MetadataSave(e) => write!(f, "Failed to save metadata: {}", e),
            WorldSaveError::MetadataLoad(e) => write!(f, "Failed to load metadata: {}", e),
        }
    }
}

impl std::error::Error for WorldSaveError {}

impl WorldSave {
    /// Create a new world save
    pub fn new<P: AsRef<Path>>(save_dir: P) -> PersistenceResult<Self> {
        let save_dir = save_dir.as_ref().to_path_buf();

        // Create directory structure
        Self::create_directory_structure(&save_dir)?;

        Ok(Self {
            save_dir,
            chunk_serializer: ChunkSerializer::new(ChunkFormat::RLE),
            chunk_cache: HashMap::new(),
            cache_size: 1024, // Cache up to 1024 chunks
            world_name: "default_world".to_string(),
            seed: 12345,
            spawn_position: Vec3::new(0.0, 64.0, 0.0),
            game_time: 0,
        })
    }

    /// Load an existing world save
    pub fn load<P: AsRef<Path>>(save_dir: P) -> PersistenceResult<Self> {
        let save_dir = save_dir.as_ref().to_path_buf();

        // Verify directory exists
        if !save_dir.exists() {
            return Err(PersistenceError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Save directory not found",
            )));
        }

        // Verify it's a valid save
        let metadata_path = save_dir.join("world.meta");
        if !metadata_path.exists() {
            return Err(PersistenceError::CorruptedData(
                "Missing world metadata".to_string(),
            ));
        }

        Ok(Self {
            save_dir,
            chunk_serializer: ChunkSerializer::new(ChunkFormat::RLE),
            chunk_cache: HashMap::new(),
            cache_size: 1024,
            world_name: "loaded_world".to_string(),
            seed: 54321, // Default seed for loaded worlds
            spawn_position: Vec3::new(0.0, 64.0, 0.0),
            game_time: 0,
        })
    }

    /// Save the entire world
    pub fn save_world(
        &mut self,
        world: &World,
        metadata: &WorldMetadata,
    ) -> Result<(), WorldSaveError> {
        // Save metadata
        self.save_metadata(metadata)
            .map_err(|e| WorldSaveError::MetadataSave(e))?;

        // Save all loaded chunks
        for (pos, chunk) in world.chunks() {
            self.save_chunk(chunk)
                .map_err(|e| WorldSaveError::ChunkSave(pos, e))?;
        }

        // Flush cache
        self.flush_cache()?;

        Ok(())
    }

    /// Load world metadata
    pub fn load_metadata(&self) -> Result<WorldMetadata, WorldSaveError> {
        let metadata_path = self.save_dir.join("world.meta");
        let data = fs::read(metadata_path).map_err(|e| WorldSaveError::MetadataLoad(e.into()))?;

        let metadata: WorldMetadata =
            bincode::deserialize(&data).map_err(|e| WorldSaveError::MetadataLoad(e.into()))?;

        Ok(metadata)
    }

    /// Save world metadata
    pub fn save_metadata(&self, metadata: &WorldMetadata) -> PersistenceResult<()> {
        let metadata_path = self.save_dir.join("world.meta");
        let data = bincode::serialize(metadata)?;
        atomic_write(metadata_path, &data)?;
        Ok(())
    }

    /// Save a single chunk
    pub fn save_chunk(&mut self, chunk: &dyn ChunkData) -> PersistenceResult<()> {
        let chunk_path = self.get_chunk_path(chunk.position());

        // Ensure chunk directory exists
        if let Some(parent) = chunk_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Downcast to ChunkSoA for serialization
        let chunk_soa = chunk
            .as_any()
            .downcast_ref::<crate::world::storage::ChunkSoA>()
            .ok_or_else(|| {
                PersistenceError::SerializationError(
                    "Failed to downcast ChunkData to ChunkSoA".to_string(),
                )
            })?;

        // Analyze chunk to determine best format
        let format = ChunkSerializer::analyze_chunk(chunk_soa);
        let serializer = ChunkSerializer::new(format);

        // Serialize and save
        let data = serializer.serialize(chunk_soa)?;
        atomic_write(&chunk_path, &data)?;

        // Cache management would go here
        // For now, we don't cache on save since Chunk doesn't implement Clone

        Ok(())
    }

    /// Load a single chunk
    pub fn load_chunk(&mut self, pos: ChunkPos) -> PersistenceResult<Option<Chunk>> {
        let chunk_path = self.get_chunk_path(pos);

        // Check if chunk file exists
        if !chunk_path.exists() {
            return Ok(None);
        }

        // Load and deserialize
        let data = fs::read(&chunk_path)?;
        let chunk = self.chunk_serializer.deserialize(&data)?;

        Ok(Some(chunk))
    }

    /// Delete a chunk
    pub fn delete_chunk(&mut self, pos: ChunkPos) -> PersistenceResult<()> {
        let chunk_path = self.get_chunk_path(pos);

        if chunk_path.exists() {
            fs::remove_file(chunk_path)?;
        }

        // Remove from cache
        self.chunk_cache.remove(&pos);

        Ok(())
    }

    /// Save multiple chunks efficiently
    pub fn save_chunks(
        &mut self,
        chunks: &[&dyn ChunkData],
    ) -> Result<(), Vec<(ChunkPos, WorldSaveError)>> {
        let mut errors = Vec::new();

        for &chunk in chunks {
            if let Err(e) = self.save_chunk(chunk) {
                errors.push((
                    chunk.position(),
                    WorldSaveError::ChunkSave(chunk.position(), e),
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Load multiple chunks efficiently
    pub fn load_chunks(&mut self, positions: &[ChunkPos]) -> HashMap<ChunkPos, Chunk> {
        let mut loaded = HashMap::new();

        for &pos in positions {
            if let Ok(Some(chunk)) = self.load_chunk(pos) {
                loaded.insert(pos, chunk);
            }
        }

        loaded
    }

    /// Get the file path for a chunk
    fn get_chunk_path(&self, pos: ChunkPos) -> PathBuf {
        // Use a directory structure to avoid too many files in one directory
        // chunks/rx/rz/chunk_x_y_z.ecnk
        let region_x = pos.x >> 5; // Divide by 32
        let region_z = pos.z >> 5;

        self.save_dir
            .join("chunks")
            .join(format!("r{}", region_x))
            .join(format!("r{}", region_z))
            .join(format!("chunk_{}_{}.ecnk", pos.x, pos.z))
    }

    /// Create the save directory structure
    fn create_directory_structure(save_dir: &Path) -> PersistenceResult<()> {
        fs::create_dir_all(save_dir)?;
        fs::create_dir_all(save_dir.join("chunks"))?;
        fs::create_dir_all(save_dir.join("players"))?;
        fs::create_dir_all(save_dir.join("backups"))?;
        Ok(())
    }

    /// Evict oldest chunks from cache if needed
    fn evict_cache_if_needed(&mut self) {
        if self.chunk_cache.len() <= self.cache_size {
            return;
        }

        // Simple LRU: remove random chunks
        // In production, use proper LRU cache
        let to_remove = self.chunk_cache.len() - self.cache_size;
        let keys: Vec<_> = self.chunk_cache.keys().cloned().take(to_remove).collect();

        for key in keys {
            self.chunk_cache.remove(&key);
        }
    }

    /// Flush all cached chunks to disk
    pub fn flush_cache(&mut self) -> Result<(), WorldSaveError> {
        // Take ownership of the cache temporarily
        let cache = std::mem::take(&mut self.chunk_cache);

        // Save each chunk
        for (pos, chunk) in cache {
            self.save_chunk(&chunk)
                .map_err(|e| WorldSaveError::ChunkSave(pos, e))?;
        }

        // Cache is now empty (from mem::take)
        Ok(())
    }

    /// Get save directory path
    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }

    /// Get statistics about the save
    pub fn get_stats(&self) -> SaveStats {
        let chunks_dir = self.save_dir.join("chunks");
        let mut chunk_count = 0;
        let mut total_size = 0;

        if let Ok(entries) = fs::read_dir(&chunks_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() && entry.path().extension() == Some("ecnk".as_ref()) {
                        chunk_count += 1;
                        total_size += metadata.len();
                    }
                }
            }
        }

        SaveStats {
            chunk_count,
            total_size,
            cache_size: self.chunk_cache.len(),
        }
    }
}

/// Statistics about a world save
#[derive(Debug, Clone)]
pub struct SaveStats {
    pub chunk_count: usize,
    pub total_size: u64,
    pub cache_size: usize,
}

/// World save format information
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveFormat {
    pub version: u32,
    pub chunk_format: u8,
    pub compression: Option<String>,
}

impl Default for SaveFormat {
    fn default() -> Self {
        Self {
            version: 1,
            chunk_format: 1, // RLE
            compression: Some("zstd".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_world_save_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
        let save = WorldSave::new(temp_dir.path()).expect("Failed to create WorldSave");

        assert!(temp_dir.path().join("chunks").exists());
        assert!(temp_dir.path().join("players").exists());
        assert!(temp_dir.path().join("backups").exists());
    }

    #[test]
    fn test_chunk_save_load() {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory for test");
        let mut save = WorldSave::new(temp_dir.path()).expect("Failed to create WorldSave");

        let mut chunk = Chunk::new(ChunkPos { x: 10, y: 0, z: -5 }, CHUNK_SIZE);
        chunk.set_block_at(
            crate::world::VoxelPos::new(15, 20, 10),
            crate::world::BlockId(42),
        );

        // Save chunk
        save.save_chunk(&chunk).expect("Failed to save chunk");

        // Clear cache to force disk load
        save.chunk_cache.clear();

        // Load chunk
        let loaded = save
            .load_chunk(chunk.position())
            .expect("Failed to load chunk")
            .expect("Chunk should exist after saving");

        assert_eq!(
            chunk.get_block_at(crate::world::VoxelPos::new(15, 20, 10)),
            loaded.get_block_at(crate::world::VoxelPos::new(15, 20, 10))
        );
    }
}
