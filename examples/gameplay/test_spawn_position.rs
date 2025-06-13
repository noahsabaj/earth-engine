use earth_engine::world::{WorldGenerator, DefaultWorldGenerator, BlockId};

fn main() {
    println!("Testing spawn position calculation...\n");
    
    // Create a world generator with test block IDs
    let generator = DefaultWorldGenerator::new(
        12345,  // seed
        BlockId(1), // grass
        BlockId(2), // dirt
        BlockId(3), // stone
        BlockId(4), // water
        BlockId(5), // sand
    );
    
    // Test spawn positions at various locations
    let test_positions = [
        (0.0, 0.0, "Origin"),
        (100.0, 100.0, "Offset position"),
        (-100.0, -100.0, "Negative position"),
        (1000.0, 0.0, "Far X"),
        (0.0, 1000.0, "Far Z"),
    ];
    
    for (x, z, label) in test_positions {
        let terrain_height = generator.get_surface_height(x, z);
        let spawn_height = generator.find_safe_spawn_height(x, z);
        
        println!("{} at ({}, {}):", label, x, z);
        println!("  Terrain height: {}", terrain_height);
        println!("  Safe spawn height: {}", spawn_height);
        println!("  Clearance above terrain: {:.1}", spawn_height - terrain_height as f32);
        println!();
    }
    
    // Test edge case - very high terrain
    println!("Testing edge cases...");
    
    // Manually check a position that might have high terrain
    let high_terrain_x = 5000.0;
    let high_terrain_z = 5000.0;
    let high_terrain_height = generator.get_surface_height(high_terrain_x, high_terrain_z);
    let high_spawn_height = generator.find_safe_spawn_height(high_terrain_x, high_terrain_z);
    
    println!("High terrain position ({}, {}):", high_terrain_x, high_terrain_z);
    println!("  Terrain height: {}", high_terrain_height);
    println!("  Safe spawn height: {}", high_spawn_height);
    println!("  Spawn height clamped: {}", high_spawn_height <= 250.0);
}