use hearth_engine::world::{WorldGenerator, DefaultWorldGenerator, BlockId, ChunkPos};

fn main() {
    println!("Testing multiple Y-level chunk generation for floating blocks...\n");
    
    let generator = DefaultWorldGenerator::new(
        12345,
        BlockId(1), // grass
        BlockId(2), // dirt
        BlockId(3), // stone
        BlockId(4), // water
        BlockId(5), // sand
    );
    
    let chunk_size = 32;
    
    // Test the spawn area at origin
    let test_chunks = [
        ChunkPos { x: 0, y: 0, z: 0 },   // Y=0-31 (underground)
        ChunkPos { x: 0, y: 1, z: 0 },   // Y=32-63 (underground)
        ChunkPos { x: 0, y: 2, z: 0 },   // Y=64-95 (surface level)
        ChunkPos { x: 0, y: 3, z: 0 },   // Y=96-127 (air level - should be mostly empty)
        ChunkPos { x: 0, y: 4, z: 0 },   // Y=128-159 (high air - should be completely empty)
        ChunkPos { x: 0, y: 5, z: 0 },   // Y=160-191 (very high air - should be completely empty)
    ];
    
    for chunk_pos in test_chunks {
        let chunk = generator.generate_chunk(chunk_pos, chunk_size);
        let world_y_start = chunk_pos.y * chunk_size as i32;
        let world_y_end = world_y_start + chunk_size as i32 - 1;
        
        println!("Chunk {:?} (world Y {}-{}):", chunk_pos, world_y_start, world_y_end);
        
        // Count non-air blocks by type
        let mut block_counts = std::collections::HashMap::new();
        let mut air_count = 0;
        
        for y in 0..chunk_size {
            for z in 0..chunk_size {
                for x in 0..chunk_size {
                    let block = chunk.get_block(x, y, z);
                    if block == BlockId::AIR {
                        air_count += 1;
                    } else {
                        *block_counts.entry(block).or_insert(0) += 1;
                    }
                }
            }
        }
        
        let total_blocks = chunk_size * chunk_size * chunk_size;
        
        if block_counts.is_empty() {
            println!("  ✓ All {} blocks are AIR (expected for high Y chunks)", total_blocks);
        } else {
            println!("  Block distribution:");
            for (block_id, count) in &block_counts {
                let block_name = match block_id {
                    BlockId(1) => "GRASS",
                    BlockId(2) => "DIRT",
                    BlockId(3) => "STONE", 
                    BlockId(4) => "WATER",
                    BlockId(5) => "SAND",
                    _ => "UNKNOWN",
                };
                let percentage = (*count as f32 / total_blocks as f32) * 100.0;
                println!("    {}: {} blocks ({:.1}%)", block_name, count, percentage);
            }
            println!("    AIR: {} blocks ({:.1}%)", air_count, (air_count as f32 / total_blocks as f32) * 100.0);
            
            // Flag unexpected blocks in high Y chunks
            if chunk_pos.y >= 3 && !block_counts.is_empty() {
                println!("  ⚠ WARNING: Found {} non-air blocks at high Y level (Y={}+)!", 
                        total_blocks - air_count, world_y_start);
                println!("    This could cause floating blocks!");
            }
        }
        
        // Sample a few positions to show specific block locations
        if !block_counts.is_empty() {
            println!("  Sample block positions:");
            let mut sample_count = 0;
            for y in 0..chunk_size {
                for z in 0..chunk_size {
                    for x in 0..chunk_size {
                        let block = chunk.get_block(x, y, z);
                        if block != BlockId::AIR && sample_count < 5 {
                            let world_x = x as i32;
                            let world_y = world_y_start + y as i32;
                            let world_z = z as i32;
                            let block_name = match block {
                                BlockId(1) => "GRASS",
                                BlockId(2) => "DIRT",
                                BlockId(3) => "STONE",
                                BlockId(4) => "WATER", 
                                BlockId(5) => "SAND",
                                _ => "UNKNOWN",
                            };
                            println!("    World ({}, {}, {}): {}", world_x, world_y, world_z, block_name);
                            sample_count += 1;
                        }
                    }
                }
            }
        }
        
        println!();
    }
    
    // Test a few more chunks around the spawn to see if there's a pattern
    println!("Testing neighboring chunks for floating block patterns...");
    let neighbor_chunks = [
        ChunkPos { x: 1, y: 3, z: 0 },   // High Y neighboring chunk
        ChunkPos { x: -1, y: 3, z: 0 },  // High Y neighboring chunk
        ChunkPos { x: 0, y: 3, z: 1 },   // High Y neighboring chunk
        ChunkPos { x: 0, y: 3, z: -1 },  // High Y neighboring chunk
    ];
    
    for chunk_pos in neighbor_chunks {
        let chunk = generator.generate_chunk(chunk_pos, chunk_size);
        let world_y_start = chunk_pos.y * chunk_size as i32;
        
        let mut non_air_blocks = 0;
        for y in 0..chunk_size {
            for z in 0..chunk_size {
                for x in 0..chunk_size {
                    if chunk.get_block(x, y, z) != BlockId::AIR {
                        non_air_blocks += 1;
                    }
                }
            }
        }
        
        if non_air_blocks > 0 {
            println!("  ⚠ Chunk {:?} (Y={}+): {} non-air blocks (FLOATING BLOCKS!)", 
                    chunk_pos, world_y_start, non_air_blocks);
        } else {
            println!("  ✓ Chunk {:?} (Y={}+): All air (correct)", chunk_pos, world_y_start);
        }
    }
}