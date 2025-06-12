use crate::{BlockId, Chunk, ChunkPos};

pub mod terrain;
pub mod caves;
pub mod ores;

#[cfg(test)]
mod tests;

pub use terrain::TerrainGenerator;
pub use caves::CaveGenerator;
pub use ores::OreGenerator;

pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> Chunk;
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32;
    
    /// Find a safe spawn height at the given position
    fn find_safe_spawn_height(&self, world_x: f64, world_z: f64) -> f32 {
        let surface_height = self.get_surface_height(world_x, world_z);
        // Add 3 blocks of clearance above the terrain
        let safe_height = surface_height as f32 + 3.0;
        // Clamp to reasonable values
        safe_height.clamp(20.0, 250.0)
    }
}

pub struct DefaultWorldGenerator {
    terrain_gen: TerrainGenerator,
    cave_gen: CaveGenerator,
    ore_gen: OreGenerator,
    grass_id: BlockId,
    dirt_id: BlockId,
    stone_id: BlockId,
    water_id: BlockId,
    sand_id: BlockId,
}

impl DefaultWorldGenerator {
    pub fn new(
        seed: u32,
        grass_id: BlockId,
        dirt_id: BlockId,
        stone_id: BlockId,
        water_id: BlockId,
        sand_id: BlockId,
    ) -> Self {
        Self {
            terrain_gen: TerrainGenerator::new(seed),
            cave_gen: CaveGenerator::new(seed),
            ore_gen: OreGenerator::new(seed),
            grass_id,
            dirt_id,
            stone_id,
            water_id,
            sand_id,
        }
    }
}

impl WorldGenerator for DefaultWorldGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> Chunk {
        let mut chunk = Chunk::new(chunk_pos, chunk_size);
        
        // Calculate world coordinates for this chunk
        let world_x_start = chunk_pos.x * chunk_size as i32;
        let world_y_start = chunk_pos.y * chunk_size as i32;
        let world_z_start = chunk_pos.z * chunk_size as i32;
        
        // Log chunk generation for debugging
        static mut GEN_COUNT: usize = 0;
        unsafe {
            if GEN_COUNT < 10 {
                log::info!("[DefaultWorldGenerator::generate_chunk] Generating chunk at {:?}, world coords: ({}, {}, {})", 
                          chunk_pos, world_x_start, world_y_start, world_z_start);
                GEN_COUNT += 1;
            }
        }
        
        // Generate terrain
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                let world_x = world_x_start + x as i32;
                let world_z = world_z_start + z as i32;
                
                // Get surface height using Perlin noise
                let surface_height = self.terrain_gen.get_height(world_x as f64, world_z as f64);
                
                for y in 0..chunk_size {
                    let world_y = world_y_start + y as i32;
                    
                    // Skip if above chunk
                    if world_y > surface_height + 5 {
                        continue;
                    }
                    
                    // Check if this position should be a cave
                    let is_cave = self.cave_gen.is_cave(world_x, world_y, world_z);
                    
                    if !is_cave {
                        // Determine block type based on height
                        let block_id = if world_y == surface_height {
                            // Surface layer
                            if surface_height < 64 { // Sea level
                                self.sand_id
                            } else {
                                self.grass_id
                            }
                        } else if world_y > surface_height - 4 && world_y < surface_height {
                            self.dirt_id
                        } else if world_y <= surface_height - 4 {
                            // Check for ore generation
                            let ore_block = self.ore_gen.get_ore_at(world_x, world_y, world_z, self.stone_id);
                            ore_block
                        } else {
                            BlockId::AIR
                        };
                        
                        if block_id != BlockId::AIR {
                            chunk.set_block(x, y, z, block_id);
                        }
                    }
                }
                
                // Add water at sea level if needed
                if surface_height < 64 {
                    for y in 0..chunk_size {
                        let world_y = world_y_start + y as i32;
                        if world_y > surface_height && world_y <= 64 {
                            let local_block = chunk.get_block(x, y, z);
                            if local_block == BlockId::AIR {
                                chunk.set_block(x, y, z, self.water_id);
                            }
                        }
                    }
                }
            }
        }
        
        // Initialize skylight for the chunk
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                let world_x = world_x_start + x as i32;
                let world_z = world_z_start + z as i32;
                let surface_height = self.terrain_gen.get_height(world_x as f64, world_z as f64);
                
                // Set skylight from top to surface
                for y in (0..chunk_size).rev() {
                    let world_y = world_y_start + y as i32;
                    
                    if world_y > surface_height + 5 {
                        chunk.set_sky_light(x, y, z, 15); // Full skylight above surface
                    } else if chunk.get_block(x, y, z) == BlockId::AIR {
                        // Check if we can see sky
                        let mut can_see_sky = true;
                        for check_y in (y+1)..chunk_size {
                            if chunk.get_block(x, check_y, z) != BlockId::AIR {
                                can_see_sky = false;
                                break;
                            }
                        }
                        chunk.set_sky_light(x, y, z, if can_see_sky { 15 } else { 0 });
                    } else {
                        chunk.set_sky_light(x, y, z, 0); // No skylight in solid blocks
                    }
                }
            }
        }
        
        // Count non-air blocks for debugging
        let mut block_count = 0;
        for y in 0..chunk_size {
            for z in 0..chunk_size {
                for x in 0..chunk_size {
                    if chunk.get_block(x, y, z) != BlockId::AIR {
                        block_count += 1;
                    }
                }
            }
        }
        
        unsafe {
            static mut LOG_COUNT: usize = 0;
            if LOG_COUNT < 10 {
                log::info!("[DefaultWorldGenerator::generate_chunk] Chunk at {:?} has {} non-air blocks out of {} total", 
                          chunk_pos, block_count, chunk_size * chunk_size * chunk_size);
                LOG_COUNT += 1;
            }
        }
        
        chunk
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        self.terrain_gen.get_height(world_x, world_z)
    }
}