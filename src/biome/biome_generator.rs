use crate::biome::{BiomeType, BiomeMap, BiomeProperties};
use crate::world::{Chunk, ChunkPos, BlockId};
use noise::{NoiseFn, Perlin};

/// Parameters for biome generation
#[derive(Debug, Clone)]
pub struct BiomeGenerationParams {
    /// Base terrain height
    pub base_height: f32,
    /// Height variation
    pub height_variation: f32,
    /// Octaves for terrain noise
    pub octaves: u32,
    /// Frequency for terrain noise
    pub frequency: f64,
    /// Persistence for terrain noise
    pub persistence: f64,
    /// Sea level
    pub sea_level: i32,
}

impl Default for BiomeGenerationParams {
    fn default() -> Self {
        Self {
            base_height: 64.0,
            height_variation: 32.0,
            octaves: 4,
            frequency: 0.01,
            persistence: 0.5,
            sea_level: 63,
        }
    }
}

/// Generates terrain based on biomes
pub struct BiomeGenerator {
    /// Biome map
    biome_map: BiomeMap,
    /// Generation parameters
    params: BiomeGenerationParams,
    /// Noise generators
    height_noise: Perlin,
    detail_noise: Perlin,
    cave_noise: Perlin,
}

impl BiomeGenerator {
    /// Create a new biome generator
    pub fn new(seed: u64) -> Self {
        let height_noise = Perlin::new(seed as u32);
        let detail_noise = Perlin::new((seed + 1) as u32);
        let cave_noise = Perlin::new((seed + 2) as u32);
        
        Self {
            biome_map: BiomeMap::new(seed),
            params: BiomeGenerationParams::default(),
            height_noise,
            detail_noise,
            cave_noise,
        }
    }
    
