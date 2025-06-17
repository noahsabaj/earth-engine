/// SOA vs AOS Performance Benchmark
/// 
/// This benchmark demonstrates the cache efficiency improvements
/// achieved by converting from Array-of-Structures to Structure-of-Arrays.

use earth_engine::ecs::{
    SoAWorld, 
    add_transform_component, 
    add_physics_component,
    soa_update_physics_system,
};
use earth_engine::particles::{ParticleData, MAX_PARTICLES};
use std::time::Instant;
use rand::Rng;

/// Traditional AOS Particle for comparison
#[derive(Clone)]
struct AosParticle {
    position: [f32; 3],
    velocity: [f32; 3],
    acceleration: [f32; 3],
    color: [f32; 4],
    size: f32,
    lifetime: f32,
    max_lifetime: f32,
}

impl AosParticle {
    fn new(position: [f32; 3], velocity: [f32; 3]) -> Self {
        Self {
            position,
            velocity,
            acceleration: [0.0; 3],
            color: [1.0; 4],
            size: 1.0,
            lifetime: 5.0,
            max_lifetime: 5.0,
        }
    }
    
    fn update(&mut self, dt: f32) {
        // Apply acceleration to velocity
        self.velocity[0] += self.acceleration[0] * dt;
        self.velocity[1] += self.acceleration[1] * dt;
        self.velocity[2] += self.acceleration[2] * dt;
        
        // Apply gravity
        self.velocity[1] -= 9.81 * dt;
        
        // Update position
        self.position[0] += self.velocity[0] * dt;
        self.position[1] += self.velocity[1] * dt;
        self.position[2] += self.velocity[2] * dt;
        
        // Update lifetime
        self.lifetime -= dt;
    }
    
    fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }
}

/// SOA particle update function
fn update_particles_soa(particles: &mut ParticleData, dt: f32) {
    let count = particles.count;
    
    // Apply acceleration to velocity (vectorizable)
    for i in 0..count {
        particles.velocity_x[i] += particles.acceleration_x[i] * dt;
        particles.velocity_y[i] += particles.acceleration_y[i] * dt;
        particles.velocity_z[i] += particles.acceleration_z[i] * dt;
    }
    
    // Apply gravity (vectorizable)
    for i in 0..count {
        particles.velocity_y[i] -= 9.81 * particles.gravity_multiplier[i] * dt;
    }
    
    // Update positions (vectorizable)
    for i in 0..count {
        particles.position_x[i] += particles.velocity_x[i] * dt;
        particles.position_y[i] += particles.velocity_y[i] * dt;
        particles.position_z[i] += particles.velocity_z[i] * dt;
    }
    
    // Update lifetimes (vectorizable)
    for i in 0..count {
        particles.lifetime[i] -= dt;
    }
}

