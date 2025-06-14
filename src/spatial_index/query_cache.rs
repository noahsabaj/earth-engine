use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use super::{SpatialQuery, QueryResult};

/// LRU cache for spatial query results
pub struct QueryCache {
    cache: Arc<RwLock<LruCache>>,
    max_size_bytes: usize,
    current_size_bytes: Arc<RwLock<usize>>,
}

/// Statistics about cache performance
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub current_size_bytes: usize,
    pub max_size_bytes: usize,
    pub entry_count: usize,
}

struct LruCache {
    entries: HashMap<u64, CacheEntry>,
    access_order: Vec<u64>,
    stats: CacheStats,
}

struct CacheEntry {
    query: SpatialQuery,
    results: Vec<QueryResult>,
    size_bytes: usize,
    last_access: std::time::Instant,
    access_count: u32,
}

impl QueryCache {
    pub fn new(size_mb: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(size_mb))),
            max_size_bytes: size_mb * 1024 * 1024,
            current_size_bytes: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Get cached results for a query
    pub fn get(&self, query: &SpatialQuery) -> Option<Vec<QueryResult>> {
        let mut cache = self.cache.write();
        let key = query.cache_key();
        
        // Check if entry exists
        let has_entry = cache.entries.contains_key(&key);
        
        if has_entry {
            // Update access information
            if let Some(entry) = cache.entries.get_mut(&key) {
                entry.last_access = std::time::Instant::now();
                entry.access_count += 1;
            }
            
            // Move to front of LRU list
            cache.access_order.retain(|&k| k != key);
            cache.access_order.push(key);
            
            // Update stats
            cache.stats.hits += 1;
            
            // Get results
            cache.entries.get(&key).map(|entry| entry.results.clone())
        } else {
            cache.stats.misses += 1;
            None
        }
    }
    
    /// Store query results in cache
    pub fn put(&self, query: SpatialQuery, results: Vec<QueryResult>) {
        let mut cache = self.cache.write();
        let key = query.cache_key();
        
        // Calculate size of results
        let size_bytes = Self::estimate_size(&results);
        
        // Check if we need to evict entries
        let mut current_size = *self.current_size_bytes.read();
        while current_size + size_bytes > self.max_size_bytes && !cache.access_order.is_empty() {
            // Evict least recently used
            if let Some(evict_key) = cache.access_order.first().cloned() {
                if let Some(evicted) = cache.entries.remove(&evict_key) {
                    current_size -= evicted.size_bytes;
                    cache.stats.evictions += 1;
                }
                cache.access_order.remove(0);
            }
        }
        
        // Insert new entry
        let entry = CacheEntry {
            query: query.clone(),
            results,
            size_bytes,
            last_access: std::time::Instant::now(),
            access_count: 1,
        };
        
        cache.entries.insert(key, entry);
        cache.access_order.push(key);
        
        // Update size tracking
        *self.current_size_bytes.write() = current_size + size_bytes;
        cache.stats.current_size_bytes = current_size + size_bytes;
        cache.stats.entry_count = cache.entries.len();
    }
    
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let mut stats = cache.stats.clone();
        stats.max_size_bytes = self.max_size_bytes;
        stats
    }
    
    /// Estimate size of query results in bytes
    fn estimate_size(results: &[QueryResult]) -> usize {
        // Base size for Vec
        let mut size = std::mem::size_of::<Vec<QueryResult>>();
        
        // Size per result
        for result in results {
            size += std::mem::size_of::<QueryResult>();
            // Add size of entity data metadata
            size += result.entity_data.metadata.len() * 
                    (std::mem::size_of::<String>() * 2 + 32); // Rough estimate
        }
        
        size
    }
    
    /// Check if a query might be affected by changes in a region
    fn query_overlaps_region(query: &SpatialQuery, center: [f32; 3], radius: f32) -> bool {
        use super::QueryType;
        
        match query.query_type() {
            QueryType::Range(range_query) => {
                // Check if two spheres overlap
                let distance = distance_3d(range_query.center(), center);
                distance <= range_query.radius() + radius
            }
            QueryType::KNearest(k_nearest) => {
                // K-nearest queries are always potentially affected
                // unless we track their actual result bounds
                let distance = distance_3d(k_nearest.center(), center);
                if let Some(max_dist) = k_nearest.max_distance() {
                    distance <= max_dist + radius
                } else {
                    true // Conservative: always invalidate
                }
            }
            QueryType::Box(box_query) => {
                // Check if sphere overlaps box
                sphere_box_overlap(center, radius, box_query.min(), box_query.max())
            }
            QueryType::Frustum(_) => {
                // Frustum queries are complex - be conservative
                true
            }
        }
    }
    
    /// Preload cache with anticipated queries
    pub fn preload_anticipated_queries(
        &self,
        player_positions: &[[f32; 3]],
        common_radius: f32,
    ) {
        // In a real implementation, we'd generate and cache
        // queries that are likely to be needed soon
        for &pos in player_positions {
            let query = SpatialQuery::range(pos, common_radius);
            // Would execute query and cache results
            let _ = query; // Suppress warning
        }
    }
}

impl LruCache {
    fn new(size_mb: usize) -> Self {
        Self {
            entries: HashMap::new(),
            access_order: Vec::new(),
            stats: CacheStats {
                max_size_bytes: size_mb * 1024 * 1024,
                ..Default::default()
            },
        }
    }
}

fn distance_3d(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Invalidate cache entries within a region
/// Function - transforms query cache by removing entries affected by region changes
pub fn invalidate_cache_region(cache: &mut QueryCache, center: [f32; 3], radius: f32) {
    let mut cache_lock = cache.cache.write();
    let mut to_remove = Vec::new();
    
    // Find entries that might be affected by changes in this region
    for (key, entry) in cache_lock.entries.iter() {
        if QueryCache::query_overlaps_region(&entry.query, center, radius) {
            to_remove.push(*key);
        }
    }
    
    // Remove invalidated entries
    let mut size_removed = 0;
    for key in to_remove {
        if let Some(entry) = cache_lock.entries.remove(&key) {
            size_removed += entry.size_bytes;
            cache_lock.access_order.retain(|&k| k != key);
        }
    }
    
    // Update size
    let mut current_size = cache.current_size_bytes.write();
    *current_size = current_size.saturating_sub(size_removed);
    cache_lock.stats.current_size_bytes = *current_size;
    cache_lock.stats.entry_count = cache_lock.entries.len();
}

/// Clear all cached entries
/// Function - transforms query cache by clearing all entries
pub fn clear_query_cache(cache: &mut QueryCache) {
    let mut cache_lock = cache.cache.write();
    cache_lock.entries.clear();
    cache_lock.access_order.clear();
    *cache.current_size_bytes.write() = 0;
    cache_lock.stats.current_size_bytes = 0;
    cache_lock.stats.entry_count = 0;
}

fn sphere_box_overlap(center: [f32; 3], radius: f32, box_min: [f32; 3], box_max: [f32; 3]) -> bool {
    let mut closest = [0.0; 3];
    for i in 0..3 {
        closest[i] = center[i].max(box_min[i]).min(box_max[i]);
    }
    
    let distance = distance_3d(center, closest);
    distance <= radius
}