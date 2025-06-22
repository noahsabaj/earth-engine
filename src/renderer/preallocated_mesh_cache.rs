use super::mesh_optimizer::{MeshLod, OptimizedMesh};
use crate::error::EngineError;
use crate::utils::chunk_spatial_hash::{chunk_pos_to_index, MAX_CHUNK_COORD};
/// Pre-allocated mesh cache using spatial hashing for chunk positions
/// Replaces HashMap-based cache with zero-allocation lookups
///
/// ## Error Handling Pattern
/// This module uses proper error propagation instead of unwrap() calls:
/// - RwLock operations return Result<T, EngineError> using the ? operator
/// - Array access is bounds-checked before indexing
/// - Position validation uses ok_or_else() for proper error messages
/// - All methods return Result types to enable error propagation up the call stack
use crate::ChunkPos;
use std::sync::RwLock;

/// Maximum LOD levels
const MAX_LOD_LEVELS: usize = 5;

/// Cache entry for a single LOD level
struct LodCacheEntry {
    mesh: Option<OptimizedMesh>,
    last_access: u64,
    size_bytes: usize,
}

/// Pre-allocated mesh cache
pub struct PreallocatedMeshCache {
    /// Dense array indexed by chunk position and LOD
    /// Layout: [chunk_index][lod_level]
    entries: RwLock<Vec<[LodCacheEntry; MAX_LOD_LEVELS]>>,

    /// Track total cache size
    current_size_bytes: RwLock<usize>,
    max_size_bytes: usize,

    /// Access counter for LRU
    access_counter: RwLock<u64>,

    /// Track active entries for efficient iteration
    active_entries: RwLock<Vec<(usize, usize)>>, // (chunk_index, lod_index)
}

impl PreallocatedMeshCache {
    pub fn new(max_size_mb: usize) -> Self {
        // Calculate total possible chunk positions
        let total_chunks = (MAX_CHUNK_COORD * 2) as usize;
        let total_positions = total_chunks * total_chunks * total_chunks;

        // Pre-allocate cache entries
        let mut entries = Vec::with_capacity(total_positions);
        for _ in 0..total_positions {
            entries.push([
                LodCacheEntry {
                    mesh: None,
                    last_access: 0,
                    size_bytes: 0,
                },
                LodCacheEntry {
                    mesh: None,
                    last_access: 0,
                    size_bytes: 0,
                },
                LodCacheEntry {
                    mesh: None,
                    last_access: 0,
                    size_bytes: 0,
                },
                LodCacheEntry {
                    mesh: None,
                    last_access: 0,
                    size_bytes: 0,
                },
                LodCacheEntry {
                    mesh: None,
                    last_access: 0,
                    size_bytes: 0,
                },
            ]);
        }

        Self {
            entries: RwLock::new(entries),
            current_size_bytes: RwLock::new(0),
            max_size_bytes: max_size_mb * 1024 * 1024,
            access_counter: RwLock::new(0),
            active_entries: RwLock::new(Vec::with_capacity(4096)),
        }
    }

    /// Get LOD index from enum
    fn lod_to_index(lod: MeshLod) -> usize {
        match lod {
            MeshLod::Lod0 => 0,
            MeshLod::Lod1 => 1,
            MeshLod::Lod2 => 2,
            MeshLod::Lod3 => 3,
            MeshLod::Lod4 => 4,
        }
    }

    /// Get a mesh from cache
    pub fn get(
        &self,
        chunk_pos: ChunkPos,
        lod: MeshLod,
    ) -> Result<Option<OptimizedMesh>, EngineError> {
        let chunk_index =
            chunk_pos_to_index(chunk_pos).ok_or_else(|| EngineError::BufferAccess {
                index: 0,
                size: (MAX_CHUNK_COORD * 2) as usize,
            })?;
        let lod_index = Self::lod_to_index(lod);

        let mut entries = self.entries.write()?;
        let mut counter = self.access_counter.write()?;

        if let Some(entry) = entries.get_mut(chunk_index).and_then(|lod_entries| {
            if lod_index < MAX_LOD_LEVELS {
                lod_entries.get_mut(lod_index)
            } else {
                None
            }
        }) {
            if let Some(ref mesh) = entry.mesh {
                *counter += 1;
                entry.last_access = *counter;
                return Ok(Some((*mesh).clone()));
            }
        }

        Ok(None)
    }

