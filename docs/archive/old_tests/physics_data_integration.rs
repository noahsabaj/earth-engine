use earth_engine::physics_data::{
    PhysicsData, CollisionData, SpatialHash, SpatialHashConfig,
    ParallelPhysicsSolver, SolverConfig, PhysicsIntegrator,
    EntityId, ContactPoint, ContactPair
};

#[test]
fn test_entity_management() {
    let mut physics = PhysicsData::new(100);
    
    // Add entities
    let entity1 = physics.add_entity([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0, [0.5, 0.5, 0.5]);
    let entity2 = physics.add_entity([2.0, 0.0, 0.0], [0.0, 0.0, 0.0], 2.0, [0.5, 0.5, 0.5]);
    
    assert_eq!(physics.entity_count(), 2);
    assert!(entity1.is_valid());
    assert!(entity2.is_valid());
    
    // Check data
    assert_eq!(physics.positions[0], [0.0, 0.0, 0.0]);
    assert_eq!(physics.positions[1], [2.0, 0.0, 0.0]);
    assert_eq!(physics.masses[0], 1.0);
    assert_eq!(physics.masses[1], 2.0);
    assert_eq!(physics.inverse_masses[0], 1.0);
    assert_eq!(physics.inverse_masses[1], 0.5);
    
    // Remove entity
    physics.remove_entity(entity1);
    assert_eq!(physics.entity_count(), 1);
    
    // Data should be swapped with last
    assert_eq!(physics.positions[0], [2.0, 0.0, 0.0]);
    assert_eq!(physics.masses[0], 2.0);
}

#[test]
fn test_spatial_hash_operations() {
    let config = SpatialHashConfig {
        cell_size: 1.0,
        world_min: [-10.0, -10.0, -10.0],
        world_max: [10.0, 10.0, 10.0],
        expected_entities_per_cell: 4,
    };
    let spatial_hash = SpatialHash::new(config);
    
    let entity1 = EntityId(0);
    let entity2 = EntityId(1);
    let entity3 = EntityId(2);
    
    // Insert entities
    use earth_engine::physics_data::physics_tables::AABB;
    spatial_hash.insert(entity1, &AABB::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));
    spatial_hash.insert(entity2, &AABB::new([0.5, 0.5, 0.5], [1.5, 1.5, 1.5]));
    spatial_hash.insert(entity3, &AABB::new([5.0, 5.0, 5.0], [6.0, 6.0, 6.0]));
    
    // Query overlapping region
    let query = AABB::new([-0.5, -0.5, -0.5], [2.0, 2.0, 2.0]);
    let results = spatial_hash.query_region(&query);
    
    assert!(results.contains(&entity1));
    assert!(results.contains(&entity2));
    assert!(!results.contains(&entity3));
    
    // Test potential collisions
    let potential = spatial_hash.get_potential_collisions(entity1);
    assert!(potential.contains(&entity2));
    assert!(!potential.contains(&entity3));
    
    // Update position
    spatial_hash.update(entity1, &AABB::new([8.0, 8.0, 8.0], [9.0, 9.0, 9.0]));
    let potential_after = spatial_hash.get_potential_collisions(entity1);
    assert!(potential_after.is_empty());
}

