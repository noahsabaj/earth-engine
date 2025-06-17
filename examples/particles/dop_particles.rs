use hearth_engine::particles::{DOPParticleSystem, ParticleType};
use hearth_engine::particles::system::{
    create_fire_effect, create_rain_effect, create_explosion_effect
};
use hearth_engine::world::World;
use glam::Vec3;
use std::time::Duration;

fn main() {
    // Create a small test world
    let world = World::new(128);
    
    // Create particle system with max 100k particles
    let mut particle_system = DOPParticleSystem::new(100_000);
    
    println!("Data-Oriented Particle System Example");
    println!("=====================================");
    
    // Example 1: Create a fire effect
    println!("\n1. Creating fire effect at origin...");
    create_fire_effect(&mut particle_system, Vec3::ZERO, 2.0);
    
    // Example 2: Add rain
    println!("2. Adding rain effect...");
    create_rain_effect(
        &mut particle_system, 
        Vec3::new(0.0, 50.0, 0.0),
        Vec3::new(100.0, 50.0, 100.0),
        1.0
    );
    
    // Example 3: Manual emitter
    println!("3. Creating custom dust emitter...");
    let dust_id = particle_system.add_sphere_emitter(
        Vec3::new(10.0, 0.0, 10.0),
        5.0,
        ParticleType::Dust,
        50.0,
        Some(Duration::from_secs(10)),
    );
    particle_system.set_emitter_velocity(dust_id, Vec3::new(1.0, 0.5, 0.0), 0.2);
    
    // Example 4: Direct particle spawning
    println!("4. Spawning particles directly...");
    let positions = vec![
        Vec3::new(5.0, 5.0, 5.0),
        Vec3::new(5.0, 5.0, 6.0),
        Vec3::new(5.0, 5.0, 7.0),
    ];
    let velocities = vec![
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 1.5, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
    ];
    particle_system.spawn_particles(&positions, &velocities, ParticleType::Magic);
    
    // Simulate for a few frames
    println!("\n5. Simulating particle system...");
    
    // Enable wind
    particle_system.wind_velocity = Vec3::new(2.0, 0.0, 0.5);
    
    for frame in 0..10 {
        // Update system
        particle_system.update(Duration::from_millis(16), &world);
        
        // Apply some forces for variety
        if frame == 5 {
            println!("   - Creating explosion at frame 5!");
            create_explosion_effect(&mut particle_system, Vec3::new(-10.0, 5.0, -10.0), 5.0);
        }
        
        if frame % 3 == 0 {
            // Apply a vortex
            particle_system.apply_vortex(
                Vec3::new(0.0, 10.0, 0.0),
                Vec3::Y,
                10.0,
                20.0
            );
        }
        
        // Print stats
        let stats = particle_system.get_stats();
        println!("   Frame {}: {} particles, {} emitters", 
            frame, 
            stats.total_particles,
            stats.active_emitters
        );
    }
    
    // Get render data
    let gpu_data = particle_system.get_gpu_data();
    println!("\n6. GPU buffer ready with {} particles for rendering", gpu_data.len());
    
    // Show detailed stats
    let stats = particle_system.get_stats();
    println!("\nFinal Statistics:");
    println!("  Total particles: {}", stats.total_particles);
    println!("  Active emitters: {}", stats.active_emitters);
    println!("  Capacity used: {:.1}%", stats.capacity_used * 100.0);
    println!("  Particles by type:");
    println!("    Rain: {}", stats.particles_by_type[ParticleType::Rain as usize]);
    println!("    Snow: {}", stats.particles_by_type[ParticleType::Snow as usize]);
    println!("    Fire: {}", stats.particles_by_type[ParticleType::Fire as usize]);
    println!("    Smoke: {}", stats.particles_by_type[ParticleType::Smoke as usize]);
    println!("    Dust: {}", stats.particles_by_type[ParticleType::Dust as usize]);
    println!("    Magic: {}", stats.particles_by_type[ParticleType::Magic as usize]);
    
    // Demonstrate removal
    println!("\n7. Removing dust emitter...");
    particle_system.remove_emitter(dust_id);
    particle_system.update(Duration::from_millis(16), &world);
    println!("   Emitters remaining: {}", particle_system.emitter_count());
    
    // Clear everything
    println!("\n8. Clearing all particles...");
    particle_system.clear();
    println!("   Final particle count: {}", particle_system.particle_count());
    
    println!("\nData-oriented particle system demonstration complete!");
    println!("\nKey advantages:");
    println!("  - All particle data in contiguous arrays (cache-friendly)");
    println!("  - No virtual function calls or object indirection");
    println!("  - Easy to parallelize with SIMD/GPU");
    println!("  - Efficient batch operations");
    println!("  - Pre-allocated memory pools");
}