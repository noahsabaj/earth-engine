use hearth_engine::world::{WorldGenerator, DefaultWorldGenerator, BlockId, ChunkPos};
use noise::{NoiseFn, Perlin};

fn main() {
    println!("=== Tracing terrain generation at origin (0, 0) ===\n");
    
    // Create the same Perlin noise generators as TerrainGenerator
    let seed = 12345u32;
    let height_noise = Perlin::new(seed);
    let detail_noise = Perlin::new(seed.wrapping_add(1));
    
    // Test position (0, 0)
    let world_x = 0.0;
    let world_z = 0.0;
    
    // Trace the noise calculations step by step
    let scale1 = 0.01;  // Large features
    let scale2 = 0.05;  // Medium features  
    let scale3 = 0.1;   // Small features
    
    println!("Input position: ({}, {})", world_x, world_z);
    println!("\nNoise sampling:");
    
    // Sample 1: Large features
    let sample_x1 = world_x * scale1;
    let sample_z1 = world_z * scale1;
    let noise_val1 = height_noise.get([sample_x1, sample_z1]);
    let height1 = noise_val1 * 32.0;
    println!("  Large features (scale={}): sample at [{}, {}] = {} * 32.0 = {}", 
             scale1, sample_x1, sample_z1, noise_val1, height1);
    
    // Sample 2: Medium features
    let sample_x2 = world_x * scale2;
    let sample_z2 = world_z * scale2;
    let noise_val2 = detail_noise.get([sample_x2, sample_z2]);
    let height2 = noise_val2 * 8.0;
    println!("  Medium features (scale={}): sample at [{}, {}] = {} * 8.0 = {}", 
             scale2, sample_x2, sample_z2, noise_val2, height2);
    
    // Sample 3: Small features
    let sample_x3 = world_x * scale3;
    let sample_z3 = world_z * scale3;
    let noise_val3 = height_noise.get([sample_x3, sample_z3]);
    let height3 = noise_val3 * 2.0;
    println!("  Small features (scale={}): sample at [{}, {}] = {} * 2.0 = {}", 
             scale3, sample_x3, sample_z3, noise_val3, height3);
    
    // Combine
    let combined_height = height1 + height2 + height3;
    println!("\nCombined height: {} + {} + {} = {}", height1, height2, height3, combined_height);
    
    // Final calculation
    let base_height = 64;
    let final_height = base_height + combined_height as i32;
    let clamped_height = final_height.clamp(10, 200);
    
    println!("\nFinal calculation:");
    println!("  Base height: {}", base_height);
    println!("  Final height: {} + {} = {}", base_height, combined_height as i32, final_height);
    println!("  Clamped height: {}", clamped_height);
    
    // Now trace what blocks would be placed in a chunk containing (0,0)
    println!("\n=== Tracing block placement in chunk containing (0,0) ===\n");
    
    let generator = DefaultWorldGenerator::new(
        seed,
        BlockId(1), // grass
        BlockId(2), // dirt
        BlockId(3), // stone
        BlockId(4), // water
        BlockId(5), // sand
    );
    
    // The chunk containing world position (0,0) with chunk_size=32
    let chunk_size = 32;
    let chunk_pos = ChunkPos { x: 0, y: 0, z: 0 };
    
    println!("Generating chunk at {:?} (contains world origin)", chunk_pos);
    println!("Chunk world coordinates: x=[0..32), y=[0..32), z=[0..32)");
    
    // Generate the chunk
    let chunk = generator.generate_chunk(chunk_pos, chunk_size);
    
    // Trace blocks at x=0, z=0 (which is at local coordinates 0,0 in this chunk)
    println!("\nBlocks at world position (0, ?, 0):");
    println!("Y | Block ID | Block Type");
    println!("--|----------|------------");
    
    let mut highest_solid_y = -1;
    for y in 0..chunk_size {
        let block = chunk.get_block(0, y, 0);
        if block != BlockId::AIR {
            highest_solid_y = y as i32;
            let block_type = match block.0 {
                1 => "Grass",
                2 => "Dirt", 
                3 => "Stone",
                4 => "Water",
                5 => "Sand",
                _ => "Unknown",
            };
            println!("{:2} | {:8} | {}", y, block.0, block_type);
        }
    }
    
    println!("\nHighest solid block in chunk at (0,0): y={}", highest_solid_y);
    
    // Now check higher chunks to find the actual surface
    println!("\n=== Checking higher chunks ===");
    
    for chunk_y in 1..5 {
        let chunk_pos = ChunkPos { x: 0, y: chunk_y, z: 0 };
        let chunk = generator.generate_chunk(chunk_pos, chunk_size);
        
        let world_y_start = chunk_y * 32;
        let world_y_end = world_y_start + 32;
        
        println!("\nChunk y={} (world y=[{}..{})):", chunk_y, world_y_start, world_y_end);
        
        for y in 0..chunk_size {
            let world_y = world_y_start + y as i32;
            let block = chunk.get_block(0, y, 0);
            if block != BlockId::AIR {
                let block_type = match block.0 {
                    1 => "Grass",
                    2 => "Dirt",
                    3 => "Stone", 
                    4 => "Water",
                    5 => "Sand",
                    _ => "Unknown",
                };
                println!("  World y={}: {} ({})", world_y, block_type, block.0);
                highest_solid_y = world_y;
            }
        }
    }
    
    println!("\n=== Summary ===");
    println!("get_surface_height(0, 0) returns: {}", clamped_height);
    println!("Actual highest solid block at (0,0): y={}", highest_solid_y);
    println!("Difference: {} blocks", highest_solid_y - clamped_height);
}