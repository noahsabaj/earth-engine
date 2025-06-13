use std::collections::HashMap;
use parking_lot::RwLock;
use super::{CellId, HierarchicalGrid, SpatialIndexConfig};

/// Tracks entity density across the spatial grid
pub struct DensityAnalyzer {
    /// Moving average of density per cell
    cell_density: RwLock<HashMap<CellId, CellDensity>>,
    
    /// Configuration
    config: SpatialIndexConfig,
    
    /// Global density statistics
    global_stats: RwLock<DensityStats>,
}

/// Density information for a cell
#[derive(Debug, Clone)]
struct CellDensity {
    current_count: usize,
    average_count: f32,
    peak_count: usize,
    sample_count: u32,
    last_update: std::time::Instant,
}

impl CellDensity {
    fn new() -> Self {
        Self {
            current_count: 0,
            average_count: 0.0,
            peak_count: 0,
            sample_count: 0,
            last_update: std::time::Instant::now(),
        }
    }
    
    fn update(&mut self, count: usize) {
        self.current_count = count;
        self.peak_count = self.peak_count.max(count);
        
        // Update moving average
        self.sample_count += 1;
        let weight = 1.0 / self.sample_count as f32;
        self.average_count = self.average_count * (1.0 - weight) + count as f32 * weight;
        
        self.last_update = std::time::Instant::now();
    }
    
    fn density_score(&self) -> f32 {
        // Combine current and average for stability
        self.current_count as f32 * 0.7 + self.average_count * 0.3
    }
}

/// Density statistics
#[derive(Debug, Clone, Default)]
pub struct DensityStats {
    pub total_cells: usize,
    pub occupied_cells: usize,
    pub average_density: f32,
    pub max_density: usize,
    pub hotspot_cells: Vec<CellId>,
}

/// A map of cell densities for rebalancing decisions
pub struct DensityMap {
    densities: HashMap<CellId, f32>,
}

impl DensityMap {
    pub fn get_density(&self, cell_id: CellId) -> f32 {
        self.densities.get(&cell_id).copied().unwrap_or(0.0)
    }
}

impl DensityAnalyzer {
    pub fn new(config: &SpatialIndexConfig) -> Self {
        Self {
            cell_density: RwLock::new(HashMap::new()),
            config: config.clone(),
            global_stats: RwLock::new(DensityStats::default()),
        }
    }
    
    /// Record an entity insertion at a position
    pub fn record_insertion(&self, position: [f32; 3]) {
        // In a real implementation, we'd track which cells are affected
        let _ = position; // Suppress warning
        
        // Update global stats
        let mut stats = self.global_stats.write();
        stats.total_cells = self.cell_density.read().len();
    }
    
    /// Record an entity removal at a position
    pub fn record_removal(&self, position: [f32; 3]) {
        let _ = position; // Suppress warning
        
        // Update global stats
        let mut stats = self.global_stats.write();
        stats.total_cells = self.cell_density.read().len();
    }
    
    /// Record entity movement
    pub fn record_movement(&self, old_position: [f32; 3], new_position: [f32; 3]) {
        // Track movement patterns for predictive loading
        let _ = (old_position, new_position); // Suppress warning
    }
    
    /// Analyze the current density distribution
    pub fn analyze(&self, grid: &HierarchicalGrid) -> DensityMap {
        let mut densities = HashMap::new();
        let mut total_density = 0.0;
        let mut max_density = 0;
        let mut hotspots = Vec::new();
        
        // Get grid statistics
        let grid_stats = grid.stats();
        
        // Update cell densities
        let cell_density = self.cell_density.write();
        
        for level_stats in &grid_stats.cells_by_level {
            // For each level, update density information
            // In a real implementation, we'd iterate through actual cells
            let level_density = level_stats.max_entities;
            
            if level_density > self.config.max_entities_per_cell {
                // This is a hotspot that needs attention
                // In real implementation, we'd get the actual cell ID
                let hotspot_id = CellId::new(level_stats.level, 0, 0, 0);
                hotspots.push(hotspot_id);
            }
            
            total_density += level_density as f32;
            max_density = max_density.max(level_density);
        }
        
        // Update global statistics
        let mut stats = self.global_stats.write();
        stats.occupied_cells = grid_stats.total_cells;
        stats.average_density = if grid_stats.total_cells > 0 {
            total_density / grid_stats.total_cells as f32
        } else {
            0.0
        };
        stats.max_density = max_density;
        stats.hotspot_cells = hotspots;
        
        // Build density map for rebalancing
        for (cell_id, density) in cell_density.iter() {
            densities.insert(*cell_id, density.density_score());
        }
        
        DensityMap { densities }
    }
    
    /// Get current density statistics
    pub fn stats(&self) -> DensityStats {
        self.global_stats.read().clone()
    }
    
    /// Predict future density based on movement patterns
    pub fn predict_density(&self, cell_id: CellId, time_ahead: f32) -> f32 {
        let cell_density = self.cell_density.read();
        
        if let Some(density) = cell_density.get(&cell_id) {
            // Simple prediction based on current trend
            // In a real implementation, we'd track velocity of density changes
            let trend = (density.current_count as f32 - density.average_count) * 0.1;
            (density.density_score() + trend * time_ahead).max(0.0)
        } else {
            0.0
        }
    }
    
    /// Identify cells that should be split due to high density
    pub fn cells_to_split(&self, threshold: f32) -> Vec<CellId> {
        let cell_density = self.cell_density.read();
        
        cell_density
            .iter()
            .filter(|(_, density)| density.density_score() > threshold)
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Identify cells that should be merged due to low density
    pub fn cells_to_merge(&self, threshold: f32) -> Vec<CellId> {
        let cell_density = self.cell_density.read();
        
        cell_density
            .iter()
            .filter(|(_, density)| density.density_score() < threshold)
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Clear old density data
    pub fn cleanup_stale_data(&self, max_age: std::time::Duration) {
        let mut cell_density = self.cell_density.write();
        let now = std::time::Instant::now();
        
        cell_density.retain(|_, density| {
            now.duration_since(density.last_update) < max_age
        });
    }
}