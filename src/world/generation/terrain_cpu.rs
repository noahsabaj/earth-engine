use crate::world::core::{BlockId, ChunkPos};
use crate::world::storage::ChunkSoA;
use noise::{NoiseFn, Perlin};

// Import terrain generation constants for voxel-scaled measurements
use crate::constants::terrain::*;

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
        let scale1 = 0.01; // Large features (mountains, valleys)
        let scale2 = 0.05; // Medium features (hills)
        let scale3 = 0.1; // Small features (bumps)

        // Sample noise at different scales (voxel-scaled for 1dcm³ world)
        let height1 =
            self.height_noise.get([world_x * scale1, world_z * scale1]) * MOUNTAIN_AMPLITUDE as f64;
        let height2 =
            self.detail_noise.get([world_x * scale2, world_z * scale2]) * HILL_AMPLITUDE as f64;
        let height3 =
            self.height_noise.get([world_x * scale3, world_z * scale3]) * DETAIL_AMPLITUDE as f64;

        // Combine octaves
        let combined_height = height1 + height2 + height3;

        // Base height at sea level (voxel-scaled: 64m = 640 voxels) with variation
        let base_height = SEA_LEVEL;
        let final_height = base_height + combined_height as i32;

        // Clamp to reasonable values (voxel-scaled: 10m-200m = 100-2000 voxels)
        final_height.clamp(MIN_HEIGHT, MAX_HEIGHT)
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

/// Default world generator implementation
pub struct DefaultWorldGenerator {
    terrain_gen: TerrainGenerator,
    // Add other generators like caves, ores, etc. later
}

impl DefaultWorldGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            terrain_gen: TerrainGenerator::new(seed),
        }
    }

    pub fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> ChunkSoA {
        log::info!("[DefaultWorldGenerator] Generating chunk {:?} with size {}", chunk_pos, chunk_size);
        let mut chunk = ChunkSoA::new(chunk_pos, chunk_size);

        // Generate terrain for this chunk
        let chunk_world_x = chunk_pos.x * chunk_size as i32;
        let chunk_world_z = chunk_pos.z * chunk_size as i32;
        let chunk_world_y = chunk_pos.y * chunk_size as i32;
        
        let mut non_air_count = 0;

        // Log the first surface height to debug
        let first_surface_height = self.terrain_gen.get_height(chunk_world_x as f64, chunk_world_z as f64);
        log::info!("[DefaultWorldGenerator] First surface height at ({}, {}): {}, chunk Y range: {}-{}", 
                  chunk_world_x, chunk_world_z, first_surface_height,
                  chunk_world_y, chunk_world_y + chunk_size as i32);
        
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                let world_x = chunk_world_x + x as i32;
                let world_z = chunk_world_z + z as i32;

                let surface_height = self.terrain_gen.get_height(world_x as f64, world_z as f64);

                for y in 0..chunk_size {
                    let world_y = chunk_world_y + y as i32;
                    let local_idx = (x + y * chunk_size + z * chunk_size * chunk_size) as usize;

                    if world_y < surface_height - 3 {
                        chunk.set_block_by_index(local_idx, BlockId::STONE);
                        non_air_count += 1;
                    } else if world_y < surface_height {
                        chunk.set_block_by_index(local_idx, BlockId::DIRT);
                        non_air_count += 1;
                    } else if world_y == surface_height {
                        chunk.set_block_by_index(local_idx, BlockId::GRASS);
                        non_air_count += 1;
                    } else {
                        chunk.set_block_by_index(local_idx, BlockId::AIR);
                    }
                }
            }
        }
        
        log::info!("[DefaultWorldGenerator] Generated chunk {:?} with {} non-air blocks out of {} total", 
                  chunk_pos, non_air_count, chunk_size * chunk_size * chunk_size);

        chunk
    }

    pub fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        self.terrain_gen.get_height(world_x, world_z)
    }
}

impl super::unified_generator::WorldGenerator for DefaultWorldGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> ChunkSoA {
        self.generate_chunk(chunk_pos, chunk_size)
    }

    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        self.get_surface_height(world_x, world_z)
    }

    fn is_gpu(&self) -> bool {
        false
    }
}
