/// Test CPU terrain generation to verify it's not producing vertical strips
use hearth_engine::*;
use hearth_engine::world::{WorldGenerator, ChunkPos};
use hearth_engine::world::generation::DefaultWorldGenerator;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    println!("\n=== Testing CPU Terrain Generation ===\n");
    
    // Create CPU world generator
    let generator = DefaultWorldGenerator::new(
        12345, // seed
        BlockId::GRASS,
        BlockId::DIRT,
        BlockId::STONE,
        BlockId::WATER,
        BlockId::SAND,
    );
    
    // Test several chunks
    let test_positions = vec![
        ChunkPos::new(0, 0, 0),
        ChunkPos::new(1, 0, 0),
        ChunkPos::new(0, 1, 0),
        ChunkPos::new(0, 2, 0),
    ];
    
    println!("Analyzing terrain generation patterns...\n");
    
    for pos in test_positions {
        println!("Chunk at {:?}:", pos);
        
        let chunk_size = 50; // Updated for 1dcm³ voxels: 5m x 5m x 5m chunks
        let chunk = generator.generate_chunk(pos, chunk_size);
        let mut layers = vec![0; chunk_size as usize]; // Count blocks per Y layer
        let mut total_blocks = 0;
        
        // Analyze terrain distribution
        for y in 0..chunk_size {
            for x in 0..chunk_size {
                for z in 0..chunk_size {
                    if chunk.get_block(x, y, z) != BlockId::AIR {
                        layers[y as usize] += 1;
                        total_blocks += 1;
                    }
                }
            }
        }
        
        // Show distribution
        println!("  Terrain distribution by Y layer:");
        let mut strip_pattern = true;
        let mut has_terrain = false;
        
        for (y, count) in layers.iter().enumerate() {
            if *count > 0 {
                has_terrain = true;
                let percent = (*count as f32 / 2500.0) * 100.0; // 50*50 voxels per layer
                println!("    Y={:2}: {:4} blocks ({:5.1}%)", y, count, percent);
                
                // Check if it's NOT a strip pattern (strips would have all 2500 blocks)
                if *count != 2500 {
                    strip_pattern = false;
                }
            }
        }
        
        if !has_terrain {
            println!("  ⚠️  WARNING: No terrain in this chunk!");
        } else if strip_pattern && total_blocks > 1024 {
            println!("  ⚠️  WARNING: Possible vertical strip pattern detected!");
        } else {
            println!("  ✅ Natural terrain variation detected");
        }
        
        println!("  Total blocks: {}/{} ({:.1}%)\n", 
                 total_blocks, 125000, (total_blocks as f32 / 125000.0) * 100.0); // 50*50*50 voxels per chunk
    }
    
    // Also test surface height function
    println!("Testing surface height generation:");
    for x in 0..5 {
        for z in 0..5 {
            let height = generator.get_surface_height(x as f64 * 16.0, z as f64 * 16.0);
            print!("{:3} ", height);
        }
        println!();
    }
    
    println!("\n=== Test Complete ===");
}