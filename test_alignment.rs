// Test for alignment and padding issues in SOA structures

const MAX_BLOCK_DISTRIBUTIONS: usize = 16;

#[repr(C)]
struct BlockDistributionSOA {
    count: u32,
    _padding: [u32; 3],
    block_ids: [u32; MAX_BLOCK_DISTRIBUTIONS],
    min_heights: [i32; MAX_BLOCK_DISTRIBUTIONS],
    max_heights: [i32; MAX_BLOCK_DISTRIBUTIONS],
    probabilities: [f32; MAX_BLOCK_DISTRIBUTIONS],
    noise_thresholds: [f32; MAX_BLOCK_DISTRIBUTIONS],
}

#[repr(C)]
struct TerrainParamsSOA {
    seed: u32,
    sea_level: f32,
    terrain_scale: f32,
    mountain_threshold: f32,
    cave_threshold: f32,
    num_distributions: u32,
    weather_type_intensity: u32,
    temperature: i32,
    distributions: BlockDistributionSOA,
}

fn main() {
    println!("=== Alignment and Padding Analysis ===\n");
    
    // Check field offsets in BlockDistributionSOA
    println!("BlockDistributionSOA field offsets:");
    unsafe {
        let dummy = std::mem::MaybeUninit::<BlockDistributionSOA>::uninit();
        let base_ptr = dummy.as_ptr() as usize;
        
        println!("  count: offset {} bytes", 
            &(*dummy.as_ptr()).count as *const _ as usize - base_ptr);
        println!("  _padding: offset {} bytes", 
            &(*dummy.as_ptr())._padding as *const _ as usize - base_ptr);
        println!("  block_ids: offset {} bytes", 
            &(*dummy.as_ptr()).block_ids as *const _ as usize - base_ptr);
        println!("  min_heights: offset {} bytes", 
            &(*dummy.as_ptr()).min_heights as *const _ as usize - base_ptr);
        println!("  max_heights: offset {} bytes", 
            &(*dummy.as_ptr()).max_heights as *const _ as usize - base_ptr);
        println!("  probabilities: offset {} bytes", 
            &(*dummy.as_ptr()).probabilities as *const _ as usize - base_ptr);
        println!("  noise_thresholds: offset {} bytes", 
            &(*dummy.as_ptr()).noise_thresholds as *const _ as usize - base_ptr);
    }
    
    println!("\nTerrainParamsSOA field offsets:");
    unsafe {
        let dummy = std::mem::MaybeUninit::<TerrainParamsSOA>::uninit();
        let base_ptr = dummy.as_ptr() as usize;
        
        println!("  seed: offset {} bytes", 
            &(*dummy.as_ptr()).seed as *const _ as usize - base_ptr);
        println!("  sea_level: offset {} bytes", 
            &(*dummy.as_ptr()).sea_level as *const _ as usize - base_ptr);
        println!("  terrain_scale: offset {} bytes", 
            &(*dummy.as_ptr()).terrain_scale as *const _ as usize - base_ptr);
        println!("  mountain_threshold: offset {} bytes", 
            &(*dummy.as_ptr()).mountain_threshold as *const _ as usize - base_ptr);
        println!("  cave_threshold: offset {} bytes", 
            &(*dummy.as_ptr()).cave_threshold as *const _ as usize - base_ptr);
        println!("  num_distributions: offset {} bytes", 
            &(*dummy.as_ptr()).num_distributions as *const _ as usize - base_ptr);
        println!("  weather_type_intensity: offset {} bytes", 
            &(*dummy.as_ptr()).weather_type_intensity as *const _ as usize - base_ptr);
        println!("  temperature: offset {} bytes", 
            &(*dummy.as_ptr()).temperature as *const _ as usize - base_ptr);
        println!("  distributions: offset {} bytes", 
            &(*dummy.as_ptr()).distributions as *const _ as usize - base_ptr);
    }
    
    // Check for potential alignment issues
    println!("\nAlignment checks:");
    println!("  BlockDistributionSOA size: {} bytes", std::mem::size_of::<BlockDistributionSOA>());
    println!("  16-byte aligned: {}", std::mem::size_of::<BlockDistributionSOA>() % 16 == 0);
    println!("  TerrainParamsSOA size: {} bytes", std::mem::size_of::<TerrainParamsSOA>());
    println!("  16-byte aligned: {}", std::mem::size_of::<TerrainParamsSOA>() % 16 == 0);
    
    // Check for potential GPU buffer issues
    println!("\nGPU buffer considerations:");
    let terrain_size = std::mem::size_of::<TerrainParamsSOA>();
    let buffer_size_16 = ((terrain_size + 15) / 16) * 16;
    let buffer_size_256 = ((terrain_size + 255) / 256) * 256;
    
    println!("  Raw size: {} bytes", terrain_size);
    println!("  16-byte aligned buffer: {} bytes", buffer_size_16);
    println!("  256-byte aligned buffer: {} bytes", buffer_size_256);
    
    // Check if 368 matches expected shader size
    if terrain_size == 368 {
        println!("\n✓ Size matches log value (368 bytes)");
    } else {
        println!("\n✗ Size mismatch! Expected 368, got {}", terrain_size);
    }
    
    // Check for potential overflow with array indexing
    println!("\nArray size calculations:");
    let array_element_size = 4; // u32/i32/f32 are all 4 bytes
    let total_array_size = 5 * MAX_BLOCK_DISTRIBUTIONS * array_element_size;
    println!("  5 arrays × {} elements × {} bytes = {} bytes", 
        MAX_BLOCK_DISTRIBUTIONS, array_element_size, total_array_size);
    println!("  Plus count + padding: {} bytes", 4 + 12);
    println!("  Total BlockDistributionSOA: {} bytes", total_array_size + 16);
}