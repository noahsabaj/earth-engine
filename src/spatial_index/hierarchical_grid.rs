use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use std::sync::Arc;

/// Unique identifier for a cell in the hierarchical grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellId {
    level: u32,
    x: i32,
    y: i32,
    z: i32,
}

impl CellId {
    pub fn new(level: u32, x: i32, y: i32, z: i32) -> Self {
        Self { level, x, y, z }
    }
    
    /// Get parent cell ID (one level up)
    pub fn parent(&self) -> Option<CellId> {
        if self.level == 0 {
            None
        } else {
            Some(CellId {
                level: self.level - 1,
                x: self.x >> 1,
                y: self.y >> 1,
                z: self.z >> 1,
            })
        }
    }
    
    /// Get child cell IDs (one level down)
    pub fn children(&self) -> [CellId; 8] {
        let level = self.level + 1;
        let base_x = self.x << 1;
        let base_y = self.y << 1;
        let base_z = self.z << 1;
        
        [
            CellId::new(level, base_x, base_y, base_z),
            CellId::new(level, base_x + 1, base_y, base_z),
            CellId::new(level, base_x, base_y + 1, base_z),
            CellId::new(level, base_x + 1, base_y + 1, base_z),
            CellId::new(level, base_x, base_y, base_z + 1),
            CellId::new(level, base_x + 1, base_y, base_z + 1),
            CellId::new(level, base_x, base_y + 1, base_z + 1),
            CellId::new(level, base_x + 1, base_y + 1, base_z + 1),
        ]
    }
}

/// A level in the hierarchical grid
pub struct GridLevel {
    level: u32,
    cell_size: f32,
    cells: RwLock<HashMap<CellId, Cell>>,
}

impl GridLevel {
    fn new(level: u32, cell_size: f32) -> Self {
        Self {
            level,
            cell_size,
            cells: RwLock::new(HashMap::new()),
        }
    }
    
    fn world_to_cell(&self, position: [f32; 3]) -> CellId {
        let x = (position[0] / self.cell_size).floor() as i32;
        let y = (position[1] / self.cell_size).floor() as i32;
        let z = (position[2] / self.cell_size).floor() as i32;
        CellId::new(self.level, x, y, z)
    }
    
    fn cell_bounds(&self, cell_id: CellId) -> ([f32; 3], [f32; 3]) {
        let min = [
            cell_id.x as f32 * self.cell_size,
            cell_id.y as f32 * self.cell_size,
            cell_id.z as f32 * self.cell_size,
        ];
        let max = [
            min[0] + self.cell_size,
            min[1] + self.cell_size,
            min[2] + self.cell_size,
        ];
        (min, max)
    }
}

/// A cell in the grid containing entities
struct Cell {
    entities: HashSet<u64>,
    is_subdivided: bool,
    total_weight: f32, // For load balancing
}

impl Cell {
    fn new() -> Self {
        Self {
            entities: HashSet::new(),
            is_subdivided: false,
            total_weight: 0.0,
        }
    }
}

/// Hierarchical grid for spatial indexing
pub struct HierarchicalGrid {
    levels: Vec<Arc<GridLevel>>,
    world_min: [f32; 3],
    world_max: [f32; 3],
    entity_levels: RwLock<HashMap<u64, u32>>, // Which level each entity is stored at
}

impl HierarchicalGrid {
    pub fn new(
        num_levels: u32,
        base_cell_size: f32,
        world_min: [f32; 3],
        world_max: [f32; 3],
    ) -> Self {
        let mut levels = Vec::with_capacity(num_levels as usize);
        
        // Create levels from coarsest to finest
        for level in 0..num_levels {
            let cell_size = base_cell_size * (1 << (num_levels - level - 1)) as f32;
            levels.push(Arc::new(GridLevel::new(level, cell_size)));
        }
        
        Self {
            levels,
            world_min,
            world_max,
            entity_levels: RwLock::new(HashMap::new()),
        }
    }
    
    /// Insert an entity into the grid
    pub fn insert(&self, entity_id: u64, position: [f32; 3], radius: f32) {
        // Determine appropriate level based on entity size
        let level = self.select_level_for_size(radius);
        
        // Get cells that the entity overlaps
        let cells = self.get_overlapping_cells(level, position, radius);
        
        // Insert into cells
        let grid_level = &self.levels[level as usize];
        let mut level_cells = grid_level.cells.write();
        
        for cell_id in cells {
            let cell = level_cells.entry(cell_id).or_insert_with(Cell::new);
            cell.entities.insert(entity_id);
            cell.total_weight += 1.0; // Simple weight for now
        }
        
        // Track which level the entity is in
        self.entity_levels.write().insert(entity_id, level);
    }
    
