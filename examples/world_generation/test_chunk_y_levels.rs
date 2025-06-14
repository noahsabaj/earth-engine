use earth_engine::world::{WorldGenerator, DefaultWorldGenerator, BlockId, ChunkPos};

fn main() {
    println!("Testing chunk generation at different Y levels...\n");
    
    let generator = DefaultWorldGenerator::new(
        12345,
        BlockId(1), // grass
        BlockId(2), // dirt
        BlockId(3), // stone
        BlockId(4), // water
        BlockId(5), // sand
    );
    
    let chunk_size = 32;
    let spawn_x = 0;
    let spawn_z = 0;
    
    // Test chunks at different Y levels around spawn
    for chunk_y in 0..6 { // Y levels 0-160 (32*5)
        let chunk_pos = ChunkPos { x: 0, y: chunk_y, z: 0 };
        let chunk = generator.generate_chunk(chunk_pos, chunk_size);
        
        let world_y_start = chunk_y * chunk_size as i32;
        let world_y_end = world_y_start + chunk_size as i32 - 1;
        
        println!("Chunk Y={} (world Y {}-{}):", chunk_y, world_y_start, world_y_end);
        
        // Count non-air blocks in this chunk
        let mut block_counts = std::collections::HashMap::new();
        let mut highest_block_y = None;
        
        for y in 0..chunk_size {
            for z in 0..chunk_size {
                for x in 0..chunk_size {
                    let block = chunk.get_block(x, y, z);
                    if block != BlockId::AIR {
                        *block_counts.entry(block).or_insert(0) += 1;
                        let world_y = world_y_start + y as i32;
                        if highest_block_y.is_none() || world_y > highest_block_y.unwrap() {
                            highest_block_y = Some(world_y);
                        }
                    }
                }
            }
        }
        
        if block_counts.is_empty() {
            println!("  All AIR");
        } else {
            println!("  Block counts:");
            for (block_id, count) in block_counts {
                let block_name = match block_id {
                    BlockId(1) => "GRASS",
                    BlockId(2) => "DIRT",
                    BlockId(3) => "STONE", 
                    BlockId(4) => "WATER",
                    BlockId(5) => "SAND",
                    _ => "UNKNOWN",
                };
                println!("    {}: {} blocks", block_name, count);
            }
            if let Some(highest_y) = highest_block_y {
                println!("  Highest block at world Y: {}", highest_y);
            }
        }
        
        // Check specific spawn column (0,0) in this chunk
        let local_x = (spawn_x % chunk_size as i32) as u32;
        let local_z = (spawn_z % chunk_size as i32) as u32;
        
        let mut spawn_column_blocks = Vec::new();
        for y in 0..chunk_size {
            let block = chunk.get_block(local_x, y, local_z);
            if block != BlockId::AIR {
                let world_y = world_y_start + y as i32;
                spawn_column_blocks.push((world_y, block));
            }
        }
        
        if !spawn_column_blocks.is_empty() {
            println!("  Spawn column (0,0) blocks:");
            for (world_y, block_id) in spawn_column_blocks {
                let block_name = match block_id {
                    BlockId(1) => "GRASS",
                    BlockId(2) => "DIRT", 
                    BlockId(3) => "STONE",
                    BlockId(4) => "WATER",
                    BlockId(5) => "SAND",
                    _ => "UNKNOWN",
                };
                println!("    Y={}: {}", world_y, block_name);
            }
        }
        
        println!();
    }
}