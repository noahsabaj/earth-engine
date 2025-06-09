use std::sync::Arc;
use rayon::prelude::*;
use super::{SpatialQuery, QueryResult, HierarchicalGrid, EntityStore, QueryType};

/// Executes spatial queries in parallel
pub struct ParallelQueryExecutor {
    thread_pool: rayon::ThreadPool,
}

/// A batch of queries to execute together
pub struct QueryBatch {
    queries: Vec<SpatialQuery>,
    priority: QueryPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryPriority {
    High,    // Player view frustums
    Medium,  // AI visibility checks
    Low,     // Background processing
}

impl ParallelQueryExecutor {
    pub fn new(num_threads: usize) -> Self {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(|i| format!("spatial-query-{}", i))
            .build()
            .expect("Failed to create query thread pool");
            
        Self { thread_pool }
    }
    
    /// Execute a batch of queries in parallel
    pub fn execute_batch(
        &self,
        queries: Vec<SpatialQuery>,
        grid: &HierarchicalGrid,
        entity_store: &EntityStore,
    ) -> Vec<Vec<QueryResult>> {
        self.thread_pool.install(|| {
            queries
                .par_iter()
                .map(|query| self.execute_single(query, grid, entity_store))
                .collect()
        })
    }
    
    /// Execute queries with priority ordering
    pub fn execute_prioritized(
        &self,
        batches: Vec<QueryBatch>,
        grid: &HierarchicalGrid,
        entity_store: &EntityStore,
    ) -> Vec<Vec<Vec<QueryResult>>> {
        // Sort batches by priority
        let mut sorted_batches = batches;
        sorted_batches.sort_by_key(|b| match b.priority {
            QueryPriority::High => 0,
            QueryPriority::Medium => 1,
            QueryPriority::Low => 2,
        });
        
        // Execute each batch
        sorted_batches
            .into_iter()
            .map(|batch| self.execute_batch(batch.queries, grid, entity_store))
            .collect()
    }
    
    /// Execute a single query
    fn execute_single(
        &self,
        query: &SpatialQuery,
        grid: &HierarchicalGrid,
        entity_store: &EntityStore,
    ) -> Vec<QueryResult> {
        match query.query_type() {
            QueryType::Range(range_query) => {
                self.execute_range_query(range_query, grid, entity_store)
            }
            QueryType::KNearest(k_nearest) => {
                self.execute_k_nearest_query(k_nearest, grid, entity_store)
            }
            QueryType::Frustum(frustum) => {
                self.execute_frustum_query(frustum, grid, entity_store)
            }
            QueryType::Box(box_query) => {
                self.execute_box_query(box_query, grid, entity_store)
            }
        }
    }
    
