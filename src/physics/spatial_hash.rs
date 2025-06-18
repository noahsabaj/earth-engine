use super::{EntityId, physics_tables::AABB};
use std::collections::HashMap;
use rayon::prelude::*;
use parking_lot::RwLock;

/// Spatial hash configuration
#[derive(Debug, Clone)]
pub struct SpatialHashConfig {
    pub cell_size: f32,
    pub world_min: [f32; 3],
    pub world_max: [f32; 3],
    pub expected_entities_per_cell: usize,
}

impl Default for SpatialHashConfig {
    fn default() -> Self {
        Self {
            cell_size: 4.0,
            world_min: [-1000.0, -100.0, -1000.0],
            world_max: [1000.0, 300.0, 1000.0],
            expected_entities_per_cell: 8,
        }
    }
}

/// Cell coordinate in the spatial hash grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CellCoord {
    x: i32,
    y: i32,
    z: i32,
}

impl CellCoord {
    fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Spatial hash grid for broad-phase collision detection
pub struct SpatialHash {
    config: SpatialHashConfig,
    cells: RwLock<HashMap<CellCoord, Vec<EntityId>>>,
    entity_cells: RwLock<HashMap<EntityId, Vec<CellCoord>>>,
}

impl SpatialHash {
    pub fn new(config: SpatialHashConfig) -> Self {
        let expected_cells = 
            ((config.world_max[0] - config.world_min[0]) / config.cell_size) as usize *
            ((config.world_max[1] - config.world_min[1]) / config.cell_size) as usize *
            ((config.world_max[2] - config.world_min[2]) / config.cell_size) as usize;
        
        Self {
            config,
            cells: RwLock::new(HashMap::with_capacity(expected_cells)),
            entity_cells: RwLock::new(HashMap::with_capacity(super::MAX_ENTITIES)),
        }
    }
    
    /// Clear all spatial hash data
    pub fn clear(&self) {
        self.cells.write().clear();
        self.entity_cells.write().clear();
    }
    
    /// Convert world position to cell coordinate
    fn world_to_cell(&self, pos: [f32; 3]) -> CellCoord {
        CellCoord::new(
            ((pos[0] - self.config.world_min[0]) / self.config.cell_size).floor() as i32,
            ((pos[1] - self.config.world_min[1]) / self.config.cell_size).floor() as i32,
            ((pos[2] - self.config.world_min[2]) / self.config.cell_size).floor() as i32,
        )
    }
    
    /// Get all cell coordinates that an AABB overlaps
    fn get_overlapping_cells(&self, aabb: &AABB) -> Vec<CellCoord> {
        let min_cell = self.world_to_cell(aabb.min);
        let max_cell = self.world_to_cell(aabb.max);
        
        let mut cells = Vec::with_capacity(
            ((max_cell.x - min_cell.x + 1) *
             (max_cell.y - min_cell.y + 1) *
             (max_cell.z - min_cell.z + 1)) as usize
        );
        
        for x in min_cell.x..=max_cell.x {
            for y in min_cell.y..=max_cell.y {
                for z in min_cell.z..=max_cell.z {
                    cells.push(CellCoord::new(x, y, z));
                }
            }
        }
        
        cells
    }
    
    /// Insert an entity into the spatial hash
    pub fn insert(&self, entity: EntityId, aabb: &AABB) {
        let cells_to_insert = self.get_overlapping_cells(aabb);
        
        // Update cells
        {
            let mut cells = self.cells.write();
            for cell_coord in &cells_to_insert {
                cells.entry(*cell_coord)
                    .or_insert_with(|| Vec::with_capacity(self.config.expected_entities_per_cell))
                    .push(entity);
            }
        }
        
        // Track which cells this entity is in
        {
            let mut entity_cells = self.entity_cells.write();
            entity_cells.insert(entity, cells_to_insert);
        }
    }
    
    /// Remove an entity from the spatial hash
    pub fn remove(&self, entity: EntityId) {
        // Get cells this entity was in
        let cells_to_remove = {
            let mut entity_cells = self.entity_cells.write();
            entity_cells.remove(&entity).unwrap_or_default()
        };
        
        // Remove from cells
        if !cells_to_remove.is_empty() {
            let mut cells = self.cells.write();
            for cell_coord in cells_to_remove {
                if let Some(entities) = cells.get_mut(&cell_coord) {
                    entities.retain(|&e| e != entity);
                    if entities.is_empty() {
                        cells.remove(&cell_coord);
                    }
                }
            }
        }
    }
    
