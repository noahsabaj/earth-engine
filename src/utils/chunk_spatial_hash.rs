/// Spatial hash implementation for chunk positions using pre-allocated arrays
/// This replaces HashMap<ChunkPos, T> with zero-allocation lookups

use crate::world::ChunkPos;

/// Maximum chunks per dimension (covers -MAX_COORD to MAX_COORD)
pub const MAX_CHUNK_COORD: i32 = 256;
pub const CHUNK_ARRAY_SIZE: usize = (MAX_CHUNK_COORD * 2) as usize;
pub const TOTAL_CHUNK_SLOTS: usize = CHUNK_ARRAY_SIZE * CHUNK_ARRAY_SIZE * CHUNK_ARRAY_SIZE;

/// Convert ChunkPos to array index
#[inline(always)]
pub fn chunk_pos_to_index(pos: ChunkPos) -> Option<usize> {
    // Check bounds
    if pos.x < -MAX_CHUNK_COORD || pos.x >= MAX_CHUNK_COORD ||
       pos.y < -MAX_CHUNK_COORD || pos.y >= MAX_CHUNK_COORD ||
       pos.z < -MAX_CHUNK_COORD || pos.z >= MAX_CHUNK_COORD {
        return None;
    }
    
    // Convert to positive indices
    let x = (pos.x + MAX_CHUNK_COORD) as usize;
    let y = (pos.y + MAX_CHUNK_COORD) as usize;
    let z = (pos.z + MAX_CHUNK_COORD) as usize;
    
    Some(x + y * CHUNK_ARRAY_SIZE + z * CHUNK_ARRAY_SIZE * CHUNK_ARRAY_SIZE)
}

/// Convert array index back to ChunkPos
#[inline(always)]
pub fn index_to_chunk_pos(index: usize) -> ChunkPos {
    let z = index / (CHUNK_ARRAY_SIZE * CHUNK_ARRAY_SIZE);
    let remainder = index % (CHUNK_ARRAY_SIZE * CHUNK_ARRAY_SIZE);
    let y = remainder / CHUNK_ARRAY_SIZE;
    let x = remainder % CHUNK_ARRAY_SIZE;
    
    ChunkPos::new(
        x as i32 - MAX_CHUNK_COORD,
        y as i32 - MAX_CHUNK_COORD,
        z as i32 - MAX_CHUNK_COORD,
    )
}

/// Pre-allocated chunk storage with spatial hashing
#[derive(Debug)]
pub struct ChunkSpatialHash<T> {
    /// Dense storage for chunk data
    data: Vec<Option<T>>,
    /// Track active chunks for efficient iteration
    active_indices: Vec<usize>,
    /// Reverse lookup: index -> position in active_indices
    index_to_active: Vec<Option<usize>>,
}

impl<T> ChunkSpatialHash<T> {
    pub fn new() -> Self {
        // Initialize vectors without using clone
        let mut data = Vec::with_capacity(TOTAL_CHUNK_SLOTS);
        let mut index_to_active = Vec::with_capacity(TOTAL_CHUNK_SLOTS);
        
        for _ in 0..TOTAL_CHUNK_SLOTS {
            data.push(None);
            index_to_active.push(None);
        }
        
        Self {
            data,
            active_indices: Vec::with_capacity(4096), // Reasonable initial capacity
            index_to_active,
        }
    }
    
    /// Insert or update a chunk
    pub fn insert(&mut self, pos: ChunkPos, value: T) -> Option<T> {
        if let Some(index) = chunk_pos_to_index(pos) {
            let old_value = self.data[index].take();
            
            // If this is a new chunk, add to active list
            if old_value.is_none() {
                let active_pos = self.active_indices.len();
                self.active_indices.push(index);
                self.index_to_active[index] = Some(active_pos);
            }
            
            self.data[index] = Some(value);
            old_value
        } else {
            None
        }
    }
    
    /// Get a chunk
    pub fn get(&self, pos: ChunkPos) -> Option<&T> {
        chunk_pos_to_index(pos)
            .and_then(|index| self.data[index].as_ref())
    }
    
    /// Get a mutable chunk
    pub fn get_mut(&mut self, pos: ChunkPos) -> Option<&mut T> {
        chunk_pos_to_index(pos)
            .and_then(|index| self.data[index].as_mut())
    }
    
