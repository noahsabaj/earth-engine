use hearth_engine::world::{WorldGenerator, DefaultWorldGenerator, BlockId, ChunkPos};
use noise::{NoiseFn, Perlin};

fn main() {
    println!("=== Detailed trace of spawn position (0, 0) ===\n");
    
    let seed = 12345u32;
    
    // Recreate the exact noise generators
    let height_noise = Perlin::new(seed);
    let detail_noise = Perlin::new(seed.wrapping_add(1));
    let cave_noise = Perlin::new(seed.wrapping_add(100));
    
    println!("1. TERRAIN HEIGHT CALCULATION");
    println!("=============================");
    
    // At position (0, 0), all noise values are 0
    println!("At origin (0, 0), Perlin noise returns 0");
    println!("Combined terrain height: 64 (base) + 0 (noise) = 64\n");
    
    println!("2. BLOCK PLACEMENT ANALYSIS");
    println!("===========================");
    
    let generator = DefaultWorldGenerator::new(
        seed,
        BlockId(1), // grass
        BlockId(2), // dirt
        BlockId(3), // stone
        BlockId(4), // water
        BlockId(5), // sand
    );
    
    // Analyze what happens at each Y level at position (0, 0)
    println!("\nAnalyzing world position (0, 0, ?) from y=0 to y=100:");
    println!("Y    | Cave? | Block Type | Logic");
    println!("-----|-------|------------|-------");
    
    for world_y in 0..=100 {
        // Check cave generation
        let is_cave = if world_y > 60 {
            false // No caves above y=60
        } else {
            let cave_scale = 0.05;
            let cave_threshold = 0.3;
            let noise_val = cave_noise.get([0.0, world_y as f64 * cave_scale, 0.0]);
            let depth_factor = (60 - world_y) as f64 / 60.0;
            let adjusted_threshold = cave_threshold - (depth_factor * 0.1);
            noise_val.abs() < adjusted_threshold
        };
        
        // Determine block type based on DefaultWorldGenerator logic
        let surface_height = 64;
        let block_type = if world_y > surface_height + 5 {
            "Air (above surface+5)"
        } else if is_cave {
            "Air (cave)"
        } else if world_y == surface_height {
            if surface_height < 64 {
                "Sand (surface, below sea)"
            } else {
                "Grass (surface)"
            }
        } else if world_y > surface_height - 4 && world_y < surface_height {
            "Dirt"
        } else if world_y <= surface_height - 4 {
            "Stone"
        } else {
            "Air"
        };
        
        if world_y >= 55 && world_y <= 75 {  // Focus on area around surface
            println!("{:4} | {:5} | {:18} | {}", 
                world_y, 
                if is_cave { "Yes" } else { "No" },
                block_type,
                if world_y == 64 { "← Surface height" } else { "" }
            );
        }
    }
    
    println!("\n3. SPAWN HEIGHT CALCULATION");
    println!("============================");
    
    let surface_height = generator.get_surface_height(0.0, 0.0);
    let spawn_height = generator.find_safe_spawn_height(0.0, 0.0);
    
    println!("get_surface_height(0, 0) = {}", surface_height);
    println!("find_safe_spawn_height(0, 0) = {}", spawn_height);
    println!("Clearance = {} blocks above surface", spawn_height - surface_height as f32);
    
    println!("\n4. CHUNK BOUNDARIES");
    println!("===================");
    
    // With chunk_size = 32, position (0,0) is at:
    println!("World position (0, 0, 0) is at:");
    println!("- Chunk (0, 0, 0), local position (0, 0, 0)");
    println!("- Surface y=64 is in chunk (0, 2, 0), local y=0");
    println!("- Spawn y=67 is in chunk (0, 2, 0), local y=3");
    
    println!("\n5. ACTUAL BLOCKS AT SPAWN");
    println!("=========================");
    
    // Generate the chunk containing spawn position
    let spawn_chunk_pos = ChunkPos { x: 0, y: 2, z: 0 }; // y=64 to y=95
    let chunk = generator.generate_chunk(spawn_chunk_pos, 32);
    
    println!("Blocks in spawn chunk at x=0, z=0:");
    for y in 0..10 {  // First 10 blocks of the chunk
        let block = chunk.get_block(0, y, 0);
        let world_y = 64 + y as i32;
        let block_name = match block.0 {
            0 => "Air",
            1 => "Grass",
            2 => "Dirt",
            3 => "Stone",
            4 => "Water",
            5 => "Sand",
            _ => "Unknown"
        };
        println!("  World y={}: {} (ID={})", world_y, block_name, block.0);
    }
    
    println!("\n6. SUMMARY");
    println!("==========");
    println!("- Terrain generator returns height 64 for position (0,0)");
    println!("- This is the exact Y coordinate where grass block is placed");
    println!("- find_safe_spawn_height() adds 3 blocks clearance → y=67");
    println!("- Player spawns at (0.5, 67.0, 0.5) in world coordinates");
    println!("- This is 3 blocks above the grass surface as intended");
}