    fn execute_range_query(
        &self,
        query: &super::RangeQuery,
        grid: &HierarchicalGrid,
        entity_store: &EntityStore,
    ) -> Vec<QueryResult> {
        let candidates = grid.query_range(query.center(), query.radius());
        
        // Process candidates in parallel
        candidates
            .par_iter()
            .filter_map(|&entity_id| {
                if let Some(entity) = entity_store.get(entity_id) {
                    // Filter by entity type if specified
                    if let Some(entity_type) = query.entity_type() {
                        if entity.entity_type() != entity_type {
                            return None;
                        }
                    }
                    
                    // Calculate actual distance
                    let distance = distance_3d(query.center(), entity.position());
                    if distance <= query.radius() {
                        Some(QueryResult {
                            entity_id,
                            distance: Some(distance),
                            entity_data: entity.data().clone(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
    
    fn execute_k_nearest_query(
        &self,
        query: &super::KNearestQuery,
        grid: &HierarchicalGrid,
        entity_store: &EntityStore,
    ) -> Vec<QueryResult> {
        // Start with a reasonable search radius
        let mut search_radius = query.max_distance().unwrap_or(100.0);
        let mut results = Vec::new();
        
        // Iteratively expand search until we have k results
        loop {
            let candidates = grid.query_range(query.center(), search_radius);
            
            // Calculate distances for all candidates
            let mut candidates_with_distance: Vec<_> = candidates
                .into_iter()
                .filter_map(|entity_id| {
                    if let Some(entity) = entity_store.get(entity_id) {
                        // Filter by entity type if specified
                        if let Some(entity_type) = query.entity_type() {
                            if entity.entity_type() != entity_type {
                                return None;
                            }
                        }
                        
                        let distance = distance_3d(query.center(), entity.position());
                        
                        // Apply max distance filter if specified
                        if let Some(max_dist) = query.max_distance() {
                            if distance > max_dist {
                                return None;
                            }
                        }
                        
                        Some((entity_id, distance, entity.data().clone()))
                    } else {
                        None
                    }
                })
                .collect();
            
            // Sort by distance
            candidates_with_distance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            
            // Take the k nearest
            results = candidates_with_distance
                .into_iter()
                .take(query.k())
                .map(|(entity_id, distance, entity_data)| QueryResult {
                    entity_id,
                    distance: Some(distance),
                    entity_data,
                })
                .collect();
            
            // Check if we have enough results or can't expand further
            if results.len() >= query.k() || search_radius >= 10000.0 {
                break;
            }
            
            // Expand search radius
            search_radius *= 2.0;
        }
        
        results
    }
    
    fn execute_frustum_query(
        &self,
        query: &super::FrustumQuery,
        grid: &HierarchicalGrid,
        entity_store: &EntityStore,
    ) -> Vec<QueryResult> {
        // Get a bounding box for the frustum (conservative approximation)
        let (min, max) = frustum_bounds(query.frustum());
        let candidates = grid.query_box(min, max);
        
        // Test each candidate against the frustum
        candidates
            .par_iter()
            .filter_map(|&entity_id| {
                if let Some(entity) = entity_store.get(entity_id) {
                    // Filter by entity type if specified
                    if let Some(entity_type) = query.entity_type() {
                        if entity.entity_type() != entity_type {
                            return None;
                        }
                    }
                    
                    // Test if entity sphere is in frustum
                    if query.frustum().contains_sphere(entity.position(), entity.radius()) {
                        Some(QueryResult {
                            entity_id,
                            distance: None,
                            entity_data: entity.data().clone(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
    
    fn execute_box_query(
        &self,
        query: &super::BoxQuery,
        grid: &HierarchicalGrid,
        entity_store: &EntityStore,
    ) -> Vec<QueryResult> {
        let candidates = grid.query_box(query.min(), query.max());
        
        candidates
            .par_iter()
            .filter_map(|&entity_id| {
                if let Some(entity) = entity_store.get(entity_id) {
                    // Filter by entity type if specified
                    if let Some(entity_type) = query.entity_type() {
                        if entity.entity_type() != entity_type {
                            return None;
                        }
                    }
                    
                    // Check if entity center is in box
                    let pos = entity.position();
                    if pos[0] >= query.min()[0] && pos[0] <= query.max()[0] &&
                       pos[1] >= query.min()[1] && pos[1] <= query.max()[1] &&
                       pos[2] >= query.min()[2] && pos[2] <= query.max()[2] {
                        Some(QueryResult {
                            entity_id,
                            distance: None,
                            entity_data: entity.data().clone(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Execute queries that can share computation
    pub fn execute_shared_computation(
        &self,
        queries: Vec<SpatialQuery>,
        grid: &HierarchicalGrid,
        entity_store: &EntityStore,
    ) -> Vec<Vec<QueryResult>> {
        // Group queries by type and spatial locality
        let grouped = self.group_queries(queries);
        
        // Execute each group with shared data loading
        grouped
            .into_iter()
            .flat_map(|group| {
                self.execute_batch(group, grid, entity_store)
            })
            .collect()
    }
    
    /// Group queries that can benefit from shared computation
    fn group_queries(&self, queries: Vec<SpatialQuery>) -> Vec<Vec<SpatialQuery>> {
        // Simple grouping by query type
        // In a real implementation, we'd also consider spatial locality
        let mut range_queries = Vec::new();
        let mut k_nearest_queries = Vec::new();
        let mut frustum_queries = Vec::new();
        let mut box_queries = Vec::new();
        
        for query in queries {
            match query.query_type() {
                QueryType::Range(_) => range_queries.push(query),
                QueryType::KNearest(_) => k_nearest_queries.push(query),
                QueryType::Frustum(_) => frustum_queries.push(query),
                QueryType::Box(_) => box_queries.push(query),
            }
        }
        
        vec![range_queries, k_nearest_queries, frustum_queries, box_queries]
            .into_iter()
            .filter(|group| !group.is_empty())
            .collect()
    }
}

impl QueryBatch {
    pub fn new(queries: Vec<SpatialQuery>, priority: QueryPriority) -> Self {
        Self { queries, priority }
    }
    
    pub fn high_priority(queries: Vec<SpatialQuery>) -> Self {
        Self::new(queries, QueryPriority::High)
    }
    
    pub fn medium_priority(queries: Vec<SpatialQuery>) -> Self {
        Self::new(queries, QueryPriority::Medium)
    }
    
    pub fn low_priority(queries: Vec<SpatialQuery>) -> Self {
        Self::new(queries, QueryPriority::Low)
    }
}

fn distance_3d(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn frustum_bounds(frustum: &super::spatial_query::Frustum) -> ([f32; 3], [f32; 3]) {
    // Conservative bounding box for frustum
    // In a real implementation, this would be more precise
    (
        [-1000.0, -1000.0, -1000.0],
        [1000.0, 1000.0, 1000.0],
    )
}