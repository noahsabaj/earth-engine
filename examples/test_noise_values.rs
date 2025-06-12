use noise::{NoiseFn, Perlin};

fn main() {
    println!("Testing Perlin noise values...\n");
    
    let perlin = Perlin::new(12345);
    
    // Test at various scales
    let scales = [0.01, 0.05, 0.1, 1.0];
    let positions = [(0.0, 0.0), (10.0, 10.0), (100.0, 100.0), (1000.0, 1000.0)];
    
    for scale in scales {
        println!("Scale: {}", scale);
        for (x, z) in positions {
            let value = perlin.get([x * scale, z * scale]);
            println!("  Position ({}, {}): raw={:.6}, scaled by 32={:.2}", 
                     x, z, value, value * 32.0);
        }
        println!();
    }
    
    // Simulate the actual terrain calculation
    println!("Simulating terrain calculation at (0, 0):");
    let height_noise = Perlin::new(12345);
    let detail_noise = Perlin::new(12346);
    
    let world_x = 0.0;
    let world_z = 0.0;
    
    let height1 = height_noise.get([world_x * 0.01, world_z * 0.01]) * 32.0;
    let height2 = detail_noise.get([world_x * 0.05, world_z * 0.05]) * 8.0;
    let height3 = height_noise.get([world_x * 0.1, world_z * 0.1]) * 2.0;
    
    println!("  height1 (scale 0.01): {:.2}", height1);
    println!("  height2 (scale 0.05): {:.2}", height2);
    println!("  height3 (scale 0.1): {:.2}", height3);
    println!("  combined: {:.2}", height1 + height2 + height3);
    println!("  final height: {}", 64 + (height1 + height2 + height3) as i32);
}