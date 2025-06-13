#![allow(unused_variables, dead_code, unused_imports)]
/// Pre-allocated hierarchical grid using fixed-size arrays
/// Replaces HashMap-based spatial indexing with zero-allocation lookups

use parking_lot::RwLock;

/// Maximum levels in the hierarchy
pub const MAX_LEVELS: usize = 8;

/// Maximum grid size per dimension at the finest level
pub const MAX_GRID_SIZE: usize = 256; // 256^3 cells at finest level

/// Maximum entities that can be tracked
pub const MAX_ENTITIES: usize = 65536;

/// Unique identifier for a cell in the hierarchical grid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellId {
    level: u8,
    x: u16,
    y: u16,
    z: u16,
}

impl CellId {
    pub fn new(level: u8, x: u16, y: u16, z: u16) -> Self {
        Self { level, x, y, z }
    }
    
    /// Convert to linear index for the level
    pub fn to_index(&self) -> usize {
        let level_size = MAX_GRID_SIZE >> self.level;
        let x = self.x as usize;
        let y = self.y as usize;
        let z = self.z as usize;
        x + y * level_size + z * level_size * level_size
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

/// Cell data at a specific level
struct GridCell {
    /// Entities in this cell
    entities: Vec<u32>,
    /// Number of active entities
    entity_count: usize,
    /// Aggregate bounds
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    /// Is this cell active?
    active: bool,
}

impl GridCell {
    fn new(capacity: usize) -> Self {
        Self {
            entities: Vec::with_capacity(capacity),
            entity_count: 0,
            bounds_min: [f32::MAX; 3],
            bounds_max: [f32::MIN; 3],
            active: false,
        }
    }
    
    fn clear(&mut self) {
        self.entity_count = 0;
        self.entities.clear();
        self.bounds_min = [f32::MAX; 3];
        self.bounds_max = [f32::MIN; 3];
        self.active = false;
    }
    
    fn add_entity(&mut self, entity_id: u32, min: [f32; 3], max: [f32; 3]) {
        if self.entity_count < self.entities.capacity() {
            if self.entity_count < self.entities.len() {
                self.entities[self.entity_count] = entity_id;
            } else {
                self.entities.push(entity_id);
            }
            self.entity_count += 1;
            
            // Update bounds
            for i in 0..3 {
                self.bounds_min[i] = self.bounds_min[i].min(min[i]);
                self.bounds_max[i] = self.bounds_max[i].max(max[i]);
            }
            
            self.active = true;
        }
    }
    
    fn remove_entity(&mut self, entity_id: u32) -> bool {
        if let Some(pos) = self.entities[..self.entity_count].iter().position(|&e| e == entity_id) {
            self.entity_count -= 1;
            if pos < self.entity_count {
                self.entities[pos] = self.entities[self.entity_count];
            }
            
            if self.entity_count == 0 {
                self.clear();
            }
            
            true
        } else {
            false
        }
    }
}

/// Grid level containing all cells at that level
struct GridLevel {
    cells: Vec<GridCell>,
    size: usize,
    cell_size: f32,
}

impl GridLevel {
    fn new(level: u8, world_size: f32, entities_per_cell: usize) -> Self {
        let size = MAX_GRID_SIZE >> level;
        let total_cells = size * size * size;
        let cell_size = world_size / size as f32;
        
        let mut cells = Vec::with_capacity(total_cells);
        for _ in 0..total_cells {
            cells.push(GridCell::new(entities_per_cell));
        }
        
        Self {
            cells,
            size,
            cell_size,
        }
    }
    
    fn world_to_cell(&self, pos: [f32; 3]) -> Option<(u16, u16, u16)> {
        let x = (pos[0] / self.cell_size).floor() as i32;
        let y = (pos[1] / self.cell_size).floor() as i32;
        let z = (pos[2] / self.cell_size).floor() as i32;
        
        if x >= 0 && x < self.size as i32 &&
           y >= 0 && y < self.size as i32 &&
           z >= 0 && z < self.size as i32 {
            Some((x as u16, y as u16, z as u16))
        } else {
            None
        }
    }
    
    fn get_cell(&self, x: u16, y: u16, z: u16) -> Option<&GridCell> {
        let index = x as usize + 
                   y as usize * self.size + 
                   z as usize * self.size * self.size;
        self.cells.get(index)
    }
    
    fn get_cell_mut(&mut self, x: u16, y: u16, z: u16) -> Option<&mut GridCell> {
        let index = x as usize + 
                   y as usize * self.size + 
                   z as usize * self.size * self.size;
        self.cells.get_mut(index)
    }
}

/// Entity tracking data
struct EntityData {
    /// Current cell at each level
    cells: [Option<CellId>; MAX_LEVELS],
    /// Entity bounds
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    /// Is entity active?
    active: bool,
}

/// Pre-allocated hierarchical grid
pub struct PreallocatedHierarchicalGrid {
    /// Grid levels
    levels: RwLock<Vec<GridLevel>>,
    /// Entity tracking
    entities: RwLock<Vec<EntityData>>,
    /// Configuration
    world_size: f32,
    entities_per_cell: usize,
    /// Active entity count
    active_entities: RwLock<usize>,
}

impl PreallocatedHierarchicalGrid {
    pub fn new(world_size: f32, num_levels: usize, entities_per_cell: usize) -> Self {
        let num_levels = num_levels.min(MAX_LEVELS);
        
        let mut levels = Vec::with_capacity(num_levels);
        for level in 0..num_levels {
            levels.push(GridLevel::new(level as u8, world_size, entities_per_cell));
        }
        
        let mut entities = Vec::with_capacity(MAX_ENTITIES);
        for _ in 0..MAX_ENTITIES {
            entities.push(EntityData {
                cells: [None; MAX_LEVELS],
                bounds_min: [0.0; 3],
                bounds_max: [0.0; 3],
                active: false,
            });
        }
        
        Self {
            levels: RwLock::new(levels),
            entities: RwLock::new(entities),
            world_size,
            entities_per_cell,
            active_entities: RwLock::new(0),
        }
    }
    
    /// Insert or update an entity
    pub fn insert(&self, entity_id: u32, min: [f32; 3], max: [f32; 3]) {
        if entity_id as usize >= MAX_ENTITIES {
            return;
        }
        
        let mut entities = self.entities.write();
        let mut levels = self.levels.write();
        
        let entity_data = &mut entities[entity_id as usize];
        
        // Remove from old cells if updating
        if entity_data.active {
            for (level_idx, cell_id) in entity_data.cells.iter().enumerate() {
                if let Some(cell_id) = cell_id {
                    if let Some(cell) = levels[level_idx].get_cell_mut(cell_id.x, cell_id.y, cell_id.z) {
                        cell.remove_entity(entity_id);
                    }
                }
            }
        } else {
            // New entity
            let mut count = self.active_entities.write();
            *count += 1;
        }
        
        // Update entity data
        entity_data.bounds_min = min;
        entity_data.bounds_max = max;
        entity_data.active = true;
        
        // Insert into appropriate cells at each level
        for (level_idx, level) in levels.iter_mut().enumerate() {
            // Get cell coordinates for entity center
            let center = [
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (min[2] + max[2]) * 0.5,
            ];
            
            if let Some((x, y, z)) = level.world_to_cell(center) {
                let cell_id = CellId::new(level_idx as u8, x, y, z);
                entity_data.cells[level_idx] = Some(cell_id);
                
                if let Some(cell) = level.get_cell_mut(x, y, z) {
                    cell.add_entity(entity_id, min, max);
                }
            } else {
                entity_data.cells[level_idx] = None;
            }
        }
    }
    
    /// Remove an entity
    pub fn remove(&self, entity_id: u32) {
        if entity_id as usize >= MAX_ENTITIES {
            return;
        }
        
        let mut entities = self.entities.write();
        let mut levels = self.levels.write();
        
        let entity_data = &mut entities[entity_id as usize];
        
        if !entity_data.active {
            return;
        }
        
        // Remove from all cells
        for (level_idx, cell_id) in entity_data.cells.iter().enumerate() {
            if let Some(cell_id) = cell_id {
                if let Some(cell) = levels[level_idx].get_cell_mut(cell_id.x, cell_id.y, cell_id.z) {
                    cell.remove_entity(entity_id);
                }
            }
        }
        
        // Clear entity data
        entity_data.active = false;
        entity_data.cells = [None; MAX_LEVELS];
        
        let mut count = self.active_entities.write();
        *count -= 1;
    }
    
    /// Query entities in a region at a specific level
    pub fn query_region(&self, level: usize, min: [f32; 3], max: [f32; 3]) -> Vec<u32> {
        if level >= MAX_LEVELS {
            return Vec::new();
        }
        
        let levels = self.levels.read();
        if level >= levels.len() {
            return Vec::new();
        }
        
        let grid_level = &levels[level];
        let mut results = Vec::new();
        let mut seen = vec![false; MAX_ENTITIES];
        
        // Get cell range
        let min_cell = grid_level.world_to_cell(min).unwrap_or((0, 0, 0));
        let max_cell = grid_level.world_to_cell(max)
            .unwrap_or((grid_level.size as u16 - 1, grid_level.size as u16 - 1, grid_level.size as u16 - 1));
        
        // Iterate through cells
        for z in min_cell.2..=max_cell.2 {
            for y in min_cell.1..=max_cell.1 {
                for x in min_cell.0..=max_cell.0 {
                    if let Some(cell) = grid_level.get_cell(x, y, z) {
                        if !cell.active {
                            continue;
                        }
                        
                        // Check if cell bounds overlap query region
                        if cell.bounds_max[0] < min[0] || cell.bounds_min[0] > max[0] ||
                           cell.bounds_max[1] < min[1] || cell.bounds_min[1] > max[1] ||
                           cell.bounds_max[2] < min[2] || cell.bounds_min[2] > max[2] {
                            continue;
                        }
                        
                        // Add entities
                        for &entity_id in &cell.entities[..cell.entity_count] {
                            if !seen[entity_id as usize] {
                                seen[entity_id as usize] = true;
                                results.push(entity_id);
                            }
                        }
                    }
                }
            }
        }
        
        results
    }
    
    /// Get statistics
    pub fn stats(&self) -> HierarchicalGridStats {
        let entities = self.entities.read();
        let levels = self.levels.read();
        let active_entities = *self.active_entities.read();
        
        let mut level_stats = Vec::new();
        for (idx, level) in levels.iter().enumerate() {
            let active_cells = level.cells.iter().filter(|c| c.active).count();
            let total_entities: usize = level.cells.iter()
                .map(|c| c.entity_count)
                .sum();
            
            level_stats.push(LevelStats {
                level: idx,
                active_cells,
                total_cells: level.cells.len(),
                entities_in_level: total_entities,
                cell_size: level.cell_size,
            });
        }
        
        HierarchicalGridStats {
            active_entities,
            max_entities: MAX_ENTITIES,
            levels: level_stats,
        }
    }
}

#[derive(Debug)]
pub struct LevelStats {
    pub level: usize,
    pub active_cells: usize,
    pub total_cells: usize,
    pub entities_in_level: usize,
    pub cell_size: f32,
}

#[derive(Debug)]
pub struct HierarchicalGridStats {
    pub active_entities: usize,
    pub max_entities: usize,
    pub levels: Vec<LevelStats>,
}