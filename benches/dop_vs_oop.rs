// Hearth Engine DOP vs OOP Performance Benchmarks
// Sprint 37: DOP Reality Check
// 
// This benchmark compares data-oriented programming patterns
// against object-oriented patterns to validate DOP performance claims.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use glam::Vec3;
use std::time::Duration;

const ENTITY_COUNTS: &[usize] = &[1000, 10000, 100000];

// ========================================
// OOP Pattern (Array of Structs)
// ========================================

#[derive(Clone)]
struct EntityOOP {
    position: Vec3,
    velocity: Vec3,
    health: f32,
    energy: f32,
}

impl EntityOOP {
    fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::new(1.0, 0.0, 0.0),
            health: 100.0,
            energy: 50.0,
        }
    }

    fn update(&mut self, dt: f32) {
        self.position += self.velocity * dt;
        self.energy -= 0.1 * dt;
        if self.energy < 0.0 {
            self.health -= 1.0;
            self.energy = 0.0;
        }
    }

    fn apply_damage(&mut self, damage: f32) {
        self.health -= damage;
        if self.health < 0.0 {
            self.health = 0.0;
        }
    }
}

struct EntitySystemOOP {
    entities: Vec<EntityOOP>,
}

impl EntitySystemOOP {
    fn new(count: usize) -> Self {
        Self {
            entities: (0..count).map(|_| EntityOOP::new()).collect(),
        }
    }

    fn update_all(&mut self, dt: f32) {
        for entity in &mut self.entities {
            entity.update(dt);
        }
    }

    fn apply_area_damage(&mut self, center: Vec3, radius: f32, damage: f32) {
        for entity in &mut self.entities {
            let distance = entity.position.distance(center);
            if distance < radius {
                entity.apply_damage(damage);
            }
        }
    }
}

// ========================================
// DOP Pattern (Structure of Arrays)
// ========================================

struct EntityDataDOP {
    count: usize,
    positions_x: Vec<f32>,
    positions_y: Vec<f32>,
    positions_z: Vec<f32>,
    velocities_x: Vec<f32>,
    velocities_y: Vec<f32>,
    velocities_z: Vec<f32>,
    health: Vec<f32>,
    energy: Vec<f32>,
}

impl EntityDataDOP {
    fn new(count: usize) -> Self {
        Self {
            count,
            positions_x: vec![0.0; count],
            positions_y: vec![0.0; count],
            positions_z: vec![0.0; count],
            velocities_x: vec![1.0; count],
            velocities_y: vec![0.0; count],
            velocities_z: vec![0.0; count],
            health: vec![100.0; count],
            energy: vec![50.0; count],
        }
    }
}

// DOP kernel functions (stateless)
fn update_positions(data: &mut EntityDataDOP, dt: f32) {
    for i in 0..data.count {
        data.positions_x[i] += data.velocities_x[i] * dt;
        data.positions_y[i] += data.velocities_y[i] * dt;
        data.positions_z[i] += data.velocities_z[i] * dt;
    }
}

fn update_energy_and_health(data: &mut EntityDataDOP, dt: f32) {
    for i in 0..data.count {
        data.energy[i] -= 0.1 * dt;
        if data.energy[i] < 0.0 {
            data.health[i] -= 1.0;
            data.energy[i] = 0.0;
        }
    }
}

fn apply_area_damage_dop(
    data: &mut EntityDataDOP,
    center_x: f32,
    center_y: f32,
    center_z: f32,
    radius: f32,
    damage: f32,
) {
    let radius_sq = radius * radius;
    
    for i in 0..data.count {
        let dx = data.positions_x[i] - center_x;
        let dy = data.positions_y[i] - center_y;
        let dz = data.positions_z[i] - center_z;
        let distance_sq = dx * dx + dy * dy + dz * dz;
        
        if distance_sq < radius_sq {
            data.health[i] -= damage;
            if data.health[i] < 0.0 {
                data.health[i] = 0.0;
            }
        }
    }
}

fn update_all_dop(data: &mut EntityDataDOP, dt: f32) {
    update_positions(data, dt);
    update_energy_and_health(data, dt);
}

// ========================================
// Cache-Friendly DOP Pattern (SIMD-ready)
// ========================================

// This version uses more cache-friendly access patterns
fn update_positions_simd_friendly(data: &mut EntityDataDOP, dt: f32) {
    // Process positions in chunks for better cache utilization
    const CHUNK_SIZE: usize = 64; // Cache line friendly
    
    let chunks = data.count / CHUNK_SIZE;
    let remainder = data.count % CHUNK_SIZE;
    
    // Process full chunks
    for chunk in 0..chunks {
        let start = chunk * CHUNK_SIZE;
        let end = start + CHUNK_SIZE;
        
        // These loops can be vectorized by the compiler
        for i in start..end {
            data.positions_x[i] += data.velocities_x[i] * dt;
        }
        for i in start..end {
            data.positions_y[i] += data.velocities_y[i] * dt;
        }
        for i in start..end {
            data.positions_z[i] += data.velocities_z[i] * dt;
        }
    }
    
    // Process remainder
    let start = chunks * CHUNK_SIZE;
    for i in start..(start + remainder) {
        data.positions_x[i] += data.velocities_x[i] * dt;
        data.positions_y[i] += data.velocities_y[i] * dt;
        data.positions_z[i] += data.velocities_z[i] * dt;
    }
}

// ========================================
// Benchmarks
// ========================================

