use noise::{NoiseFn, Perlin};

fn main() {
    println!("Testing Perlin noise at various positions...\n");
    
    let perlin = Perlin::new(12345);
    let scale = 0.01;
    
    println!("Testing at scale {} around origin:", scale);
    for x in -5..=5 {
        for z in -5..=5 {
            let fx = x as f64;
            let fz = z as f64;
            let value = perlin.get([fx * scale, fz * scale]);
            print!("{:6.3} ", value);
        }
        println!();
    }
    
    println!("\nTesting with offset (0.5, 0.5):");
    for x in -5..=5 {
        for z in -5..=5 {
            let fx = x as f64 + 0.5;
            let fz = z as f64 + 0.5;
            let value = perlin.get([fx * scale, fz * scale]);
            print!("{:6.3} ", value);
        }
        println!();
    }
    
    // Test what happens with actual world coordinates
    println!("\nTesting actual terrain height calculation:");
    let terrain_gen = earth_engine::world::generation::terrain::TerrainGenerator::new(12345);
    
    for x in -10..=10 {
        for z in -10..=10 {
            let height = terrain_gen.get_height(x as f64, z as f64);
            print!("{:3} ", height);
        }
        println!();
    }
}