    /// Remove a chunk
    pub fn remove(&mut self, pos: ChunkPos) -> Option<T> {
        if let Some(index) = chunk_pos_to_index(pos) {
            if let Some(value) = self.data[index].take() {
                // Remove from active list
                if let Some(active_pos) = self.index_to_active[index].take() {
                    // Swap with last element for O(1) removal
                    let last_index = self.active_indices.len() - 1;
                    if active_pos < last_index {
                        self.active_indices[active_pos] = self.active_indices[last_index];
                        // Update the moved element's reverse lookup
                        let moved_index = self.active_indices[active_pos];
                        self.index_to_active[moved_index] = Some(active_pos);
                    }
                    self.active_indices.pop();
                }
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Check if a chunk exists
    pub fn contains(&self, pos: ChunkPos) -> bool {
        chunk_pos_to_index(pos)
            .map(|index| self.data[index].is_some())
            .unwrap_or(false)
    }
    
    /// Iterate over all chunks
    pub fn iter(&self) -> impl Iterator<Item = (ChunkPos, &T)> {
        self.active_indices.iter()
            .filter_map(move |&index| {
                self.data[index].as_ref().map(|value| {
                    (index_to_chunk_pos(index), value)
                })
            })
    }
    
    /// Iterate over all chunks mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ChunkPos, &mut T)> {
        // Collect indices first to avoid closure capture issues
        let indices: Vec<usize> = self.active_indices.clone();
        let data_ptr = self.data.as_mut_ptr();
        
        // SAFETY: We're iterating over known valid indices from active_indices
        // and each index is unique, so we're not creating multiple mutable references
        // to the same data
        indices.into_iter()
            .filter_map(move |index| {
                unsafe {
                    let slot = &mut *data_ptr.add(index);
                    slot.as_mut().map(|value| {
                        (index_to_chunk_pos(index), value)
                    })
                }
            })
    }
    
    /// Get the number of active chunks
    pub fn len(&self) -> usize {
        self.active_indices.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.active_indices.is_empty()
    }
    
    /// Clear all chunks
    pub fn clear(&mut self) {
        for &index in &self.active_indices {
            self.data[index] = None;
            self.index_to_active[index] = None;
        }
        self.active_indices.clear();
    }
    
    /// Retain chunks based on a predicate
    pub fn retain<F>(&mut self, mut f: F) 
    where
        F: FnMut(ChunkPos, &mut T) -> bool
    {
        let mut write_pos = 0;
        
        for read_pos in 0..self.active_indices.len() {
            let index = self.active_indices[read_pos];
            let pos = index_to_chunk_pos(index);
            
            if let Some(value) = &mut self.data[index] {
                if f(pos, value) {
                    // Keep this chunk
                    if write_pos != read_pos {
                        self.active_indices[write_pos] = index;
                        self.index_to_active[index] = Some(write_pos);
                    }
                    write_pos += 1;
                } else {
                    // Remove this chunk
                    self.data[index] = None;
                    self.index_to_active[index] = None;
                }
            }
        }
        
        self.active_indices.truncate(write_pos);
    }
}

/// Specialized version for view distance culling
#[derive(Debug)]
pub struct ChunkDistanceHash<T> {
    storage: ChunkSpatialHash<T>,
    /// Cached center position for distance calculations
    center: ChunkPos,
    /// Maximum view distance squared
    max_distance_sq: i32,
}

impl<T> ChunkDistanceHash<T> {
    pub fn new(view_distance: i32) -> Self {
        Self {
            storage: ChunkSpatialHash::new(),
            center: ChunkPos::new(0, 0, 0),
            max_distance_sq: view_distance * view_distance,
        }
    }
    
    /// Update center position and cull distant chunks
    pub fn update_center(&mut self, new_center: ChunkPos) {
        self.center = new_center;
        self.storage.retain(|pos, _| {
            pos.distance_squared_to(self.center) <= self.max_distance_sq
        });
    }
    
    pub fn insert(&mut self, pos: ChunkPos, value: T) -> Option<T> {
        // Only insert if within view distance
        if pos.distance_squared_to(self.center) <= self.max_distance_sq {
            self.storage.insert(pos, value)
        } else {
            None
        }
    }
    
    // Delegate other methods to storage
    pub fn get(&self, pos: ChunkPos) -> Option<&T> {
        self.storage.get(pos)
    }
    
    pub fn get_mut(&mut self, pos: ChunkPos) -> Option<&mut T> {
        self.storage.get_mut(pos)
    }
    
    pub fn remove(&mut self, pos: ChunkPos) -> Option<T> {
        self.storage.remove(pos)
    }
    
    pub fn iter(&self) -> impl Iterator<Item = (ChunkPos, &T)> {
        self.storage.iter()
    }
    
    pub fn len(&self) -> usize {
        self.storage.len()
    }
}