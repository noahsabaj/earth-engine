/// Pre-allocated spatial hash implementation using fixed-size arrays
/// Replaces HashMap-based spatial hash with zero-allocation lookups
use super::{physics_tables::AABB, EntityId, MAX_ENTITIES};
use parking_lot::RwLock;
use rayon::prelude::*;

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

/// Maximum cells per dimension
const MAX_CELLS_PER_DIM: usize = 512;
const TOTAL_CELLS: usize = MAX_CELLS_PER_DIM * MAX_CELLS_PER_DIM * MAX_CELLS_PER_DIM;

/// Cell coordinate in the spatial hash grid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CellCoord {
    x: u16,
    y: u16,
    z: u16,
}

impl CellCoord {
    fn new(x: u16, y: u16, z: u16) -> Self {
        Self { x, y, z }
    }

    fn to_index(self) -> usize {
        self.x as usize
            + self.y as usize * MAX_CELLS_PER_DIM
            + self.z as usize * MAX_CELLS_PER_DIM * MAX_CELLS_PER_DIM
    }
}

/// Pre-allocated cell data
struct Cell {
    entities: Vec<EntityId>,
    count: usize,
}

impl Cell {
    fn new(capacity: usize) -> Self {
        Self {
            entities: Vec::with_capacity(capacity),
            count: 0,
        }
    }

    fn clear(&mut self) {
        self.count = 0;
        self.entities.clear();
    }

    fn add(&mut self, entity: EntityId) {
        if self.count < self.entities.capacity() {
            if self.count < self.entities.len() {
                self.entities[self.count] = entity;
            } else {
                self.entities.push(entity);
            }
            self.count += 1;
        }
    }

    fn remove(&mut self, entity: EntityId) {
        if let Some(pos) = self.entities[..self.count]
            .iter()
            .position(|&e| e == entity)
        {
            // Swap with last element for O(1) removal
            self.count -= 1;
            if pos < self.count {
                self.entities[pos] = self.entities[self.count];
            }
        }
    }

    fn contains(&self, entity: EntityId) -> bool {
        self.entities[..self.count].contains(&entity)
    }

    fn iter(&self) -> &[EntityId] {
        &self.entities[..self.count]
    }
}

/// Pre-allocated spatial hash grid for broad-phase collision detection
pub struct PreallocatedSpatialHash {
    config: SpatialHashConfig,
    cells: RwLock<Vec<Cell>>,
    // Track which cells each entity occupies (up to 8 cells per entity)
    entity_cells: RwLock<Vec<[Option<CellCoord>; 8]>>,
    grid_dimensions: [u16; 3],
}

impl PreallocatedSpatialHash {
    pub fn new(config: SpatialHashConfig) -> Self {
        let grid_dimensions = [
            ((config.world_max[0] - config.world_min[0]) / config.cell_size).ceil() as u16,
            ((config.world_max[1] - config.world_min[1]) / config.cell_size).ceil() as u16,
            ((config.world_max[2] - config.world_min[2]) / config.cell_size).ceil() as u16,
        ];

        // Ensure we don't exceed maximum dimensions
        assert!(grid_dimensions[0] as usize <= MAX_CELLS_PER_DIM);
        assert!(grid_dimensions[1] as usize <= MAX_CELLS_PER_DIM);
        assert!(grid_dimensions[2] as usize <= MAX_CELLS_PER_DIM);

        let total_cells =
            grid_dimensions[0] as usize * grid_dimensions[1] as usize * grid_dimensions[2] as usize;

        let mut cells = Vec::with_capacity(total_cells);
        for _ in 0..total_cells {
            cells.push(Cell::new(config.expected_entities_per_cell));
        }

        Self {
            config,
            cells: RwLock::new(cells),
            entity_cells: RwLock::new(vec![[None; 8]; MAX_ENTITIES]),
            grid_dimensions,
        }
    }

    /// Clear all spatial hash data
    pub fn clear(&self) {
        let mut cells = self.cells.write();
        for cell in cells.iter_mut() {
            cell.clear();
        }

        let mut entity_cells = self.entity_cells.write();
        for entity_cell in entity_cells.iter_mut() {
            *entity_cell = [None; 8];
        }
    }

    /// Convert world position to cell coordinate
    fn world_to_cell(&self, pos: [f32; 3]) -> Option<CellCoord> {
        let x = ((pos[0] - self.config.world_min[0]) / self.config.cell_size).floor() as i32;
        let y = ((pos[1] - self.config.world_min[1]) / self.config.cell_size).floor() as i32;
        let z = ((pos[2] - self.config.world_min[2]) / self.config.cell_size).floor() as i32;

        if x >= 0
            && x < self.grid_dimensions[0] as i32
            && y >= 0
            && y < self.grid_dimensions[1] as i32
            && z >= 0
            && z < self.grid_dimensions[2] as i32
        {
            Some(CellCoord::new(x as u16, y as u16, z as u16))
        } else {
            None
        }
    }

    /// Get all cell coordinates that an AABB overlaps (up to 8 cells)
    fn get_overlapping_cells(&self, aabb: &AABB) -> [Option<CellCoord>; 8] {
        let mut result = [None; 8];
        let mut count = 0;

        if let (Some(min_cell), Some(max_cell)) =
            (self.world_to_cell(aabb.min), self.world_to_cell(aabb.max))
        {
            for x in min_cell.x..=max_cell.x.min(min_cell.x + 1) {
                for y in min_cell.y..=max_cell.y.min(min_cell.y + 1) {
                    for z in min_cell.z..=max_cell.z.min(min_cell.z + 1) {
                        if count < 8 {
                            result[count] = Some(CellCoord::new(x, y, z));
                            count += 1;
                        }
                    }
                }
            }
        }

        result
    }

