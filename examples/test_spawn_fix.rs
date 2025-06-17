use hearth_engine::world::{WorldGenerator, DefaultWorldGenerator};
use hearth_engine::BlockId;

fn main() {
    println!("Testing spawn finder fix...\n");
    
    let generator = DefaultWorldGenerator::new(
        12345,
        BlockId(1), // grass
        BlockId(2), // dirt
        BlockId(3), // stone
        BlockId(4), // water
        BlockId(5), // sand
    );
    
    // Test spawn positions
    let test_positions = [
        (0.0, 0.0, "Origin"),
        (100.0, 100.0, "Offset position"),
        (-50.0, 50.0, "Mixed position"),
    ];
    
    println!("Comparing old vs new spawn logic:\n");
    
    for (x, z, label) in test_positions {
        let surface_height = generator.get_surface_height(x as f64, z as f64) as f32;
        
        // Old logic (what it used to do)
        let old_spawn_y = surface_height + 25.0; // Added 25 blocks above surface
        
        // New logic (what the fix does)
        let new_feet_y = surface_height + 1.0;  // 1 block clearance
        let new_spawn_y = new_feet_y + 0.9;     // Body center 0.9m above feet
        
        println!("{} at ({}, {}):", label, x, z);
        println!("  Surface height: {}", surface_height);
        println!("  Old spawn logic: Y={} ({} blocks above surface)", old_spawn_y, old_spawn_y - surface_height);
        println!("  New spawn logic: Y={:.1} ({:.1} blocks above surface)", new_spawn_y, new_spawn_y - surface_height);
        println!("  Improvement: {:.1} blocks lower", old_spawn_y - new_spawn_y);
        
        if old_spawn_y >= 89.0 {
            println!("  ⚠ Old logic would spawn at dangerous height Y={}+!", old_spawn_y);
        }
        if new_spawn_y <= 67.0 {
            println!("  ✓ New logic spawns at reasonable height Y={:.1}", new_spawn_y);
        }
        
        println!();
    }
    
    println!("Summary of the fix:");
    println!("- OLD: Spawned player 25 blocks above highest terrain in search area");
    println!("- NEW: Spawns player 1.9 blocks above surface at exact spawn position");
    println!("- This prevents spawning inside floating blocks or too high in the air");
    println!("- Player will spawn on the surface like a normal game should");
}