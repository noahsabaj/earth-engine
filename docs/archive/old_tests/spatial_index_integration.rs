use earth_engine::spatial_index::*;

#[test]
fn test_basic_insert_and_query() {
    let config = SpatialIndexConfig::default();
    let mut index = SpatialIndex::new(config);
    
    // Insert some entities
    let entity1 = SpatialEntity::new(1, EntityType::Player, [0.0, 0.0, 0.0], 1.0);
    let entity2 = SpatialEntity::new(2, EntityType::Mob, [10.0, 0.0, 0.0], 1.0);
    let entity3 = SpatialEntity::new(3, EntityType::Item, [5.0, 5.0, 5.0], 0.5);
    
    index.insert(entity1).unwrap();
    index.insert(entity2).unwrap();
    index.insert(entity3).unwrap();
    
    // Range query
    let query = SpatialQuery::range([0.0, 0.0, 0.0], 15.0);
    let results = index.query(&query);
    
    assert_eq!(results.len(), 3);
    assert!(results.iter().any(|r| r.entity_id == 1));
    assert!(results.iter().any(|r| r.entity_id == 2));
    assert!(results.iter().any(|r| r.entity_id == 3));
}

#[test]
fn test_entity_type_filtering() {
    let config = SpatialIndexConfig::default();
    let mut index = SpatialIndex::new(config);
    
    // Insert entities of different types
    for i in 0..10 {
        let entity_type = match i % 3 {
            0 => EntityType::Player,
            1 => EntityType::Mob,
            _ => EntityType::Item,
        };
        
        let entity = SpatialEntity::new(
            i,
            entity_type,
            [i as f32 * 2.0, 0.0, 0.0],
            1.0,
        );
        index.insert(entity).unwrap();
    }
    
    // Query for only mobs
    let query = SpatialQuery::range([10.0, 0.0, 0.0], 20.0)
        .query_type()
        .clone();
    
    if let QueryType::Range(mut range_query) = query {
        range_query = range_query.with_type(EntityType::Mob);
        let query = SpatialQuery::range(range_query.center(), range_query.radius());
        let results = index.query(&query);
        
        // Should only find mobs
        for result in &results {
            assert_eq!(result.entity_data.entity_type, EntityType::Mob);
        }
    }
}

#[test]
fn test_entity_movement() {
    let config = SpatialIndexConfig::default();
    let mut index = SpatialIndex::new(config);
    
    // Insert entity
    let entity = SpatialEntity::new(1, EntityType::Player, [0.0, 0.0, 0.0], 1.0);
    index.insert(entity).unwrap();
    
    // Query at original position
    let query = SpatialQuery::range([0.0, 0.0, 0.0], 5.0);
    let results = index.query(&query);
    assert_eq!(results.len(), 1);
    
    // Move entity
    index.update(1, [20.0, 0.0, 0.0]).unwrap();
    
    // Query at original position - should be empty
    let results = index.query(&query);
    assert_eq!(results.len(), 0);
    
    // Query at new position
    let query = SpatialQuery::range([20.0, 0.0, 0.0], 5.0);
    let results = index.query(&query);
    assert_eq!(results.len(), 1);
}

#[test]
fn test_entity_removal() {
    let config = SpatialIndexConfig::default();
    let mut index = SpatialIndex::new(config);
    
    // Insert entities
    for i in 0..5 {
        let entity = SpatialEntity::new(
            i,
            EntityType::Item,
            [i as f32 * 5.0, 0.0, 0.0],
            1.0,
        );
        index.insert(entity).unwrap();
    }
    
    // Verify all entities exist
    let query = SpatialQuery::range([10.0, 0.0, 0.0], 50.0);
    let results = index.query(&query);
    assert_eq!(results.len(), 5);
    
    // Remove middle entity
    index.remove(2).unwrap();
    
    // Query again
    let results = index.query(&query);
    assert_eq!(results.len(), 4);
    assert!(!results.iter().any(|r| r.entity_id == 2));
}