    /// Insert an entity into the spatial hash
    pub fn insert(&self, entity: EntityId, aabb: &AABB) {
        if !entity.is_valid() {
            return;
        }

        let cells_to_insert = self.get_overlapping_cells(aabb);

        // Update cells
        {
            let mut cells = self.cells.write();
            for &cell_coord in cells_to_insert.iter().flatten() {
                let index = cell_coord.to_index();
                if index < cells.len() {
                    cells[index].add(entity);
                }
            }
        }

        // Track which cells this entity is in
        {
            let mut entity_cells = self.entity_cells.write();
            if entity.index() < entity_cells.len() {
                entity_cells[entity.index()] = cells_to_insert;
            }
        }
    }

    /// Remove an entity from the spatial hash
    pub fn remove(&self, entity: EntityId) {
        if !entity.is_valid() {
            return;
        }

        // Get cells this entity was in
        let cells_to_remove = {
            let mut entity_cells = self.entity_cells.write();
            if entity.index() < entity_cells.len() {
                let cells = entity_cells[entity.index()];
                entity_cells[entity.index()] = [None; 8];
                cells
            } else {
                return;
            }
        };

        // Remove from cells
        let mut cells = self.cells.write();
        for &cell_coord in cells_to_remove.iter().flatten() {
            let index = cell_coord.to_index();
            if index < cells.len() {
                cells[index].remove(entity);
            }
        }
    }

    /// Update an entity's position in the spatial hash
    pub fn update(&self, entity: EntityId, aabb: &AABB) {
        if !entity.is_valid() {
            return;
        }

        let new_cells = self.get_overlapping_cells(aabb);

        // Get current cells
        let old_cells = {
            let entity_cells = self.entity_cells.read();
            if entity.index() < entity_cells.len() {
                entity_cells[entity.index()]
            } else {
                return;
            }
        };

        // Quick check: if cells haven't changed, nothing to do
        if old_cells == new_cells {
            return;
        }

        let mut cells = self.cells.write();

        // Remove from old cells that are no longer occupied
        for &old_cell in old_cells.iter().flatten() {
            if !new_cells.iter().any(|&c| c == Some(old_cell)) {
                let index = old_cell.to_index();
                if index < cells.len() {
                    cells[index].remove(entity);
                }
            }
        }

        // Add to new cells
        for &new_cell in new_cells.iter().flatten() {
            if !old_cells.iter().any(|&c| c == Some(new_cell)) {
                let index = new_cell.to_index();
                if index < cells.len() {
                    cells[index].add(entity);
                }
            }
        }

        // Update entity's cell list
        drop(cells);
        let mut entity_cells = self.entity_cells.write();
        if entity.index() < entity_cells.len() {
            entity_cells[entity.index()] = new_cells;
        }
    }

    /// Query entities in a region
    pub fn query_region(&self, aabb: &AABB) -> Vec<EntityId> {
        let cells_to_check = self.get_overlapping_cells(aabb);
        let mut entities = Vec::new();
        let mut seen = [false; MAX_ENTITIES];

        let cells = self.cells.read();
        for &cell_coord in cells_to_check.iter().flatten() {
            let index = cell_coord.to_index();
            if index < cells.len() {
                for &entity in cells[index].iter() {
                    if entity.is_valid() && entity.index() < MAX_ENTITIES {
                        if !seen[entity.index()] {
                            seen[entity.index()] = true;
                            entities.push(entity);
                        }
                    }
                }
            }
        }

        entities
    }

    /// Get potential collision pairs for a single entity
    pub fn get_potential_collisions(&self, entity: EntityId) -> Vec<EntityId> {
        if !entity.is_valid() {
            return Vec::new();
        }

        let entity_cells_data = self.entity_cells.read();
        let cells_to_check = if entity.index() < entity_cells_data.len() {
            entity_cells_data[entity.index()]
        } else {
            return Vec::new();
        };
        drop(entity_cells_data);

        let mut potential = Vec::new();
        let mut seen = [false; MAX_ENTITIES];

        let cells = self.cells.read();
        for &cell_coord in cells_to_check.iter().flatten() {
            let index = cell_coord.to_index();
            if index < cells.len() {
                for &other in cells[index].iter() {
                    if other != entity && other.is_valid() && other.index() < MAX_ENTITIES {
                        if !seen[other.index()] {
                            seen[other.index()] = true;
                            potential.push(other);
                        }
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
        let cell_pairs: Vec<_> = (0..cells.len())
            .into_par_iter()
            .flat_map(|cell_idx| {
                let mut pairs = Vec::new();
                let entities = cells[cell_idx].iter();
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

        // Remove duplicates by sorting
        let mut unique_pairs = cell_pairs;
        unique_pairs.par_sort_unstable();
        unique_pairs.dedup();

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
        let mut non_empty_cells = 0;

        for cell in cells.iter() {
            let count = cell.count;
            if count > 0 {
                non_empty_cells += 1;
                max_entities_per_cell = max_entities_per_cell.max(count);
                total_entities_in_cells += count;
            }
        }

        let active_entities = entity_cells
            .iter()
            .filter(|ec| ec.iter().any(|c| c.is_some()))
            .count();

        SpatialHashStats {
            total_cells: cells.len(),
            non_empty_cells,
            total_entities: active_entities,
            max_entities_per_cell,
            avg_entities_per_cell: if non_empty_cells > 0 {
                total_entities_in_cells as f32 / non_empty_cells as f32
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug)]
pub struct SpatialHashStats {
    pub total_cells: usize,
    pub non_empty_cells: usize,
    pub total_entities: usize,
    pub max_entities_per_cell: usize,
    pub avg_entities_per_cell: f32,
}