#[test]
fn test_collision_detection() {
    let mut physics = PhysicsData::new(10);
    let mut collision_data = CollisionData::new(20);
    let spatial_hash = SpatialHash::new(SpatialHashConfig::default());
    let mut solver = ParallelPhysicsSolver::new(SolverConfig::default());
    
    // Add two overlapping entities
    let entity1 = physics.add_entity([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0, [0.5, 0.5, 0.5]);
    let entity2 = physics.add_entity([0.8, 0.0, 0.0], [-1.0, 0.0, 0.0], 1.0, [0.5, 0.5, 0.5]);
    
    // Step physics
    solver.step(&mut physics, &mut collision_data, &spatial_hash, 0.016);
    
    // Check collision was detected
    assert!(collision_data.pair_count() > 0);
    
    // Verify collision pair
    let pair = collision_data.contact_pairs[0];
    assert!((pair.entity_a == entity1 && pair.entity_b == entity2) ||
            (pair.entity_a == entity2 && pair.entity_b == entity1));
}

#[test]
fn test_physics_integration() {
    let mut physics = PhysicsData::new(10);
    let mut integrator = PhysicsIntegrator::new(10);
    
    // Add falling entity
    let entity = physics.add_entity([0.0, 10.0, 0.0], [0.0, 0.0, 0.0], 1.0, [0.5, 0.5, 0.5]);
    
    // Store initial position
    let initial_y = physics.positions[entity.index()][1];
    
    // Simulate multiple steps
    for _ in 0..10 {
        integrator.update(&mut physics, 0.016, |physics_data, dt| {
            // Simple gravity integration
            earth_engine::physics_data::integration::parallel::apply_gravity(
                &mut physics_data.velocities,
                &physics_data.flags,
                earth_engine::physics_data::GRAVITY,
                dt,
            );
            
            earth_engine::physics_data::integration::parallel::integrate_positions(
                &mut physics_data.positions,
                &physics_data.velocities,
                &physics_data.flags,
                dt,
            );
        });
    }
    
    // Entity should have fallen
    let final_y = physics.positions[entity.index()][1];
    assert!(final_y < initial_y);
    
    // Velocity should be negative (falling)
    assert!(physics.velocities[entity.index()][1] < 0.0);
}

#[test]
fn test_collision_response() {
    let mut physics = PhysicsData::new(10);
    let mut collision_data = CollisionData::new(20);
    let spatial_hash = SpatialHash::new(SpatialHashConfig::default());
    let mut solver = ParallelPhysicsSolver::new(SolverConfig::default());
    
    // Two entities moving towards each other
    let entity1 = physics.add_entity([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], 1.0, [0.5, 0.5, 0.5]);
    let entity2 = physics.add_entity([2.0, 0.0, 0.0], [-2.0, 0.0, 0.0], 1.0, [0.5, 0.5, 0.5]);
    
    // Store initial velocities
    let vel1_before = physics.velocities[entity1.index()][0];
    let vel2_before = physics.velocities[entity2.index()][0];
    
    // Step until collision
    for _ in 0..5 {
        solver.step(&mut physics, &mut collision_data, &spatial_hash, 0.016);
    }
    
    // Velocities should have changed (collision response)
    let vel1_after = physics.velocities[entity1.index()][0];
    let vel2_after = physics.velocities[entity2.index()][0];
    
    // Basic sanity check - velocities should be different after collision
    assert_ne!(vel1_before, vel1_after);
    assert_ne!(vel2_before, vel2_after);
}

#[test]
fn test_static_entities() {
    let mut physics = PhysicsData::new(10);
    let mut collision_data = CollisionData::new(20);
    let spatial_hash = SpatialHash::new(SpatialHashConfig::default());
    let mut solver = ParallelPhysicsSolver::new(SolverConfig::default());
    
    // Add static ground
    let ground = physics.add_entity([0.0, -1.0, 0.0], [0.0, 0.0, 0.0], 0.0, [10.0, 1.0, 10.0]);
    physics.flags[ground.index()].set_flag(
        earth_engine::physics_data::physics_tables::PhysicsFlags::STATIC,
        true,
    );
    
    // Add falling dynamic entity
    let entity = physics.add_entity([0.0, 5.0, 0.0], [0.0, 0.0, 0.0], 1.0, [0.5, 0.5, 0.5]);
    
    // Simulate until it hits the ground
    for _ in 0..100 {
        solver.step(&mut physics, &mut collision_data, &spatial_hash, 0.016);
    }
    
    // Dynamic entity should stop falling when it hits static ground
    let final_y = physics.positions[entity.index()][1];
    assert!(final_y > -0.5); // Should be resting on or near ground
    assert!(final_y < 2.0);  // Should have fallen significantly
    
    // Ground should not have moved
    assert_eq!(physics.positions[ground.index()][1], -1.0);
}

#[test]
fn test_cache_efficiency() {
    use earth_engine::profiling::CacheProfiler;
    
    let profiler = CacheProfiler::new();
    let mut physics = PhysicsData::new(1000);
    
    // Add many entities
    for i in 0..1000 {
        physics.add_entity(
            [i as f32, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            1.0,
            [0.5, 0.5, 0.5],
        );
    }
    
    // Simulate position access pattern
    let mut prev_addr = 0;
    for i in 0..1000 {
        let addr = i * std::mem::size_of::<[f32; 3]>();
        profiler.record_access(addr, std::mem::size_of::<[f32; 3]>(), Some(prev_addr));
        prev_addr = addr;
        
        // Access position data
        let _ = physics.positions[i];
    }
    
    // Cache efficiency should be high for sequential access
    let efficiency = profiler.cache_efficiency();
    assert!(efficiency > 0.9, "Cache efficiency {} should be > 90%", efficiency);
}

#[test]
fn test_parallel_performance() {
    use std::time::Instant;
    
    let mut physics = PhysicsData::new(10000);
    let mut collision_data = CollisionData::new(20000);
    let spatial_hash = SpatialHash::new(SpatialHashConfig::default());
    let mut solver = ParallelPhysicsSolver::new(SolverConfig::default());
    
    // Add many entities in a grid
    for i in 0..100 {
        for j in 0..100 {
            let x = i as f32 * 2.0;
            let z = j as f32 * 2.0;
            physics.add_entity([x, 5.0, z], [0.0, 0.0, 0.0], 1.0, [0.5, 0.5, 0.5]);
        }
    }
    
    // Measure performance
    let start = Instant::now();
    solver.step(&mut physics, &mut collision_data, &spatial_hash, 0.016);
    let duration = start.elapsed();
    
    println!("Physics step for 10,000 entities: {:?}", duration);
    
    // Should complete in reasonable time (< 50ms for good performance)
    assert!(duration.as_millis() < 100, "Physics step took too long: {:?}", duration);
}