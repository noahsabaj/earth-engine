use noise::{NoiseFn, Perlin};
use crate::BlockId;

pub struct OreGenerator {
    ore_noise: Perlin,
    seed: u32,
}

impl OreGenerator {
    pub fn new(seed: u32) -> Self {
        let ore_noise = Perlin::new(seed.wrapping_add(200)); // Different seed for ores
        
        Self {
            ore_noise,
            seed,
        }
    }
    
    pub fn get_ore_at(&self, world_x: i32, world_y: i32, world_z: i32, default_block: BlockId) -> BlockId {
        // Different ores at different depths
        if world_y > 128 {
            return default_block; // No ores in high mountains
        }
        
        // Use noise to create ore veins
        let scale = 0.1;
        let noise_value = self.ore_noise.get([
            world_x as f64 * scale,
            world_y as f64 * scale,
            world_z as f64 * scale,
        ]);
        
        // Coal - common, found at all depths below 128
        if world_y <= 128 && noise_value > 0.85 {
            return BlockId(8); // Coal ore
        }
        
        // Iron - less common, below 64
        if world_y <= 64 && noise_value > 0.9 {
            return BlockId(9); // Iron ore
        }
        
        // Gold - rare, below 32
        if world_y <= 32 && noise_value > 0.95 {
            return BlockId(10); // Gold ore
        }
        
        // Diamond - very rare, below 16
        if world_y <= 16 && noise_value > 0.98 {
            return BlockId(11); // Diamond ore
        }
        
        default_block
    }
    
    pub fn get_ore_density(&self, world_y: i32) -> f64 {
        // Higher density at lower depths
        if world_y > 128 {
            0.0
        } else if world_y > 64 {
            0.02
        } else if world_y > 32 {
            0.03
        } else if world_y > 16 {
            0.04
        } else {
            0.05
        }
    }
}