fn bench_entity_update_oop(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_update_oop");
    
    for &count in ENTITY_COUNTS {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                let mut system = EntitySystemOOP::new(count);
                b.iter(|| {
                    system.update_all(black_box(0.016));
                });
            },
        );
    }
    
    group.finish();
}

fn bench_entity_update_dop(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_update_dop");
    
    for &count in ENTITY_COUNTS {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                let mut data = EntityDataDOP::new(count);
                b.iter(|| {
                    update_all_dop(&mut data, black_box(0.016));
                });
            },
        );
    }
    
    group.finish();
}

fn bench_entity_update_dop_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_update_dop_simd");
    
    for &count in ENTITY_COUNTS {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                let mut data = EntityDataDOP::new(count);
                b.iter(|| {
                    update_positions_simd_friendly(&mut data, black_box(0.016));
                    update_energy_and_health(&mut data, black_box(0.016));
                });
            },
        );
    }
    
    group.finish();
}

fn bench_area_damage_oop(c: &mut Criterion) {
    let mut group = c.benchmark_group("area_damage_oop");
    
    for &count in ENTITY_COUNTS {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                let mut system = EntitySystemOOP::new(count);
                b.iter(|| {
                    system.apply_area_damage(
                        black_box(Vec3::new(50.0, 50.0, 50.0)),
                        black_box(25.0),
                        black_box(10.0),
                    );
                });
            },
        );
    }
    
    group.finish();
}

fn bench_area_damage_dop(c: &mut Criterion) {
    let mut group = c.benchmark_group("area_damage_dop");
    
    for &count in ENTITY_COUNTS {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                let mut data = EntityDataDOP::new(count);
                b.iter(|| {
                    apply_area_damage_dop(
                        &mut data,
                        black_box(50.0),
                        black_box(50.0),
                        black_box(50.0),
                        black_box(25.0),
                        black_box(10.0),
                    );
                });
            },
        );
    }
    
    group.finish();
}

// ========================================
// Memory Layout Benchmarks
// ========================================

fn bench_memory_layout_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_layout");
    group.measurement_time(Duration::from_secs(10));
    
    const COUNT: usize = 100000;
    
    // AoS - Array of Structs (cache-hostile)
    group.bench_function("aos_position_only", |b| {
        let entities = vec![EntityOOP::new(); COUNT];
        
        b.iter(|| {
            let mut sum = 0.0f32;
            for entity in &entities {
                sum += entity.position.x + entity.position.y + entity.position.z;
            }
            black_box(sum);
        });
    });
    
    // SoA - Structure of Arrays (cache-friendly)
    group.bench_function("soa_position_only", |b| {
        let data = EntityDataDOP::new(COUNT);
        
        b.iter(|| {
            let mut sum = 0.0f32;
            for i in 0..data.count {
                sum += data.positions_x[i] + data.positions_y[i] + data.positions_z[i];
            }
            black_box(sum);
        });
    });
    
    // SoA with separate loops (optimal cache usage)
    group.bench_function("soa_position_separate_loops", |b| {
        let data = EntityDataDOP::new(COUNT);
        
        b.iter(|| {
            let mut sum = 0.0f32;
            
            // Process each component separately for maximum cache efficiency
            for i in 0..data.count {
                sum += data.positions_x[i];
            }
            for i in 0..data.count {
                sum += data.positions_y[i];
            }
            for i in 0..data.count {
                sum += data.positions_z[i];
            }
            
            black_box(sum);
        });
    });
    
    group.finish();
}

// ========================================
// Cache Efficiency Test
// ========================================

fn bench_cache_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_efficiency");
    
    const COUNT: usize = 1000000;
    
    // Test random access (cache-hostile)
    group.bench_function("random_access", |b| {
        let data = EntityDataDOP::new(COUNT);
        let indices: Vec<usize> = (0..COUNT).map(|i| (i * 7919) % COUNT).collect();
        
        b.iter(|| {
            let mut sum = 0.0f32;
            for &i in &indices {
                sum += data.positions_x[i];
            }
            black_box(sum);
        });
    });
    
    // Test sequential access (cache-friendly)
    group.bench_function("sequential_access", |b| {
        let data = EntityDataDOP::new(COUNT);
        
        b.iter(|| {
            let mut sum = 0.0f32;
            for i in 0..data.count {
                sum += data.positions_x[i];
            }
            black_box(sum);
        });
    });
    
    group.finish();
}

// ========================================
// Registration
// ========================================

criterion_group!(
    benches,
    bench_entity_update_oop,
    bench_entity_update_dop,
    bench_entity_update_dop_simd,
    bench_area_damage_oop,
    bench_area_damage_dop,
    bench_memory_layout_comparison,
    bench_cache_efficiency
);

criterion_main!(benches);

// ========================================
// Analysis and Results Documentation
// ========================================

// Expected Performance Results:
// 
// 1. **Entity Update**:
//    - DOP should be 2-3x faster than OOP due to cache efficiency
//    - SIMD-friendly DOP should be 3-5x faster with compiler vectorization
// 
// 2. **Area Damage**:
//    - DOP should be 5-10x faster due to:
//      - Better cache locality for position data
//      - Reduced pointer chasing
//      - SIMD-friendly distance calculations
// 
// 3. **Memory Layout**:
//    - SoA separate loops should be 10-20x faster than AoS
//    - Demonstrates the power of cache-friendly access patterns
// 
// 4. **Cache Efficiency**:
//    - Sequential access should be 100x+ faster than random access
//    - Shows why data layout matters more than algorithmic complexity
// 
// These benchmarks validate the DOP approach and provide concrete
// evidence for why Hearth Engine adopts these patterns.