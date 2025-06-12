use earth_engine::world::{WorldGenerator, DefaultWorldGenerator, BlockId, ChunkPos};

fn main() {
    println!("Testing terrain generation at chunk boundaries...\n");
    
    let generator = DefaultWorldGenerator::new(
        12345,  // seed
        BlockId(1), // grass
        BlockId(2), // dirt
        BlockId(3), // stone
        BlockId(4), // water
        BlockId(5), // sand
    );
    
    let chunk_size = 32;
    
    // Test positions that might cause issues
    let test_positions = [
        (0.0, 0.0, "Origin"),
        (0.5, 0.5, "Offset by 0.5"),
        (16.0, 16.0, "Middle of chunk"),
        (31.0, 31.0, "Near chunk boundary"),
        (32.0, 32.0, "Chunk boundary"),
        (33.0, 33.0, "Just past chunk boundary"),
    ];
    
    for (x, z, label) in test_positions {
        println!("=== {} at ({}, {}) ===", label, x, z);
        
        // Get calculated heights
        let surface_height = generator.get_surface_height(x, z);
        let spawn_height = generator.find_safe_spawn_height(x, z);
        
        println!("Calculated surface height: {}", surface_height);
        println!("Calculated spawn height: {}", spawn_height);
        
        // Check what chunk this would be in
        let chunk_x = (x as i32) / chunk_size as i32;
        let chunk_y = (spawn_height as i32) / chunk_size as i32;
        let chunk_z = (z as i32) / chunk_size as i32;
        
        println!("Would be in chunk: ({}, {}, {})", chunk_x, chunk_y, chunk_z);
        
        // Generate the chunk and check what's actually there
        let chunk = generator.generate_chunk(ChunkPos { x: chunk_x, y: chunk_y, z: chunk_z }, chunk_size);
        
        let local_x = ((x as i32) % chunk_size as i32) as u32;
        let local_z = ((z as i32) % chunk_size as i32) as u32;
        
        println!("Local coordinates in chunk: ({}, {})", local_x, local_z);
        
        // Check blocks around spawn height
        println!("Blocks at this position:");
        for y_offset in -3..=3 {
            let world_y = spawn_height as i32 + y_offset;
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
                
                let marker = if world_y == spawn_height as i32 { " <- Spawn" } else { "" };
                println!("  y={}: {}{}", world_y, block_type, marker);
            }
        }
        
        // Also check the actual terrain height at integer coordinates
        let int_x = x.floor();
        let int_z = z.floor();
        let int_height = generator.get_surface_height(int_x, int_z);
        if (int_x != x || int_z != z) {
            println!("Height at integer coords ({}, {}): {}", int_x, int_z, int_height);
        }
        
        println!();
    }
}