/// Benchmark: Position-only operations
fn benchmark_position_only_access(count: usize, iterations: usize) {
    println!("\n=== Position-Only Access Benchmark ===");
    
    // AOS version - positions scattered throughout struct
    let mut aos_particles: Vec<AosParticle> = Vec::with_capacity(count);
    for i in 0..count {
        aos_particles.push(AosParticle::new(
            [i as f32, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        ));
    }
    
    // SOA version - positions contiguous in memory
    let mut soa_particles = ParticleData::new(count);
    for i in 0..count {
        soa_particles.position_x.push(i as f32);
        soa_particles.position_y.push(0.0);
        soa_particles.position_z.push(0.0);
        soa_particles.velocity_x.push(1.0);
        soa_particles.velocity_y.push(1.0);
        soa_particles.velocity_z.push(1.0);
        soa_particles.acceleration_x.push(0.0);
        soa_particles.acceleration_y.push(0.0);
        soa_particles.acceleration_z.push(0.0);
        soa_particles.lifetime.push(5.0);
        soa_particles.max_lifetime.push(5.0);
        soa_particles.gravity_multiplier.push(1.0);
        // Fill other fields with defaults
        soa_particles.color_r.push(1.0);
        soa_particles.color_g.push(1.0);
        soa_particles.color_b.push(1.0);
        soa_particles.color_a.push(1.0);
        soa_particles.size.push(1.0);
        soa_particles.particle_type.push(0);
        soa_particles.drag.push(0.0);
        soa_particles.bounce.push(0.0);
        soa_particles.rotation.push(0.0);
        soa_particles.rotation_speed.push(0.0);
        soa_particles.texture_frame.push(0);
        soa_particles.animation_speed.push(0.0);
        soa_particles.emissive.push(false);
        soa_particles.emission_intensity.push(0.0);
        soa_particles.size_curve_type.push(0);
        soa_particles.size_curve_param1.push(0.0);
        soa_particles.size_curve_param2.push(0.0);
        soa_particles.size_curve_param3.push(0.0);
        soa_particles.color_curve_type.push(0);
        soa_particles.color_curve_param1.push(0.0);
        soa_particles.color_curve_param2.push(0.0);
    }
    soa_particles.count = count;
    
    // Benchmark AOS position access
    let aos_start = Instant::now();
    for _ in 0..iterations {
        let mut sum = 0.0f32;
        for particle in &aos_particles {
            sum += particle.position[0] + particle.position[1] + particle.position[2];
        }
        // Prevent optimization
        std::hint::black_box(sum);
    }
    let aos_duration = aos_start.elapsed();
    
    // Benchmark SOA position access
    let soa_start = Instant::now();
    for _ in 0..iterations {
        let mut sum = 0.0f32;
        for i in 0..count {
            sum += soa_particles.position_x[i] + 
                   soa_particles.position_y[i] + 
                   soa_particles.position_z[i];
        }
        // Prevent optimization
        std::hint::black_box(sum);
    }
    let soa_duration = soa_start.elapsed();
    
    println!("Particles: {}, Iterations: {}", count, iterations);
    println!("AOS Duration: {:?}", aos_duration);
    println!("SOA Duration: {:?}", soa_duration);
    println!("SOA Speedup: {:.2}x", aos_duration.as_secs_f64() / soa_duration.as_secs_f64());
}

/// Benchmark: Full physics update
fn benchmark_physics_update(count: usize, iterations: usize) {
    println!("\n=== Physics Update Benchmark ===");
    
    let mut rng = rand::thread_rng();
    
    // AOS version
    let mut aos_particles: Vec<AosParticle> = Vec::with_capacity(count);
    for _ in 0..count {
        aos_particles.push(AosParticle::new(
            [rng.gen_range(-100.0..100.0), rng.gen_range(0.0..100.0), rng.gen_range(-100.0..100.0)],
            [rng.gen_range(-10.0..10.0), rng.gen_range(-10.0..10.0), rng.gen_range(-10.0..10.0)],
        ));
    }
    
    // SOA version
    let mut soa_particles = ParticleData::new(count);
    for _ in 0..count {
        soa_particles.position_x.push(rng.gen_range(-100.0..100.0));
        soa_particles.position_y.push(rng.gen_range(0.0..100.0));
        soa_particles.position_z.push(rng.gen_range(-100.0..100.0));
        soa_particles.velocity_x.push(rng.gen_range(-10.0..10.0));
        soa_particles.velocity_y.push(rng.gen_range(-10.0..10.0));
        soa_particles.velocity_z.push(rng.gen_range(-10.0..10.0));
        soa_particles.acceleration_x.push(0.0);
        soa_particles.acceleration_y.push(0.0);
        soa_particles.acceleration_z.push(0.0);
        soa_particles.lifetime.push(5.0);
        soa_particles.max_lifetime.push(5.0);
        soa_particles.gravity_multiplier.push(1.0);
        // Fill other required fields
        soa_particles.color_r.push(1.0);
        soa_particles.color_g.push(1.0);
        soa_particles.color_b.push(1.0);
        soa_particles.color_a.push(1.0);
        soa_particles.size.push(1.0);
        soa_particles.particle_type.push(0);
        soa_particles.drag.push(0.0);
        soa_particles.bounce.push(0.0);
        soa_particles.rotation.push(0.0);
        soa_particles.rotation_speed.push(0.0);
        soa_particles.texture_frame.push(0);
        soa_particles.animation_speed.push(0.0);
        soa_particles.emissive.push(false);
        soa_particles.emission_intensity.push(0.0);
        soa_particles.size_curve_type.push(0);
        soa_particles.size_curve_param1.push(0.0);
        soa_particles.size_curve_param2.push(0.0);
        soa_particles.size_curve_param3.push(0.0);
        soa_particles.color_curve_type.push(0);
        soa_particles.color_curve_param1.push(0.0);
        soa_particles.color_curve_param2.push(0.0);
    }
    soa_particles.count = count;
    
    let dt = 1.0 / 60.0; // 60 FPS
    
    // Benchmark AOS physics update
    let aos_start = Instant::now();
    for _ in 0..iterations {
        for particle in &mut aos_particles {
            particle.update(dt);
        }
    }
    let aos_duration = aos_start.elapsed();
    
    // Benchmark SOA physics update
    let soa_start = Instant::now();
    for _ in 0..iterations {
        update_particles_soa(&mut soa_particles, dt);
    }
    let soa_duration = soa_start.elapsed();
    
    println!("Particles: {}, Iterations: {}", count, iterations);
    println!("AOS Duration: {:?}", aos_duration);
    println!("SOA Duration: {:?}", soa_duration);
    println!("SOA Speedup: {:.2}x", aos_duration.as_secs_f64() / soa_duration.as_secs_f64());
}

/// Benchmark: ECS physics system
fn benchmark_ecs_physics(entity_count: usize, iterations: usize) {
    println!("\n=== ECS Physics System Benchmark ===");
    
    let mut world = SoAWorld::new();
    let mut rng = rand::thread_rng();
    
    // Create entities with transform and physics components
    for _ in 0..entity_count {
        let entity = world.create_entity();
        
        add_transform_component(
            &mut world.transforms,
            &mut world.entities,
            entity,
            [rng.gen_range(-100.0..100.0), rng.gen_range(0.0..100.0), rng.gen_range(-100.0..100.0)],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        );
        
        add_physics_component(
            &mut world.physics,
            &mut world.entities,
            entity,
            [rng.gen_range(-10.0..10.0), rng.gen_range(-10.0..10.0), rng.gen_range(-10.0..10.0)],
            1.0,
        );
    }
    
    let dt = 1.0 / 60.0; // 60 FPS
    
    // Benchmark SOA ECS physics update
    let start = Instant::now();
    for _ in 0..iterations {
        soa_update_physics_system(&mut world.transforms, &mut world.physics, dt);
    }
    let duration = start.elapsed();
    
    println!("Entities: {}, Iterations: {}", entity_count, iterations);
    println!("SOA ECS Duration: {:?}", duration);
    println!("Entities/sec: {:.0}", entity_count as f64 * iterations as f64 / duration.as_secs_f64());
}

/// Benchmark: Memory locality test
fn benchmark_memory_locality(count: usize) {
    println!("\n=== Memory Locality Benchmark ===");
    
    // Test random access pattern on AOS vs SOA
    let mut rng = rand::thread_rng();
    let indices: Vec<usize> = (0..count).map(|_| rng.gen_range(0..count)).collect();
    
    // AOS version
    let mut aos_particles: Vec<AosParticle> = Vec::with_capacity(count);
    for i in 0..count {
        aos_particles.push(AosParticle::new([i as f32; 3], [1.0; 3]));
    }
    
    // SOA version  
    let mut soa_particles = ParticleData::new(count);
    for i in 0..count {
        soa_particles.position_x.push(i as f32);
        soa_particles.position_y.push(i as f32);
        soa_particles.position_z.push(i as f32);
        // Fill minimal required fields
        soa_particles.velocity_x.push(1.0);
        soa_particles.velocity_y.push(1.0);
        soa_particles.velocity_z.push(1.0);
        soa_particles.acceleration_x.push(0.0);
        soa_particles.acceleration_y.push(0.0);
        soa_particles.acceleration_z.push(0.0);
        soa_particles.lifetime.push(5.0);
        soa_particles.max_lifetime.push(5.0);
        soa_particles.gravity_multiplier.push(1.0);
        soa_particles.color_r.push(1.0);
        soa_particles.color_g.push(1.0);
        soa_particles.color_b.push(1.0);
        soa_particles.color_a.push(1.0);
        soa_particles.size.push(1.0);
        soa_particles.particle_type.push(0);
        soa_particles.drag.push(0.0);
        soa_particles.bounce.push(0.0);
        soa_particles.rotation.push(0.0);
        soa_particles.rotation_speed.push(0.0);
        soa_particles.texture_frame.push(0);
        soa_particles.animation_speed.push(0.0);
        soa_particles.emissive.push(false);
        soa_particles.emission_intensity.push(0.0);
        soa_particles.size_curve_type.push(0);
        soa_particles.size_curve_param1.push(0.0);
        soa_particles.size_curve_param2.push(0.0);
        soa_particles.size_curve_param3.push(0.0);
        soa_particles.color_curve_type.push(0);
        soa_particles.color_curve_param1.push(0.0);
        soa_particles.color_curve_param2.push(0.0);
    }
    soa_particles.count = count;
    
    // Test random position access (AOS)
    let aos_start = Instant::now();
    let mut sum = 0.0f32;
    for &idx in &indices {
        sum += aos_particles[idx].position[0];
    }
    let aos_duration = aos_start.elapsed();
    std::hint::black_box(sum);
    
    // Test random position access (SOA)
    let soa_start = Instant::now();
    let mut sum = 0.0f32;
    for &idx in &indices {
        sum += soa_particles.position_x[idx];
    }
    let soa_duration = soa_start.elapsed();
    std::hint::black_box(sum);
    
    println!("Random Access Pattern ({} accesses)", indices.len());
    println!("AOS Duration: {:?}", aos_duration);
    println!("SOA Duration: {:?}", soa_duration);
    println!("SOA Speedup: {:.2}x", aos_duration.as_secs_f64() / soa_duration.as_secs_f64());
}

fn main() {
    println!("Hearth Engine SOA Performance Benchmarks");
    println!("=======================================");
    
    // Position-only access benchmarks
    benchmark_position_only_access(10_000, 1000);
    benchmark_position_only_access(100_000, 100);
    
    // Full physics update benchmarks
    benchmark_physics_update(10_000, 100);
    benchmark_physics_update(100_000, 10);
    
    // ECS physics system benchmarks
    benchmark_ecs_physics(10_000, 100);
    benchmark_ecs_physics(50_000, 20);
    
    // Memory locality benchmarks
    benchmark_memory_locality(10_000);
    benchmark_memory_locality(100_000);
    
    println!("\n=== Summary ===");
    println!("SOA provides significant performance improvements due to:");
    println!("1. Better cache locality for component-wise operations");
    println!("2. SIMD-friendly data layout");
    println!("3. Reduced memory bandwidth requirements");
    println!("4. Elimination of object allocation overhead");
    println!("\nThese improvements demonstrate the cache efficiency gains");
    println!("achieved through data-oriented programming patterns.");
}