    /// Remove an entity from the grid
    pub fn remove(&self, entity_id: u64) {
        // Find which level the entity is in
        let level = match self.entity_levels.write().remove(&entity_id) {
            Some(level) => level,
            None => return,
        };
        
        // Remove from all cells at that level
        let grid_level = &self.levels[level as usize];
        let mut level_cells = grid_level.cells.write();
        
        let mut empty_cells = Vec::new();
        for (cell_id, cell) in level_cells.iter_mut() {
            if cell.entities.remove(&entity_id) {
                cell.total_weight -= 1.0;
                if cell.entities.is_empty() {
                    empty_cells.push(*cell_id);
                }
            }
        }
        
        // Remove empty cells
        for cell_id in empty_cells {
            level_cells.remove(&cell_id);
        }
    }
    
    /// Update an entity's position
    pub fn update(
        &self,
        entity_id: u64,
        old_position: [f32; 3],
        new_position: [f32; 3],
        radius: f32,
    ) {
        // Get current level
        let level = match self.entity_levels.read().get(&entity_id) {
            Some(&level) => level,
            None => return,
        };
        
        let grid_level = &self.levels[level as usize];
        
        // Get old and new cells
        let old_cells = self.get_overlapping_cells(level, old_position, radius);
        let new_cells = self.get_overlapping_cells(level, new_position, radius);
        
        // Quick check: if cells haven't changed, nothing to do
        if old_cells == new_cells {
            return;
        }
        
        let mut level_cells = grid_level.cells.write();
        
        // Remove from old cells not in new cells
        for cell_id in &old_cells {
            if !new_cells.contains(cell_id) {
                if let Some(cell) = level_cells.get_mut(cell_id) {
                    cell.entities.remove(&entity_id);
                    cell.total_weight -= 1.0;
                }
            }
        }
        
        // Add to new cells not in old cells
        for cell_id in &new_cells {
            if !old_cells.contains(cell_id) {
                let cell = level_cells.entry(*cell_id).or_insert_with(Cell::new);
                cell.entities.insert(entity_id);
                cell.total_weight += 1.0;
            }
        }
    }
    
    /// Query entities in a range
    pub fn query_range(&self, center: [f32; 3], radius: f32) -> Vec<u64> {
        let mut results = HashSet::new();
        
        // Query all levels
        for (level_idx, grid_level) in self.levels.iter().enumerate() {
            let cells = self.get_overlapping_cells_sphere(level_idx as u32, center, radius);
            
            let level_cells = grid_level.cells.read();
            for cell_id in cells {
                if let Some(cell) = level_cells.get(&cell_id) {
                    // Add all entities from this cell
                    for &entity_id in &cell.entities {
                        results.insert(entity_id);
                    }
                }
            }
        }
        
        results.into_iter().collect()
    }
    
    /// Query entities in a box
    pub fn query_box(&self, min: [f32; 3], max: [f32; 3]) -> Vec<u64> {
        let mut results = HashSet::new();
        
        // Query all levels
        for (level_idx, grid_level) in self.levels.iter().enumerate() {
            let cells = self.get_overlapping_cells_box(level_idx as u32, min, max);
            
            let level_cells = grid_level.cells.read();
            for cell_id in cells {
                if let Some(cell) = level_cells.get(&cell_id) {
                    for &entity_id in &cell.entities {
                        results.insert(entity_id);
                    }
                }
            }
        }
        
        results.into_iter().collect()
    }
    
    /// Split a cell into its children
    pub fn split_cell(&self, cell_id: CellId) {
        if cell_id.level >= self.levels.len() as u32 - 1 {
            return; // Can't split finest level
        }
        
        let grid_level = &self.levels[cell_id.level as usize];
        let mut level_cells = grid_level.cells.write();
        
        if let Some(cell) = level_cells.get_mut(&cell_id) {
            cell.is_subdivided = true;
            // Entities remain in parent cell but are also indexed in children
        }
    }
    
    /// Merge cells back to parent
    pub fn merge_cells(&self, child_ids: Vec<CellId>) {
        if child_ids.is_empty() || child_ids[0].level == 0 {
            return;
        }
        
        // Get parent
        let parent_id = child_ids[0].parent().unwrap();
        
        let parent_level = &self.levels[parent_id.level as usize];
        let mut parent_cells = parent_level.cells.write();
        
        if let Some(parent_cell) = parent_cells.get_mut(&parent_id) {
            parent_cell.is_subdivided = false;
        }
    }
    
