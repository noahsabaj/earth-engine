use earth_engine::world::{WorldGenerator, DefaultWorldGenerator, BlockId, ChunkPos};
use earth_engine::world::generation::terrain::TerrainGenerator;

fn main() {
    println!("Testing terrain generation at spawn position...\n");
    
    // Create a world generator with test block IDs
    // Try different seeds to find one with more terrain variation
    let seeds = [12345, 54321, 99999, 11111, 7777];
    
    for seed in seeds {
        println!("=== Testing with seed {} ===", seed);
        let generator = DefaultWorldGenerator::new(
            seed,
            BlockId(1), // grass
            BlockId(2), // dirt
            BlockId(3), // stone
            BlockId(4), // water
            BlockId(5), // sand
        );
        
        // Quick check of terrain heights in a grid
        let mut max_height = 0;
        let mut min_height = 200;
        for x in -50..=50 {
            for z in -50..=50 {
                let h = generator.get_surface_height(x as f64, z as f64);
                max_height = max_height.max(h);
                min_height = min_height.min(h);
            }
        }
        println!("Height range: {} to {}", min_height, max_height);
        
        if max_height > 70 {
            println!("Found varied terrain with seed {}!", seed);
            // Do detailed test with this seed
            test_spawn_with_generator(&generator);
            break;
        }
    }
}

fn test_spawn_with_generator(generator: &DefaultWorldGenerator) {
    
    // Test spawn position
    let spawn_x = 0.0;
    let spawn_z = 0.0;
    
    // Get the calculated heights
    let terrain_height = generator.get_surface_height(spawn_x, spawn_z);
    let spawn_height = generator.find_safe_spawn_height(spawn_x, spawn_z);
    
    println!("Spawn position ({}, {}):", spawn_x, spawn_z);
    println!("  Calculated terrain height: {}", terrain_height);
    println!("  Calculated spawn height: {}", spawn_height);
    println!();
    
    // Now generate the actual chunk and see what blocks are there
    let chunk_size = 32;
    let chunk_x = (spawn_x as i32) / chunk_size as i32;
    let chunk_y = (spawn_height as i32) / chunk_size as i32;
    let chunk_z = (spawn_z as i32) / chunk_size as i32;
    
    let chunk_pos = ChunkPos { x: chunk_x, y: chunk_y, z: chunk_z };
    println!("Generating chunk at position: {:?}", chunk_pos);
    
    let chunk = generator.generate_chunk(chunk_pos, chunk_size);
    
    // Check what blocks are at the spawn position
    let local_x = ((spawn_x as i32) % chunk_size as i32) as u32;
    let local_z = ((spawn_z as i32) % chunk_size as i32) as u32;
    
    println!("\nBlocks at spawn column (x={}, z={}):", spawn_x, spawn_z);
    println!("Local chunk coordinates: ({}, {})", local_x, local_z);
    
    // Check blocks from y=60 to y=80
    for world_y in 60..=80 {
        let local_y = (world_y - chunk_y * chunk_size as i32) as u32;
        
        if local_y < chunk_size {
            let block = chunk.get_block(local_x, local_y, local_z);
            let block_type = match block {
                BlockId(0) => "AIR",
                BlockId(1) => "GRASS",
                BlockId(2) => "DIRT",
                BlockId(3) => "STONE",
                BlockId(4) => "WATER",
                BlockId(5) => "SAND",
                _ => "UNKNOWN",
            };
            
            let marker = if world_y == terrain_height { " <- Calculated surface" } 
                        else if world_y == spawn_height as i32 { " <- Spawn position" }
                        else { "" };
            
            println!("  y={}: {} (BlockId: {}){}", world_y, block_type, block.0, marker);
        }
    }
    
    // Also check neighboring positions to see if there's variation
    println!("\nChecking neighboring positions for height variation:");
    for dx in -2..=2 {
        for dz in -2..=2 {
            let x = spawn_x + dx as f64;
            let z = spawn_z + dz as f64;
            let h = generator.get_surface_height(x, z);
            println!("  ({:3}, {:3}): height = {}", x, z, h);
        }
    }
}