    /// Insert a mesh into cache
    pub fn insert(
        &self,
        chunk_pos: ChunkPos,
        lod: MeshLod,
        mesh: OptimizedMesh,
    ) -> Result<(), EngineError> {
        let chunk_index =
            chunk_pos_to_index(chunk_pos).ok_or_else(|| EngineError::BufferAccess {
                index: 0,
                size: (MAX_CHUNK_COORD * 2) as usize,
            })?;
        let lod_index = Self::lod_to_index(lod);

        let mesh_size = Self::estimate_mesh_size(&mesh);

        // Check if we need to evict entries
        {
            let current_size = *self.current_size_bytes.read()?;
            if current_size + mesh_size > self.max_size_bytes {
                self.evict_lru(mesh_size)?;
            }
        }

        // Insert the mesh
        let mut entries = self.entries.write()?;
        let mut current_size = self.current_size_bytes.write()?;
        let mut counter = self.access_counter.write()?;
        let mut active = self.active_entries.write()?;

        *counter += 1;

        if let Some(lod_entries) = entries.get_mut(chunk_index) {
            if lod_index >= MAX_LOD_LEVELS {
                return Err(EngineError::BufferAccess {
                    index: lod_index,
                    size: MAX_LOD_LEVELS,
                });
            }
            let entry = &mut lod_entries[lod_index];

            // Remove old entry size if replacing
            if entry.mesh.is_some() {
                *current_size -= entry.size_bytes;
            } else {
                // New entry, add to active list
                active.push((chunk_index, lod_index));
            }

            // Insert new entry
            entry.mesh = Some(mesh);
            entry.last_access = *counter;
            entry.size_bytes = mesh_size;
            *current_size += mesh_size;
        }

        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&self) -> Result<(), EngineError> {
        let mut entries = self.entries.write()?;
        let mut current_size = self.current_size_bytes.write()?;
        let mut active = self.active_entries.write()?;

        for (chunk_idx, lod_idx) in active.iter() {
            if let Some(lod_entries) = entries.get_mut(*chunk_idx) {
                if *lod_idx < MAX_LOD_LEVELS {
                    lod_entries[*lod_idx] = LodCacheEntry {
                        mesh: None,
                        last_access: 0,
                        size_bytes: 0,
                    };
                }
            }
        }

        *current_size = 0;
        active.clear();
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats, EngineError> {
        let current_size = *self.current_size_bytes.read()?;
        let active = self.active_entries.read()?;

        Ok(CacheStats {
            entries: active.len(),
            size_mb: current_size as f32 / (1024.0 * 1024.0),
            capacity_mb: self.max_size_bytes as f32 / (1024.0 * 1024.0),
        })
    }

    /// Estimate mesh size in bytes
    fn estimate_mesh_size(mesh: &OptimizedMesh) -> usize {
        use crate::renderer::Vertex;
        mesh.vertices.len() * std::mem::size_of::<Vertex>()
            + mesh.indices.len() * std::mem::size_of::<u32>()
            + std::mem::size_of::<super::MeshStats>()
    }

    /// Evict least recently used entries to make room
    fn evict_lru(&self, needed_bytes: usize) -> Result<(), EngineError> {
        let mut entries = self.entries.write()?;
        let mut current_size = self.current_size_bytes.write()?;
        let mut active = self.active_entries.write()?;

        // Sort active entries by last access time
        let mut sorted_active: Vec<_> = active
            .iter()
            .enumerate()
            .filter_map(|(idx, &(chunk_idx, lod_idx))| {
                entries
                    .get(chunk_idx)
                    .and_then(|lod_entries| lod_entries.get(lod_idx))
                    .and_then(|entry| {
                        entry
                            .mesh
                            .as_ref()
                            .map(|_| (idx, chunk_idx, lod_idx, entry.last_access))
                    })
            })
            .collect();

        sorted_active.sort_by_key(|&(_, _, _, access)| access);

        // Evict oldest entries until we have enough space
        let mut freed_bytes = 0;
        let mut indices_to_remove = Vec::new();

        for (active_idx, chunk_idx, lod_idx, _) in sorted_active {
            if freed_bytes >= needed_bytes {
                break;
            }

            if let Some(lod_entries) = entries.get_mut(chunk_idx) {
                if lod_idx >= MAX_LOD_LEVELS {
                    continue;
                }
                let entry = &mut lod_entries[lod_idx];
                if entry.mesh.is_some() {
                    freed_bytes += entry.size_bytes;
                    *current_size -= entry.size_bytes;
                    entry.mesh = None;
                    entry.last_access = 0;
                    entry.size_bytes = 0;
                    indices_to_remove.push(active_idx);
                }
            }
        }

        // Remove evicted entries from active list
        indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort descending
        for idx in indices_to_remove {
            active.swap_remove(idx);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub entries: usize,
    pub size_mb: f32,
    pub capacity_mb: f32,
}