    /// Generate a chunk with biome-based terrain
    pub fn generate_chunk(&mut self, chunk_pos: ChunkPos, chunk_size: u32) -> Chunk {
        let mut chunk = Chunk::new(chunk_pos, chunk_size);
        
        // Generate height map and biome data for the chunk
        let mut height_map = vec![vec![0i32; chunk_size as usize]; chunk_size as usize];
        let mut biome_map = vec![vec![BiomeType::Plains; chunk_size as usize]; chunk_size as usize];
        
        // Calculate heights and biomes
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                let world_x = chunk_pos.x * chunk_size as i32 + x as i32;
                let world_z = chunk_pos.z * chunk_size as i32 + z as i32;
                
                // Get biome
                let biome = self.biome_map.get_biome(world_x as f64, world_z as f64);
                biome_map[x as usize][z as usize] = biome;
                
                // Calculate height based on biome
                let height = self.calculate_height(world_x as f64, world_z as f64, biome);
                height_map[x as usize][z as usize] = height;
            }
        }
        
        // Fill chunk with blocks
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                let height = height_map[x as usize][z as usize];
                let biome = biome_map[x as usize][z as usize];
                let props = BiomeProperties::from_biome_type(biome);
                
                for y in 0..chunk_size {
                    let world_y = chunk_pos.y * chunk_size as i32 + y as i32;
                    let block = self.get_block_for_position(
                        x as i32, world_y, z as i32,
                        height, &props,
                    );
                    
                    chunk.set_block(x, y, z, block);
                }
            }
        }
        
        // Generate caves
        self.generate_caves(&mut chunk, chunk_pos, chunk_size);
        
        chunk
    }
    
    /// Calculate terrain height at a position
    fn calculate_height(&self, x: f64, z: f64, biome: BiomeType) -> i32 {
        let biome_height = biome.base_height();
        let biome_variation = biome.height_variation();
        
        // Multi-octave noise for terrain
        let mut height = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = self.params.frequency;
        
        for _ in 0..self.params.octaves {
            height += self.height_noise.get([x * frequency, z * frequency]) * amplitude;
            amplitude *= self.params.persistence;
            frequency *= 2.0;
        }
        
        // Add detail noise
        let detail = self.detail_noise.get([x * 0.05, z * 0.05]) * 0.1;
        height += detail;
        
        // Apply biome-specific modifications
        let height = match biome {
            BiomeType::Mountains | BiomeType::SnowyMountains => {
                // Amplify height for mountains
                height * 1.5 + 0.3
            },
            BiomeType::Ocean | BiomeType::DeepOcean => {
                // Flatten and lower oceans
                height * 0.3 - 0.5
            },
            BiomeType::River => {
                // Carve river channel
                height * 0.5 - 0.3
            },
            BiomeType::Beach => {
                // Smooth beach transition
                height * 0.2
            },
            BiomeType::Swamp => {
                // Flatten swamps
                height * 0.3
            },
            _ => height,
        };
        
        // Convert to world height
        let world_height = self.params.base_height
            + biome_height * 16.0
            + height as f32 * self.params.height_variation * biome_variation;
        
        world_height.round() as i32
    }
    
    /// Get the appropriate block for a position
    fn get_block_for_position(
        &self,
        _x: i32,
        y: i32,
        _z: i32,
        surface_height: i32,
        props: &BiomeProperties,
    ) -> BlockId {
        if y > surface_height {
            // Above ground
            if y <= self.params.sea_level {
                BlockId::WATER
            } else {
                BlockId::AIR
            }
        } else if y == surface_height {
            // Surface block
            if y <= self.params.sea_level - 1 {
                // Underwater surface
                props.subsurface_block
            } else {
                props.surface_block
            }
        } else if y >= surface_height - 3 {
            // Subsurface
            props.subsurface_block
        } else {
            // Deep underground
            props.stone_block
        }
    }
    
    /// Generate caves in a chunk
    fn generate_caves(&self, chunk: &mut Chunk, chunk_pos: ChunkPos, chunk_size: u32) {
        let cave_threshold = 0.5;
        
        for x in 0..chunk_size {
            for y in 0..chunk_size {
                for z in 0..chunk_size {
                    let world_x = chunk_pos.x * chunk_size as i32 + x as i32;
                    let world_y = chunk_pos.y * chunk_size as i32 + y as i32;
                    let world_z = chunk_pos.z * chunk_size as i32 + z as i32;
                    
                    // Skip near surface
                    if world_y > self.params.sea_level - 10 {
                        continue;
                    }
                    
                    // 3D cave noise
                    let cave_value = self.cave_noise.get([
                        world_x as f64 * 0.03,
                        world_y as f64 * 0.03,
                        world_z as f64 * 0.03,
                    ]);
                    
                    // Carve cave
                    if cave_value > cave_threshold {
                        let current = chunk.get_block(x, y, z);
                        if current != BlockId::WATER && current != BlockId::AIR {
                            chunk.set_block(x, y, z, BlockId::AIR);
                        }
                    }
                }
            }
        }
    }
    
    /// Get surface height at a world position
    pub fn get_surface_height(&mut self, x: f64, z: f64) -> i32 {
        let biome = self.biome_map.get_biome(x, z);
        self.calculate_height(x, z, biome)
    }
    
    /// Set generation parameters
    pub fn set_params(&mut self, params: BiomeGenerationParams) {
        self.params = params;
    }
    
    /// Get the biome at a position
    pub fn get_biome_at(&mut self, x: f64, z: f64) -> BiomeType {
        self.biome_map.get_biome(x, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_biome_generation() {
        let mut generator = BiomeGenerator::new(12345);
        let chunk = generator.generate_chunk(ChunkPos { x: 0, y: 0, z: 0 }, 16);
        
        // Check that chunk has some non-air blocks
        let mut has_blocks = false;
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    if chunk.get_block(x, y, z) != BlockId::AIR {
                        has_blocks = true;
                        break;
                    }
                }
            }
        }
        assert!(has_blocks);
    }
}