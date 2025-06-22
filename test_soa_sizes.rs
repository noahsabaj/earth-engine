// Standalone test to calculate SOA structure sizes

const MAX_BLOCK_DISTRIBUTIONS: usize = 16;

#[repr(C)]
struct BlockDistributionSOA {
    count: u32,
    _padding: [u32; 3],
    
    // Arrays - each field stored contiguously
    block_ids: [u32; MAX_BLOCK_DISTRIBUTIONS],
    min_heights: [i32; MAX_BLOCK_DISTRIBUTIONS],
    max_heights: [i32; MAX_BLOCK_DISTRIBUTIONS],
    probabilities: [f32; MAX_BLOCK_DISTRIBUTIONS],
    noise_thresholds: [f32; MAX_BLOCK_DISTRIBUTIONS],
}

#[repr(C)]
struct TerrainParamsSOA {
    // Scalar parameters
    seed: u32,
    sea_level: f32,
    terrain_scale: f32,
    mountain_threshold: f32,
    cave_threshold: f32,
    num_distributions: u32,
    weather_type_intensity: u32,
    temperature: i32,
    
    // Embedded SOA distributions
    distributions: BlockDistributionSOA,
}

fn main() {
    println!("=== SOA Structure Size Analysis ===\n");
    
    // Calculate BlockDistributionSOA size
    let block_dist_size = std::mem::size_of::<BlockDistributionSOA>();
    println!("BlockDistributionSOA:");
    println!("  Rust size: {} bytes", block_dist_size);
    println!("  Breakdown:");
    println!("    count: {} bytes", std::mem::size_of::<u32>());
    println!("    _padding: {} bytes", std::mem::size_of::<[u32; 3]>());
    println!("    block_ids: {} bytes ({} * {})", 
        std::mem::size_of::<[u32; MAX_BLOCK_DISTRIBUTIONS]>(),
        std::mem::size_of::<u32>(),
        MAX_BLOCK_DISTRIBUTIONS
    );
    println!("    min_heights: {} bytes", std::mem::size_of::<[i32; MAX_BLOCK_DISTRIBUTIONS]>());
    println!("    max_heights: {} bytes", std::mem::size_of::<[i32; MAX_BLOCK_DISTRIBUTIONS]>());
    println!("    probabilities: {} bytes", std::mem::size_of::<[f32; MAX_BLOCK_DISTRIBUTIONS]>());
    println!("    noise_thresholds: {} bytes", std::mem::size_of::<[f32; MAX_BLOCK_DISTRIBUTIONS]>());
    
    let calculated_block_dist = 4 + 12 + (5 * MAX_BLOCK_DISTRIBUTIONS * 4);
    println!("  Calculated total: {} bytes", calculated_block_dist);
    
    // Calculate TerrainParamsSOA size
    println!("\nTerrainParamsSOA:");
    let terrain_params_size = std::mem::size_of::<TerrainParamsSOA>();
    println!("  Rust size: {} bytes", terrain_params_size);
    println!("  Breakdown:");
    println!("    Scalar fields (8 fields): {} bytes", 8 * 4);
    println!("    distributions: {} bytes", block_dist_size);
    
    let calculated_terrain = 32 + block_dist_size;
    println!("  Calculated total: {} bytes", calculated_terrain);
    
    // Check alignment
    println!("\nAlignment Analysis:");
    println!("  BlockDistributionSOA alignment: {} bytes", std::mem::align_of::<BlockDistributionSOA>());
    println!("  TerrainParamsSOA alignment: {} bytes", std::mem::align_of::<TerrainParamsSOA>());
    
    // GPU alignment considerations
    println!("\nGPU Alignment:");
    let block_dist_aligned = ((block_dist_size + 15) / 16) * 16;
    let terrain_aligned = ((terrain_params_size + 15) / 16) * 16;
    println!("  BlockDistributionSOA (16-byte aligned): {} bytes", block_dist_aligned);
    println!("  TerrainParamsSOA (16-byte aligned): {} bytes", terrain_aligned);
    
    // Compare with expected log value
    println!("\nComparison with logs:");
    println!("  Log shows: TerrainParamsSOA size: 368 bytes");
    println!("  Actual size: {} bytes", terrain_params_size);
    if terrain_params_size == 368 {
        println!("  ✓ Sizes match!");
    } else {
        println!("  ✗ Size mismatch! Difference: {} bytes", 
            (terrain_params_size as i32 - 368).abs());
    }
}