use noise::{NoiseFn, Perlin};

pub struct TerrainGenerator {
    height_noise: Perlin,
    detail_noise: Perlin,
    seed: u32,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        let height_noise = Perlin::new(seed);
        let detail_noise = Perlin::new(seed.wrapping_add(1));
        
        Self {
            height_noise,
            detail_noise,
            seed,
        }
    }
    
    pub fn get_height(&self, world_x: f64, world_z: f64) -> i32 {
        // Multiple octaves for more interesting terrain
        let scale1 = 0.01;  // Large features (mountains, valleys)
        let scale2 = 0.05;  // Medium features (hills)
        let scale3 = 0.1;   // Small features (bumps)
        
        // Sample noise at different scales
        let height1 = self.height_noise.get([world_x * scale1, world_z * scale1]) * 32.0;
        let height2 = self.detail_noise.get([world_x * scale2, world_z * scale2]) * 8.0;
        let height3 = self.height_noise.get([world_x * scale3, world_z * scale3]) * 2.0;
        
        // Combine octaves
        let combined_height = height1 + height2 + height3;
        
        // Base height at y=64 (sea level) with variation
        let base_height = 64;
        let final_height = base_height + combined_height as i32;
        
        // Clamp to reasonable values
        final_height.clamp(10, 200)
    }
    
    pub fn get_biome_factor(&self, world_x: f64, world_z: f64) -> f64 {
        // Use a different scale for biome variation
        let biome_scale = 0.003;
        let biome_noise = self.height_noise.get([
            world_x * biome_scale + 1000.0, // Offset to get different values
            world_z * biome_scale + 1000.0,
        ]);
        
        // Return value between 0 and 1
        (biome_noise + 1.0) * 0.5
    }
}