/// Hierarchical spatial indexing system for efficient entity queries
/// 
/// This module provides a general-purpose spatial index that supports:
/// - Hierarchical levels of detail (octree-like)
/// - Dynamic cell sizing based on entity density
/// - Various query types (range, k-nearest, frustum)
/// - Entity type filtering
/// - Load balancing for crowded areas

mod hierarchical_grid;
mod spatial_query;
mod entity_store;
mod density_analyzer;
mod query_cache;
mod parallel_query;

pub use hierarchical_grid::{HierarchicalGrid, CellId};
pub use spatial_query::{
    SpatialQuery, QueryType, QueryResult, 
    RangeQuery, KNearestQuery, FrustumQuery, BoxQuery
};
pub use entity_store::{SpatialEntity, EntityType, EntityData};
pub use density_analyzer::{DensityAnalyzer, DensityMap};
pub use query_cache::{QueryCache, CacheStats};
pub use parallel_query::{ParallelQueryExecutor, QueryBatch};

use std::sync::Arc;

/// Configuration for the spatial index
#[derive(Debug, Clone)]
pub struct SpatialIndexConfig {
    /// Base cell size at the finest level
    pub base_cell_size: f32,
    
    /// Number of hierarchy levels
    pub hierarchy_levels: u32,
    
    /// Maximum entities per cell before subdivision
    pub max_entities_per_cell: usize,
    
    /// Minimum entities to maintain cell (prevents thrashing)
    pub min_entities_per_cell: usize,
    
    /// World bounds
    pub world_min: [f32; 3],
    pub world_max: [f32; 3],
    
    /// Enable query caching
    pub enable_cache: bool,
    
    /// Cache size in MB
    pub cache_size_mb: usize,
    
    /// Number of worker threads for parallel queries
    pub query_threads: usize,
}

impl Default for SpatialIndexConfig {
    fn default() -> Self {
        Self {
            base_cell_size: 2.0,
            hierarchy_levels: 4,
            max_entities_per_cell: 32,
            min_entities_per_cell: 8,
            world_min: [-2048.0, -256.0, -2048.0],
            world_max: [2048.0, 512.0, 2048.0],
            enable_cache: true,
            cache_size_mb: 64,
            query_threads: num_cpus::get(),
        }
    }
}

/// Main spatial index structure
pub struct SpatialIndex {
    config: SpatialIndexConfig,
    grid: HierarchicalGrid,
    entity_store: entity_store::EntityStore,
    density_analyzer: DensityAnalyzer,
    query_cache: Option<QueryCache>,
    query_executor: Arc<ParallelQueryExecutor>,
}

impl SpatialIndex {
    /// Create a new spatial index
    pub fn new(config: SpatialIndexConfig) -> Self {
        let grid = HierarchicalGrid::new(
            config.hierarchy_levels,
            config.base_cell_size,
            config.world_min,
            config.world_max,
        );
        
        let entity_store = entity_store::EntityStore::new();
        let density_analyzer = DensityAnalyzer::new(&config);
        
        let query_cache = if config.enable_cache {
            Some(QueryCache::new(config.cache_size_mb))
        } else {
            None
        };
        
        let query_executor = Arc::new(ParallelQueryExecutor::new(config.query_threads));
        
        Self {
            config,
            grid,
            entity_store,
            density_analyzer,
            query_cache,
            query_executor,
        }
    }
    
    /// Insert an entity into the spatial index
    pub fn insert(&mut self, entity: SpatialEntity) -> Result<(), &'static str> {
        // Validate bounds
        if !self.is_in_bounds(entity.position()) {
            return Err("Entity position out of world bounds");
        }
        
        // Store entity data
        self.entity_store.insert(entity.id(), entity.clone());
        
        // Insert into grid
        self.grid.insert(entity.id(), entity.position(), entity.radius());
        
        // Update density information
        self.density_analyzer.record_insertion(entity.position());
        
        // Invalidate relevant cache entries
        if let Some(cache) = &mut self.query_cache {
            cache.invalidate_region(entity.position(), entity.radius());
        }
        