    /// Plan rebalancing operations based on density
    pub fn plan_rebalance(
        &self,
        density_map: &super::DensityMap,
        config: &super::SpatialIndexConfig,
    ) -> Vec<RebalanceOp> {
        let mut operations = Vec::new();
        
        // Check each cell at each level
        for (level_idx, grid_level) in self.levels.iter().enumerate() {
            let level_cells = grid_level.cells.read();
            
            for (cell_id, cell) in level_cells.iter() {
                let cell_density = density_map.get_density(*cell_id);
                
                // Should split?
                if !cell.is_subdivided && 
                   cell.entities.len() > config.max_entities_per_cell &&
                   level_idx < self.levels.len() - 1 {
                    operations.push(RebalanceOp::Split(*cell_id));
                }
                
                // Should merge?
                if cell.is_subdivided &&
                   cell_density < config.min_entities_per_cell as f32 {
                    // Check if all children have low density
                    let children = cell_id.children();
                    let should_merge = children.iter().all(|child| {
                        density_map.get_density(*child) < config.min_entities_per_cell as f32
                    });
                    
                    if should_merge {
                        operations.push(RebalanceOp::Merge(children.to_vec()));
                    }
                }
            }
        }
        
        operations
    }
    
    /// Get statistics about the grid
    pub fn stats(&self) -> GridStats {
        let mut total_cells = 0;
        let mut total_entities = 0;
        let mut max_entities_per_cell = 0;
        let mut cells_by_level = Vec::new();
        
        for grid_level in &self.levels {
            let level_cells = grid_level.cells.read();
            let level_cell_count = level_cells.len();
            total_cells += level_cell_count;
            
            let mut level_max = 0;
            for cell in level_cells.values() {
                let count = cell.entities.len();
                total_entities += count;
                level_max = level_max.max(count);
                max_entities_per_cell = max_entities_per_cell.max(count);
            }
            
            cells_by_level.push(LevelStats {
                level: grid_level.level,
                cell_count: level_cell_count,
                max_entities: level_max,
            });
        }
        
        GridStats {
            total_cells,
            total_entities,
            max_entities_per_cell,
            cells_by_level,
        }
    }
    
    // Helper methods
    
    fn select_level_for_size(&self, radius: f32) -> u32 {
        // Select the finest level where entity fits comfortably in a cell
        for (idx, level) in self.levels.iter().enumerate().rev() {
            if radius * 2.0 <= level.cell_size * 0.5 {
                return idx as u32;
            }
        }
        0 // Use coarsest level for very large entities
    }
    
    fn get_overlapping_cells(&self, level: u32, position: [f32; 3], radius: f32) -> Vec<CellId> {
        let grid_level = &self.levels[level as usize];
        let min = [position[0] - radius, position[1] - radius, position[2] - radius];
        let max = [position[0] + radius, position[1] + radius, position[2] + radius];
        
        let min_cell = grid_level.world_to_cell(min);
        let max_cell = grid_level.world_to_cell(max);
        
        let mut cells = Vec::new();
        for x in min_cell.x..=max_cell.x {
            for y in min_cell.y..=max_cell.y {
                for z in min_cell.z..=max_cell.z {
                    cells.push(CellId::new(level, x, y, z));
                }
            }
        }
        
        cells
    }
    
    fn get_overlapping_cells_sphere(&self, level: u32, center: [f32; 3], radius: f32) -> Vec<CellId> {
        // Start with box approximation
        let cells = self.get_overlapping_cells(level, center, radius);
        
        // Filter to actual sphere overlap
        let grid_level = &self.levels[level as usize];
        cells.into_iter().filter(|&cell_id| {
            let (cell_min, cell_max) = grid_level.cell_bounds(cell_id);
            sphere_aabb_overlap(center, radius, cell_min, cell_max)
        }).collect()
    }
    
    fn get_overlapping_cells_box(&self, level: u32, min: [f32; 3], max: [f32; 3]) -> Vec<CellId> {
        let grid_level = &self.levels[level as usize];
        let min_cell = grid_level.world_to_cell(min);
        let max_cell = grid_level.world_to_cell(max);
        
        let mut cells = Vec::new();
        for x in min_cell.x..=max_cell.x {
            for y in min_cell.y..=max_cell.y {
                for z in min_cell.z..=max_cell.z {
                    cells.push(CellId::new(level, x, y, z));
                }
            }
        }
        
        cells
    }
}

/// Rebalancing operation
#[derive(Debug)]
pub enum RebalanceOp {
    Split(CellId),
    Merge(Vec<CellId>),
}

#[derive(Debug)]
pub struct GridStats {
    pub total_cells: usize,
    pub total_entities: usize,
    pub max_entities_per_cell: usize,
    pub cells_by_level: Vec<LevelStats>,
}

#[derive(Debug)]
pub struct LevelStats {
    pub level: u32,
    pub cell_count: usize,
    pub max_entities: usize,
}

fn sphere_aabb_overlap(center: [f32; 3], radius: f32, aabb_min: [f32; 3], aabb_max: [f32; 3]) -> bool {
    let mut closest = [0.0; 3];
    for i in 0..3 {
        closest[i] = center[i].max(aabb_min[i]).min(aabb_max[i]);
    }
    
    let dx = closest[0] - center[0];
    let dy = closest[1] - center[1];
    let dz = closest[2] - center[2];
    
    (dx * dx + dy * dy + dz * dz) <= radius * radius
}