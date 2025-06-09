use earth_engine::spatial_index::*;
use std::time::Instant;
use rand::Rng;

fn main() {
    println!("=== Spatial Index Benchmark ===\n");
    
    // Create spatial index
    let config = SpatialIndexConfig {
        base_cell_size: 4.0,
        hierarchy_levels: 4,
        max_entities_per_cell: 32,
        min_entities_per_cell: 8,
        world_min: [-1024.0, -128.0, -1024.0],
        world_max: [1024.0, 256.0, 1024.0],
        enable_cache: true,
        cache_size_mb: 64,
        query_threads: num_cpus::get(),
    };
    
    println!("Configuration:");
    println!("  World size: {:?} to {:?}", config.world_min, config.world_max);
    println!("  Hierarchy levels: {}", config.hierarchy_levels);
    println!("  Base cell size: {}", config.base_cell_size);
    println!("  Query threads: {}", config.query_threads);
    println!();
    
    let mut index = SpatialIndex::new(config);
    let mut rng = rand::thread_rng();
    
    // Benchmark entity insertion
    println!("=== Entity Insertion ===");
    let entity_counts = [1000, 5000, 10000, 25000, 50000];
    
    for &count in &entity_counts {
        let start = Instant::now();
        
        for i in 0..count {
            let entity = SpatialEntity::new(
                i as u64,
                match rng.gen_range(0..4) {
                    0 => EntityType::Player,
                    1 => EntityType::Mob,
                    2 => EntityType::Item,
                    _ => EntityType::Projectile,
                },
                [
                    rng.gen_range(-1000.0..1000.0),
                    rng.gen_range(-100.0..200.0),
                    rng.gen_range(-1000.0..1000.0),
                ],
                rng.gen_range(0.5..2.0),
            );
            
            index.insert(entity).expect("Failed to insert entity");
        }
        
        let elapsed = start.elapsed();
        println!(
            "{} entities: {:.2}ms ({:.0} entities/sec)",
            count,
            elapsed.as_millis(),
            count as f64 / elapsed.as_secs_f64()
        );
    }
    
    println!("\n=== Spatial Query Performance ===");
    
    // Range queries
    println!("\nRange Queries:");
    let radii = [10.0, 25.0, 50.0, 100.0];
    
    for &radius in &radii {
        let mut total_results = 0;
        let mut total_time = std::time::Duration::ZERO;
        let num_queries = 100;
        
        for _ in 0..num_queries {
            let center = [
                rng.gen_range(-500.0..500.0),
                rng.gen_range(-50.0..100.0),
                rng.gen_range(-500.0..500.0),
            ];
            
            let query = SpatialQuery::range(center, radius);
            let start = Instant::now();
            let results = index.query(&query);
            total_time += start.elapsed();
            total_results += results.len();
        }
        
        println!(
            "  Radius {}: avg {:.3}ms, avg {} results",
            radius,
            total_time.as_secs_f64() * 1000.0 / num_queries as f64,
            total_results / num_queries
        );
    }
    
    // K-nearest queries
    println!("\nK-Nearest Queries:");
    let k_values = [5, 10, 20, 50];
    
    for &k in &k_values {
        let mut total_time = std::time::Duration::ZERO;
        let num_queries = 50;
        
        for _ in 0..num_queries {
            let center = [
                rng.gen_range(-500.0..500.0),
                rng.gen_range(-50.0..100.0),
                rng.gen_range(-500.0..500.0),
            ];
            
            let query = SpatialQuery::k_nearest(center, k);
            let start = Instant::now();
            let results = index.query(&query);
            total_time += start.elapsed();
            
            assert!(results.len() <= k);
        }
        
        println!(
            "  K={}: avg {:.3}ms",
            k,
            total_time.as_secs_f64() * 1000.0 / num_queries as f64
        );
    }
    
    // Box queries
    println!("\nBox Queries:");
    let box_sizes = [25.0, 50.0, 100.0, 200.0];
    
    for &size in &box_sizes {
        let mut total_results = 0;
        let mut total_time = std::time::Duration::ZERO;
        let num_queries = 100;
        
        for _ in 0..num_queries {
            let center = [
                rng.gen_range(-500.0..500.0),
                rng.gen_range(-50.0..100.0),
                rng.gen_range(-500.0..500.0),
            ];
            
            let half_size = size / 2.0;
            let query = SpatialQuery::box_query(
                [center[0] - half_size, center[1] - half_size, center[2] - half_size],
                [center[0] + half_size, center[1] + half_size, center[2] + half_size],
            );
            
            let start = Instant::now();
            let results = index.query(&query);
            total_time += start.elapsed();
            total_results += results.len();
        }
        
        println!(
            "  Box size {}: avg {:.3}ms, avg {} results",
            size,
            total_time.as_secs_f64() * 1000.0 / num_queries as f64,
            total_results / num_queries
        );
    }
    
    // Parallel batch queries
    println!("\n=== Parallel Batch Queries ===");
    let batch_sizes = [10, 50, 100, 500];
    
    for &batch_size in &batch_sizes {
        let mut queries = Vec::new();
        
        for _ in 0..batch_size {
            let center = [
                rng.gen_range(-500.0..500.0),
                rng.gen_range(-50.0..100.0),
                rng.gen_range(-500.0..500.0),
            ];
            queries.push(SpatialQuery::range(center, 50.0));
        }
        
        let start = Instant::now();
        let results = index.batch_query(queries);
        let elapsed = start.elapsed();
        
        let total_results: usize = results.iter().map(|r| r.len()).sum();
        
        println!(
            "  Batch size {}: {:.2}ms ({:.2}ms per query), {} total results",
            batch_size,
            elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 1000.0 / batch_size as f64,
            total_results
        );
    }
    
    // Entity movement
    println!("\n=== Entity Movement ===");
    let move_counts = [100, 500, 1000, 5000];
    
    for &count in &move_counts {
        let start = Instant::now();
        
        for i in 0..count {
            let entity_id = rng.gen_range(0..10000) as u64;
            let new_pos = [
                rng.gen_range(-1000.0..1000.0),
                rng.gen_range(-100.0..200.0),
                rng.gen_range(-1000.0..1000.0),
            ];
            
            let _ = index.update(entity_id, new_pos);
        }
        
        let elapsed = start.elapsed();
        println!(
            "{} moves: {:.2}ms ({:.0} moves/sec)",
            count,
            elapsed.as_millis(),
            count as f64 / elapsed.as_secs_f64()
        );
    }
    
    // Cache performance
    println!("\n=== Cache Performance ===");
    let stats = index.stats();
    if let Some(cache_stats) = stats.cache_stats {
        let hit_rate = if cache_stats.hits + cache_stats.misses > 0 {
            cache_stats.hits as f64 / (cache_stats.hits + cache_stats.misses) as f64 * 100.0
        } else {
            0.0
        };
        
        println!("  Hit rate: {:.1}%", hit_rate);
        println!("  Hits: {}", cache_stats.hits);
        println!("  Misses: {}", cache_stats.misses);
        println!("  Evictions: {}", cache_stats.evictions);
        println!("  Cache size: {:.2} MB / {:.2} MB",
            cache_stats.current_size_bytes as f64 / 1024.0 / 1024.0,
            cache_stats.max_size_bytes as f64 / 1024.0 / 1024.0
        );
        println!("  Entries: {}", cache_stats.entry_count);
    }
    
    // Final statistics
    println!("\n=== Final Statistics ===");
    println!("  Total entities: {}", stats.total_entities);
    println!("  Grid cells: {}", stats.grid_stats.total_cells);
    println!("  Max entities per cell: {}", stats.grid_stats.max_entities_per_cell);
    println!("\n  Cells by level:");
    for level_stats in &stats.grid_stats.cells_by_level {
        println!("    Level {}: {} cells, max {} entities",
            level_stats.level,
            level_stats.cell_count,
            level_stats.max_entities
        );
    }
    
    println!("\n  Density statistics:");
    println!("    Average density: {:.2}", stats.density_stats.average_density);
    println!("    Max density: {}", stats.density_stats.max_density);
    println!("    Hotspot cells: {}", stats.density_stats.hotspot_cells.len());
}