        Ok(())
    }
    
    /// Remove an entity from the spatial index
    pub fn remove(&mut self, entity_id: u64) -> Result<(), &'static str> {
        // Get entity data
        let entity = self.entity_store.get(entity_id)
            .ok_or("Entity not found")?;
        
        let position = entity.position();
        let radius = entity.radius();
        
        // Remove from grid
        self.grid.remove(entity_id);
        
        // Remove from store
        self.entity_store.remove(entity_id);
        
        // Update density
        self.density_analyzer.record_removal(position);
        
        // Invalidate cache
        if let Some(cache) = &mut self.query_cache {
            cache.invalidate_region(position, radius);
        }
        
        Ok(())
    }
    
    /// Update an entity's position
    pub fn update(&mut self, entity_id: u64, new_position: [f32; 3]) -> Result<(), &'static str> {
        // Validate bounds
        if !self.is_in_bounds(new_position) {
            return Err("New position out of world bounds");
        }
        
        // Get entity
        let mut entity = self.entity_store.get_mut(entity_id)
            .ok_or("Entity not found")?;
        
        let old_position = entity.position();
        let radius = entity.radius();
        
        // Update position
        entity.set_position(new_position);
        
        // Update grid
        self.grid.update(entity_id, old_position, new_position, radius);
        
        // Update density
        self.density_analyzer.record_movement(old_position, new_position);
        
        // Invalidate cache for both old and new regions
        if let Some(cache) = &mut self.query_cache {
            cache.invalidate_region(old_position, radius);
            cache.invalidate_region(new_position, radius);
        }
        
        Ok(())
    }
    
    /// Execute a spatial query
    pub fn query(&self, query: &SpatialQuery) -> Vec<QueryResult> {
        // Check cache first
        if let Some(cache) = &self.query_cache {
            if let Some(results) = cache.get(query) {
                return results;
            }
        }
        
        // Execute query
        let results = match query.query_type() {
            QueryType::Range(range_query) => {
                self.execute_range_query(range_query)
            }
            QueryType::KNearest(k_nearest) => {
                self.execute_k_nearest_query(k_nearest)
            }
            QueryType::Frustum(frustum) => {
                self.execute_frustum_query(frustum)
            }
            QueryType::Box(box_query) => {
                self.execute_box_query(box_query)
            }
        };
        
        // Cache results
        if let Some(cache) = &self.query_cache {
            cache.put(query.clone(), results.clone());
        }
        
        results
    }
    
    /// Execute multiple queries in parallel
    pub fn batch_query(&self, queries: Vec<SpatialQuery>) -> Vec<Vec<QueryResult>> {
        self.query_executor.execute_batch(queries, &self.grid, &self.entity_store)
    }
    
    /// Rebalance the spatial index based on current density
    pub fn rebalance(&mut self) {
        let density_map = self.density_analyzer.analyze(&self.grid);
        
        // Identify cells that need splitting or merging
        let operations = self.grid.plan_rebalance(&density_map, &self.config);
        
        // Apply operations
        for operation in operations {
            match operation {
                hierarchical_grid::RebalanceOp::Split(cell_id) => {
                    self.grid.split_cell(cell_id);
                }
                hierarchical_grid::RebalanceOp::Merge(cell_ids) => {
                    self.grid.merge_cells(cell_ids);
                }
            }
        }
        
        // Clear cache after rebalancing
        if let Some(cache) = &mut self.query_cache {
            cache.clear();
        }
    }
    
    /// Get statistics about the spatial index
    pub fn stats(&self) -> SpatialIndexStats {
        SpatialIndexStats {
            total_entities: self.entity_store.count(),
            grid_stats: self.grid.stats(),
            density_stats: self.density_analyzer.stats(),
            cache_stats: self.query_cache.as_ref().map(|c| c.stats()),
        }
    }
    
    // Private helper methods
    
    fn is_in_bounds(&self, position: [f32; 3]) -> bool {
        position[0] >= self.config.world_min[0] && position[0] <= self.config.world_max[0] &&
        position[1] >= self.config.world_min[1] && position[1] <= self.config.world_max[1] &&
        position[2] >= self.config.world_min[2] && position[2] <= self.config.world_max[2]
    }
    
    fn execute_range_query(&self, query: &RangeQuery) -> Vec<QueryResult> {
        let candidates = self.grid.query_range(query.center(), query.radius());
        
        let mut results = Vec::new();
        for entity_id in candidates {
            if let Some(entity) = self.entity_store.get(entity_id) {
                // Filter by entity type if specified
                if let Some(entity_type) = query.entity_type() {
                    if entity.entity_type() != entity_type {
                        continue;
                    }
                }
                
                // Calculate actual distance
                let distance = distance_3d(query.center(), entity.position());
                if distance <= query.radius() {
                    results.push(QueryResult {
                        entity_id,
                        distance: Some(distance),
                        entity_data: entity.data().clone(),
                    });
                }
            }
        }
        
        results
    }
    
    fn execute_k_nearest_query(&self, _query: &KNearestQuery) -> Vec<QueryResult> {
        // Implementation for k-nearest neighbors
        todo!("K-nearest query implementation")
    }
    
    fn execute_frustum_query(&self, _query: &FrustumQuery) -> Vec<QueryResult> {
        // Implementation for frustum culling
        todo!("Frustum query implementation")
    }
    
    fn execute_box_query(&self, query: &BoxQuery) -> Vec<QueryResult> {
        let candidates = self.grid.query_box(query.min(), query.max());
        
        let mut results = Vec::new();
        for entity_id in candidates {
            if let Some(entity) = self.entity_store.get(entity_id) {
                // Filter by entity type if specified
                if let Some(entity_type) = query.entity_type() {
                    if entity.entity_type() != entity_type {
                        continue;
                    }
                }
                
                // Check if entity is actually in box
                let pos = entity.position();
                if pos[0] >= query.min()[0] && pos[0] <= query.max()[0] &&
                   pos[1] >= query.min()[1] && pos[1] <= query.max()[1] &&
                   pos[2] >= query.min()[2] && pos[2] <= query.max()[2] {
                    results.push(QueryResult {
                        entity_id,
                        distance: None,
                        entity_data: entity.data().clone(),
                    });
                }
            }
        }
        
        results
    }
}

#[derive(Debug)]
pub struct SpatialIndexStats {
    pub total_entities: usize,
    pub grid_stats: hierarchical_grid::GridStats,
    pub density_stats: density_analyzer::DensityStats,
    pub cache_stats: Option<CacheStats>,
}

fn distance_3d(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}