use noise::{NoiseFn, Perlin};

pub struct CaveGenerator {
    cave_noise: Perlin,
    seed: u32,
}

impl CaveGenerator {
    pub fn new(seed: u32) -> Self {
        let cave_noise = Perlin::new(seed.wrapping_add(100)); // Different seed for caves
        
        Self {
            cave_noise,
            seed,
        }
    }
    
    pub fn is_cave(&self, world_x: i32, world_y: i32, world_z: i32) -> bool {
        // Don't generate caves too close to surface
        if world_y > 60 {
            return false;
        }
        
        // 3D noise for cave generation
        let scale = 0.05; // Cave scale
        let threshold = 0.3; // Higher = fewer caves
        
        let noise_value = self.cave_noise.get([
            world_x as f64 * scale,
            world_y as f64 * scale,
            world_z as f64 * scale,
        ]);
        
        // Create larger caves at lower depths
        let depth_factor = (60 - world_y) as f64 / 60.0;
        let adjusted_threshold = threshold - (depth_factor * 0.1);
        
        noise_value.abs() < adjusted_threshold
    }
    
    pub fn get_cave_size(&self, world_x: i32, world_y: i32, world_z: i32) -> f64 {
        // Get a value indicating how "cavey" this position is
        let scale = 0.05;
        let noise_value = self.cave_noise.get([
            world_x as f64 * scale,
            world_y as f64 * scale,
            world_z as f64 * scale,
        ]);
        
        noise_value.abs()
    }
}