    /// Update an entity's position in the spatial hash
    pub fn update(&self, entity: EntityId, aabb: &AABB) {
        let new_cells = self.get_overlapping_cells(aabb);
        
        // Get current cells
        let old_cells = {
            let entity_cells = self.entity_cells.read();
            entity_cells.get(&entity).cloned().unwrap_or_default()
        };
        
        // Quick check: if cells haven't changed, nothing to do
        if old_cells == new_cells {
            return;
        }
        
        // Remove from old cells that are no longer occupied
        {
            let mut cells = self.cells.write();
            for old_cell in &old_cells {
                if !new_cells.contains(old_cell) {
                    if let Some(entities) = cells.get_mut(old_cell) {
                        entities.retain(|&e| e != entity);
                        if entities.is_empty() {
                            cells.remove(old_cell);
                        }
                    }
                }
            }
            
            // Add to new cells
            for new_cell in &new_cells {
                if !old_cells.contains(new_cell) {
                    cells.entry(*new_cell)
                        .or_insert_with(|| Vec::with_capacity(self.config.expected_entities_per_cell))
                        .push(entity);
                }
            }
        }
        
        // Update entity's cell list
        {
            let mut entity_cells = self.entity_cells.write();
            entity_cells.insert(entity, new_cells);
        }
    }
    
    /// Query entities in a region
    pub fn query_region(&self, aabb: &AABB) -> Vec<EntityId> {
        let cells_to_check = self.get_overlapping_cells(aabb);
        let mut entities = Vec::new();
        let mut seen = std::collections::HashSet::new();
        
        let cells = self.cells.read();
        for cell_coord in cells_to_check {
            if let Some(cell_entities) = cells.get(&cell_coord) {
                for &entity in cell_entities {
                    if seen.insert(entity) {
                        entities.push(entity);
                    }
                }
            }
        }
        
        entities
    }
    
    /// Get potential collision pairs for a single entity
    pub fn get_potential_collisions(&self, entity: EntityId) -> Vec<EntityId> {
        let entity_cells = self.entity_cells.read();
        let cells_to_check = match entity_cells.get(&entity) {
            Some(cells) => cells.clone(),
            None => return Vec::new(),
        };
        drop(entity_cells);
        
        let mut potential = Vec::new();
        let mut seen = std::collections::HashSet::new();
        
        let cells = self.cells.read();
        for cell_coord in cells_to_check {
            if let Some(cell_entities) = cells.get(&cell_coord) {
                for &other in cell_entities {
                    if other != entity && seen.insert(other) {
                        potential.push(other);
                    }
                }
            }
        }
        
        potential
    }
    
    /// Get all potential collision pairs (for parallel processing)
    pub fn get_all_potential_pairs(&self) -> Vec<(EntityId, EntityId)> {
        let cells = self.cells.read();
        
        // Collect all pairs from each cell
        let cell_pairs: Vec<_> = cells.par_iter()
            .flat_map(|(_, entities)| {
                let mut pairs = Vec::new();
                for (i, &entity_a) in entities.iter().enumerate() {
                    for &entity_b in &entities[i + 1..] {
                        if entity_a.0 < entity_b.0 {
                            pairs.push((entity_a, entity_b));
                        } else {
                            pairs.push((entity_b, entity_a));
                        }
                    }
                }
                pairs
            })
            .collect();
        
        // Remove duplicates
        let mut unique_pairs: Vec<_> = cell_pairs.into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        
        unique_pairs.sort_unstable();
        unique_pairs
    }
    
    /// Update multiple entities in parallel
    pub fn batch_update(&self, updates: &[(EntityId, AABB)]) {
        updates.par_iter().for_each(|(entity, aabb)| {
            self.update(*entity, aabb);
        });
    }
    
    /// Get statistics about the spatial hash
    pub fn get_stats(&self) -> SpatialHashStats {
        let cells = self.cells.read();
        let entity_cells = self.entity_cells.read();
        
        let mut max_entities_per_cell = 0;
        let mut total_entities_in_cells = 0;
        
        for (_, entities) in cells.iter() {
            let count = entities.len();
            max_entities_per_cell = max_entities_per_cell.max(count);
            total_entities_in_cells += count;
        }
        
        SpatialHashStats {
            total_cells: cells.len(),
            total_entities: entity_cells.len(),
            max_entities_per_cell,
            avg_entities_per_cell: if cells.is_empty() { 
                0.0 
            } else { 
                total_entities_in_cells as f32 / cells.len() as f32 
            },
        }
    }
}

#[derive(Debug)]
pub struct SpatialHashStats {
    pub total_cells: usize,
    pub total_entities: usize,
    pub max_entities_per_cell: usize,
    pub avg_entities_per_cell: f32,
}