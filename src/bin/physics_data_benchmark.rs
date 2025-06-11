use earth_engine::physics_data::{
    PhysicsData, CollisionData, SpatialHash, SpatialHashConfig,
    ParallelPhysicsSolver, SolverConfig, PhysicsIntegrator, PhysicsConfig
};
use earth_engine::profiling::{CacheProfiler, PerformanceMetrics};
use std::time::Instant;

fn main() {
    println!("=== Data-Oriented Physics Benchmark ===\n");
    
    let profiler = CacheProfiler::new();
    let metrics = PerformanceMetrics::new();
    
    // Configuration
    let entity_counts = vec![1000, 5000, 10000, 20000];
    let physics_config = PhysicsConfig::default();
    let solver_config = SolverConfig::default();
    let spatial_config = SpatialHashConfig {
        cell_size: 2.0,
        world_min: [-100.0, 0.0, -100.0],
        world_max: [100.0, 50.0, 100.0],
        expected_entities_per_cell: 4,
    };
    
    for &entity_count in &entity_counts {
        println!("Testing with {} entities...", entity_count);
        
        // Create physics system
        let mut physics_data = PhysicsData::new(entity_count);
        let mut collision_data = CollisionData::new(entity_count * 2);
        let spatial_hash = SpatialHash::new(spatial_config.clone());
        let mut solver = ParallelPhysicsSolver::new(solver_config.clone())
            .expect("Failed to create physics solver");
        let mut integrator = PhysicsIntegrator::new(entity_count);
        
        // Add random entities
        let start_spawn = Instant::now();
        for i in 0..entity_count {
            let x = (i % 100) as f32 * 2.0 - 100.0;
            let y = (i / 100) as f32 * 2.0 + 1.0;
            let z = ((i * 7) % 100) as f32 * 2.0 - 100.0;
            
            let vx = ((i * 13) % 20) as f32 - 10.0;
            let vy = ((i * 17) % 10) as f32;
            let vz = ((i * 23) % 20) as f32 - 10.0;
            
            physics_data.add_entity(
                [x, y, z],
                [vx * 0.1, vy * 0.1, vz * 0.1],
                1.0,
                [0.5, 0.5, 0.5],
            );
        }
        let spawn_time = start_spawn.elapsed();
        
        // Run physics simulation
        let mut total_step_time = std::time::Duration::ZERO;
        let mut total_cache_efficiency = 0.0;
        let steps = 100;
        
        for step in 0..steps {
            // Record cache access patterns
            let prev_addr = step * 1000; // Simulate memory access
            profiler.record_access(prev_addr, entity_count * 4 * 12, Some(prev_addr));
            
            let step_start = Instant::now();
            
            // Physics step
            integrator.update(&mut physics_data, 0.016, |physics, dt| {
                solver.step(physics, &mut collision_data, &spatial_hash, dt);
            });
            
            let step_time = step_start.elapsed();
            total_step_time += step_time;
            
            // Update metrics
            metrics.record_frame(step_time);
            metrics.record_cache_efficiency((profiler.cache_efficiency() * 100.0) as u64);
            total_cache_efficiency += profiler.cache_efficiency();
            
            // Print stats every 25 steps
            if (step + 1) % 25 == 0 {
                let stats = solver.get_stats();
                println!(
                    "  Step {}: {:.2}ms, {} collisions, {} contacts",
                    step + 1,
                    step_time.as_secs_f64() * 1000.0,
                    stats.narrow_phase_pairs,
                    stats.contact_points
                );
            }
        }
        
        // Print results
        let avg_step_time = total_step_time.as_secs_f64() / steps as f64 * 1000.0;
        let avg_cache_efficiency = total_cache_efficiency / steps as f64 * 100.0;
        let final_stats = solver.get_stats();
        
        println!("\nResults for {} entities:", entity_count);
        println!("  Entity spawn time: {:.2}ms", spawn_time.as_secs_f64() * 1000.0);
        println!("  Average step time: {:.2}ms", avg_step_time);
        println!("  Average FPS: {:.2}", 1000.0 / avg_step_time);
        println!("  Cache efficiency: {:.1}%", avg_cache_efficiency);
        println!("  Broad phase time: {:.2}ms", final_stats.broad_phase_time_us as f64 / 1000.0);
        println!("  Narrow phase time: {:.2}ms", final_stats.narrow_phase_time_us as f64 / 1000.0);
        println!("  Solver time: {:.2}ms", final_stats.solver_time_us as f64 / 1000.0);
        println!("  Entities/second: {:.0}", entity_count as f64 / avg_step_time * 1000.0);
        
        // Test memory layout efficiency
        println!("\nMemory Layout Analysis:");
        let position_size = entity_count * std::mem::size_of::<[f32; 3]>();
        let velocity_size = entity_count * std::mem::size_of::<[f32; 3]>();
        let total_size = position_size + velocity_size;
        println!("  Position array: {} KB", position_size / 1024);
        println!("  Velocity array: {} KB", velocity_size / 1024);
        println!("  Total SoA size: {} KB", total_size / 1024);
        
        // Compare with AoS layout
        let aos_size = entity_count * (
            std::mem::size_of::<[f32; 3]>() * 4 + // pos, vel, rot, ang_vel
            std::mem::size_of::<f32>() * 4 +      // mass, inv_mass, restitution, friction
            std::mem::size_of::<u32>() * 3 +      // groups, masks, flags
            64                                     // padding/alignment
        );
        println!("  Equivalent AoS size: {} KB", aos_size / 1024);
        println!("  Memory savings: {:.1}%", (1.0 - total_size as f64 / aos_size as f64) * 100.0);
        
        println!("\n{}", "-".repeat(50));
    }
    
    // Print overall performance metrics
    println!("\n=== Overall Performance Metrics ===");
    metrics.report();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_physics_data_creation() {
        let mut physics = PhysicsData::new(100);
        let entity = physics.add_entity([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0, [0.5, 0.5, 0.5]);
        assert_eq!(physics.entity_count(), 1);
        assert!(entity.is_valid());
    }
    
    #[test]
    fn test_spatial_hash() {
        let config = SpatialHashConfig::default();
        let spatial_hash = SpatialHash::new(config);
        
        let entity = earth_engine::physics_data::EntityId(0);
        let aabb = earth_engine::physics_data::physics_tables::AABB::new(
            [-1.0, -1.0, -1.0],
            [1.0, 1.0, 1.0],
        );
        
        spatial_hash.insert(entity, &aabb);
        let query_region = earth_engine::physics_data::physics_tables::AABB::new(
            [-2.0, -2.0, -2.0],
            [2.0, 2.0, 2.0],
        );
        let results = spatial_hash.query_region(&query_region);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], entity);
    }
}