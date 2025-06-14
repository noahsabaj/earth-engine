use earth_engine::world::{WorldGenerator, DefaultWorldGenerator, SpawnFinder, World, WorldConfig};
use earth_engine::BlockId;

fn main() {
    println!("Testing new spawn finder logic...\n");
    
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        BlockId(1), // grass
        BlockId(2), // dirt
        BlockId(3), // stone
        BlockId(4), // water
        BlockId(5), // sand
    ));
    
    let config = WorldConfig::default();
    let world = World::new(generator, config);
    
    // Test spawn finder at various positions
    let test_positions = [
        (0.0, 0.0, "Origin"),
        (100.0, 100.0, "Offset position"),
        (-50.0, 50.0, "Mixed position"),
    ];
    
    for (x, z, label) in test_positions {
        println!("Testing {} at ({}, {}):", label, x, z);
        
        // Get the actual surface height from world
        let surface_height = world.get_surface_height(x as f64, z as f64);
        println!("  Surface height from world: {}", surface_height);
        
        // Use spawn finder to find safe spawn
        match SpawnFinder::find_safe_spawn(&world, x, z, 10) {
            Ok(spawn_pos) => {
                println!("  Spawn finder result: {:?}", spawn_pos);
                
                let feet_y = spawn_pos.y - 0.9; // Body center to feet
                let clearance = feet_y - surface_height as f32;
                
                println!("  Player feet will be at Y={:.1}", feet_y);
                println!("  Clearance above surface: {:.1} blocks", clearance);
                
                if clearance < 0.5 {
                    println!("  ⚠ WARNING: Player might be too close to surface!");
                } else if clearance > 5.0 {
                    println!("  ⚠ WARNING: Player spawning too high above surface!");
                } else {
                    println!("  ✓ Good spawn height");
                }
            }
            Err(e) => {
                println!("  ❌ Spawn finder failed: {}", e);
            }
        }
        
        println!();
    }
    
    // Test what the old logic would have done vs new logic
    println!("Comparing old vs new spawn logic:");
    let test_x = 0.0;
    let test_z = 0.0;
    let surface_height = world.get_surface_height(test_x as f64, test_z as f64) as f32;
    
    // Old logic (what it used to do)
    let old_spawn_y = surface_height + 25.0; // Added 25 blocks above surface
    
    // New logic
    let new_feet_y = surface_height + 1.0;  // 1 block clearance
    let new_spawn_y = new_feet_y + 0.9;     // Body center 0.9m above feet
    
    println!("  Surface height: {}", surface_height);
    println!("  Old spawn logic: Y={} ({} blocks above surface)", old_spawn_y, old_spawn_y - surface_height);
    println!("  New spawn logic: Y={:.1} ({:.1} blocks above surface)", new_spawn_y, new_spawn_y - surface_height);
    println!("  Improvement: {:.1} blocks lower", old_spawn_y - new_spawn_y);
}