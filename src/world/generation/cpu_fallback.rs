//! CPU fallback world generator for the unified architecture
//! 
//! Provides CPU-based world generation when GPU is not available.
//! This is a clean, efficient implementation following DOP principles.

use crate::world::{
    core::{BlockId, ChunkPos, VoxelPos},
    storage::ChunkSoA,
    generation::{WorldGenerator, TerrainParams, BlockIds},
};
use noise::{NoiseFn, Perlin, Seedable};

/// CPU-based world generator for fallback and debugging
pub struct CpuWorldGenerator {
    terrain_noise: Perlin,
    cave_noise: Perlin,
    ore_noise: Perlin,
    params: TerrainParams,
    block_ids: BlockIds,
}


impl CpuWorldGenerator {
    /// Create a new CPU world generator
    pub fn new(seed: u32, params: TerrainParams, block_ids: BlockIds) -> Self {
        let terrain_noise = Perlin::new(seed);
        let cave_noise = Perlin::new(seed.wrapping_add(1));
        let ore_noise = Perlin::new(seed.wrapping_add(2));
        
        Self {
            terrain_noise,
            cave_noise,
            ore_noise,
            params,
            block_ids,
        }
    }
    
    /// Generate terrain height at world coordinates
    fn get_terrain_height(&self, world_x: f64, world_z: f64) -> i32 {
        let scale = self.params.terrain_scale as f64;
        let amplitude = self.params.terrain_amplitude as f64;
        let base_height = self.params.terrain_offset as f64;
        
        // Multi-octave noise for more interesting terrain
        let mut height = 0.0;
        let mut frequency = 1.0 / scale;
        let mut amp = amplitude;
        
        for _ in 0..4 {
            let noise = self.terrain_noise.get([world_x * frequency, world_z * frequency]);
            height += noise * amp;
            frequency *= 2.0;
            amp *= 0.5;
        }
        
        (base_height + height) as i32
    }
    
    /// Check if a position should be a cave
    fn is_cave(&self, x: i32, y: i32, z: i32) -> bool {
        let threshold = self.params.cave_threshold;
        let scale = 0.05;
        
        let noise = self.cave_noise.get([
            x as f64 * scale,
            y as f64 * scale,
            z as f64 * scale,
        ]) as f32;
        
        noise > threshold
    }
    
    /// Get block type for a position
    fn get_block_at(&self, world_pos: VoxelPos, surface_height: i32) -> BlockId {
        let y = world_pos.y;
        
        // Air above surface
        if y > surface_height {
            return self.block_ids.air;
        }
        
        // Check for caves
        if self.is_cave(world_pos.x, world_pos.y, world_pos.z) && y < surface_height - 2 {
            return self.block_ids.air;
        }
        
        // Surface block
        if y == surface_height {
            if surface_height < self.params.water_level {
                return self.block_ids.sand;
            } else {
                return self.block_ids.grass;
            }
        }
        
        // Just below surface
        if y >= surface_height - 3 {
            return self.block_ids.dirt;
        }
        
        // Deep underground
        self.block_ids.stone
    }
}

impl WorldGenerator for CpuWorldGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> ChunkSoA {
        let mut chunk = ChunkSoA::new(chunk_pos, chunk_size);
        
        // Generate terrain for each column
        for local_x in 0..chunk_size {
            for local_z in 0..chunk_size {
                let world_x = chunk_pos.x * chunk_size as i32 + local_x as i32;
                let world_z = chunk_pos.z * chunk_size as i32 + local_z as i32;
                
                // Get surface height for this column
                let surface_height = self.get_terrain_height(world_x as f64, world_z as f64);
                
                // Fill the column
                for local_y in 0..chunk_size {
                    let world_y = chunk_pos.y * chunk_size as i32 + local_y as i32;
                    let world_pos = VoxelPos::new(world_x, world_y, world_z);
                    
                    let block = self.get_block_at(world_pos, surface_height);
                    chunk.set_block(local_x, local_y, local_z, block);
                }
                
                // Add water if below water level
                if surface_height < self.params.water_level {
                    for local_y in 0..chunk_size {
                        let world_y = chunk_pos.y * chunk_size as i32 + local_y as i32;
                        
                        if world_y > surface_height && world_y <= self.params.water_level {
                            if chunk.get_block(local_x, local_y, local_z) == self.block_ids.air {
                                chunk.set_block(local_x, local_y, local_z, self.block_ids.water);
                            }
                        }
                    }
                }
            }
        }
        
        chunk
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        self.get_terrain_height(world_x, world_z)
    }
    
    fn is_gpu(&self) -> bool {
        false // This is the CPU fallback
    }
}