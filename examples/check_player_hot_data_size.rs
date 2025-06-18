use hearth_engine::persistence::player_data_dop::{PlayerHotData, CACHE_LINE_SIZE};

fn main() {
    println!("PlayerHotData size: {} bytes", std::mem::size_of::<PlayerHotData>());
    println!("Cache line size: {} bytes", CACHE_LINE_SIZE);
    println!("Alignment: {} bytes", std::mem::align_of::<PlayerHotData>());
    
    // Check individual field sizes
    println!("\nField sizes:");
    println!("Vec3 (position): {} bytes", std::mem::size_of::<glam::Vec3>());
    println!("Vec3 (velocity): {} bytes", std::mem::size_of::<glam::Vec3>());
    println!("Quat (rotation): {} bytes", std::mem::size_of::<glam::Quat>());
    println!("f32 (health): {} bytes", std::mem::size_of::<f32>());
    println!("f32 (hunger): {} bytes", std::mem::size_of::<f32>());
    println!("u32 (experience): {} bytes", std::mem::size_of::<u32>());
    println!("u32 (level): {} bytes", std::mem::size_of::<u32>());
    println!("u8 (game_mode): {} bytes", std::mem::size_of::<u8>());
    println!("u8 (movement_state): {} bytes", std::mem::size_of::<u8>());
    println!("u8 (dirty_flags): {} bytes", std::mem::size_of::<u8>());
    
    // Calculate expected size
    let expected_size = 
        std::mem::size_of::<glam::Vec3>() * 2 +  // position + velocity = 24 bytes
        std::mem::size_of::<glam::Quat>() +      // rotation = 16 bytes
        std::mem::size_of::<f32>() * 2 +         // health + hunger = 8 bytes
        std::mem::size_of::<u32>() * 2 +         // experience + level = 8 bytes
        std::mem::size_of::<u8>() * 3 +          // game_mode + movement_state + dirty_flags = 3 bytes
        5;                                       // padding = 5 bytes
    
    println!("\nExpected size (without alignment): {} bytes", expected_size);
    println!("Fits in cache line: {}", expected_size <= CACHE_LINE_SIZE);
    println!("Actual fits in cache line: {}", std::mem::size_of::<PlayerHotData>() <= CACHE_LINE_SIZE);
}