#[test]
fn test_k_nearest_queries() {
    let config = SpatialIndexConfig::default();
    let mut index = SpatialIndex::new(config);
    
    // Insert entities at known distances
    for i in 0..20 {
        let entity = SpatialEntity::new(
            i,
            EntityType::Mob,
            [i as f32 * 2.0, 0.0, 0.0],
            1.0,
        );
        index.insert(entity).unwrap();
    }
    
    // Query for 5 nearest to origin
    let query = SpatialQuery::k_nearest([0.0, 0.0, 0.0], 5);
    let results = index.query(&query);
    
    assert_eq!(results.len(), 5);
    
    // Verify they are the closest ones
    let mut ids: Vec<_> = results.iter().map(|r| r.entity_id).collect();
    ids.sort();
    assert_eq!(ids, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_box_queries() {
    let config = SpatialIndexConfig::default();
    let mut index = SpatialIndex::new(config);
    
    // Insert entities in a grid pattern
    for x in 0..5 {
        for z in 0..5 {
            let entity = SpatialEntity::new(
                (x * 5 + z) as u64,
                EntityType::Structure,
                [x as f32 * 10.0, 0.0, z as f32 * 10.0],
                1.0,
            );
            index.insert(entity).unwrap();
        }
    }
    
    // Query a box in the middle
    let query = SpatialQuery::box_query(
        [5.0, -5.0, 5.0],
        [25.0, 5.0, 25.0],
    );
    let results = index.query(&query);
    
    // Should find 9 entities (3x3 grid)
    assert_eq!(results.len(), 9);
}

#[test]
fn test_parallel_batch_queries() {
    let config = SpatialIndexConfig::default();
    let mut index = SpatialIndex::new(config);
    
    // Insert many entities
    for i in 0..100 {
        let entity = SpatialEntity::new(
            i,
            EntityType::Particle,
            [
                (i as f32 * 7.0) % 100.0,
                0.0,
                (i as f32 * 13.0) % 100.0,
            ],
            0.5,
        );
        index.insert(entity).unwrap();
    }
    
    // Create multiple queries
    let queries = vec![
        SpatialQuery::range([0.0, 0.0, 0.0], 20.0),
        SpatialQuery::range([50.0, 0.0, 50.0], 20.0),
        SpatialQuery::range([25.0, 0.0, 25.0], 30.0),
        SpatialQuery::box_query([0.0, -10.0, 0.0], [30.0, 10.0, 30.0]),
    ];
    
    // Execute in parallel
    let results = index.batch_query(queries);
    
    assert_eq!(results.len(), 4);
    
    // Each query should return some results
    for result_set in &results {
        assert!(!result_set.is_empty());
    }
}

#[test]
fn test_world_bounds_validation() {
    let config = SpatialIndexConfig {
        world_min: [-100.0, -100.0, -100.0],
        world_max: [100.0, 100.0, 100.0],
        ..Default::default()
    };
    let mut index = SpatialIndex::new(config);
    
    // Insert entity within bounds - should succeed
    let entity = SpatialEntity::new(1, EntityType::Player, [0.0, 0.0, 0.0], 1.0);
    assert!(index.insert(entity).is_ok());
    
    // Insert entity outside bounds - should fail
    let entity = SpatialEntity::new(2, EntityType::Player, [200.0, 0.0, 0.0], 1.0);
    assert!(index.insert(entity).is_err());
}

#[test]
fn test_density_based_rebalancing() {
    let config = SpatialIndexConfig {
        max_entities_per_cell: 10,
        min_entities_per_cell: 2,
        ..Default::default()
    };
    let mut index = SpatialIndex::new(config);
    
    // Insert many entities in a small area to trigger high density
    for i in 0..50 {
        let entity = SpatialEntity::new(
            i,
            EntityType::Mob,
            [
                (i % 5) as f32,
                0.0,
                (i / 5) as f32,
            ],
            0.5,
        );
        index.insert(entity).unwrap();
    }
    
    // Get initial stats
    let stats_before = index.stats();
    
    // Trigger rebalancing
    index.rebalance();
    
    // Get stats after rebalancing
    let stats_after = index.stats();
    
    // Grid should have adapted to the density
    println!("Cells before: {}, after: {}", 
        stats_before.grid_stats.total_cells,
        stats_after.grid_stats.total_cells
    );
}

#[test]
fn test_cache_functionality() {
    let config = SpatialIndexConfig {
        enable_cache: true,
        cache_size_mb: 1,
        ..Default::default()
    };
    let mut index = SpatialIndex::new(config);
    
    // Insert some entities
    for i in 0..10 {
        let entity = SpatialEntity::new(
            i,
            EntityType::Item,
            [i as f32 * 5.0, 0.0, 0.0],
            1.0,
        );
        index.insert(entity).unwrap();
    }
    
    // Execute the same query multiple times
    let query = SpatialQuery::range([25.0, 0.0, 0.0], 20.0);
    
    // First query - cache miss
    let _ = index.query(&query);
    
    // Subsequent queries - cache hits
    for _ in 0..5 {
        let _ = index.query(&query);
    }
    
    let stats = index.stats();
    if let Some(cache_stats) = stats.cache_stats {
        assert!(cache_stats.hits > 0);
        assert!(cache_stats.misses > 0);
        assert!(cache_stats.hits > cache_stats.misses);
    }
}