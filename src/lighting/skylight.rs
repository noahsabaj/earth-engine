use crate::world::{World, VoxelPos, ChunkPos, BlockId};
use crate::lighting::{MAX_LIGHT_LEVEL, LightType};

/// Calculates skylight propagation from the sky downward
pub struct SkylightCalculator;

impl SkylightCalculator {
    /// Calculate skylight for a newly loaded chunk
    pub fn calculate_for_chunk(world: &mut World, chunk_pos: ChunkPos, chunk_size: u32) {
        let world_x_start = chunk_pos.x * chunk_size as i32;
        let world_y_start = chunk_pos.y * chunk_size as i32;
        let world_z_start = chunk_pos.z * chunk_size as i32;
        
        // For each column in the chunk
        for local_x in 0..chunk_size {
            for local_z in 0..chunk_size {
                let world_x = world_x_start + local_x as i32;
                let world_z = world_z_start + local_z as i32;
                
                // Get the surface height for this column
                let surface_height = world.get_surface_height(world_x as f64, world_z as f64);
                
                // Propagate skylight down from the top
                for local_y in (0..chunk_size).rev() {
                    let world_y = world_y_start + local_y as i32;
                    let pos = VoxelPos::new(world_x, world_y, world_z);
                    
                    if world_y > surface_height + 10 {
                        // Above surface with margin - full skylight
                        world.set_sky_light(pos, MAX_LIGHT_LEVEL);
                    } else {
                        // Check if we need to propagate light down
                        let block = world.get_block(pos);
                        
                        if block == BlockId::AIR || world.is_block_transparent(pos) {
                            // Check light level above
                            let above_pos = VoxelPos::new(world_x, world_y + 1, world_z);
                            let above_light = world.get_sky_light(above_pos);
                            
                            if above_light == MAX_LIGHT_LEVEL {
                                // Full skylight continues down
                                world.set_sky_light(pos, MAX_LIGHT_LEVEL);
                            } else if above_light > 0 {
                                // Reduced skylight
                                world.set_sky_light(pos, above_light.saturating_sub(1));
                            } else {
                                world.set_sky_light(pos, 0);
                            }
                        } else {
                            // Solid block - no skylight
                            world.set_sky_light(pos, 0);
                        }
                    }
                }
            }
        }
    }
    
    /// Update skylight when a block is placed or removed
    pub fn update_column(world: &mut World, x: i32, y: i32, z: i32) {
        // When a block is removed, skylight might need to propagate down
        let pos = VoxelPos::new(x, y, z);
        
        if world.get_block(pos) == BlockId::AIR {
            // Block was removed - check if skylight should propagate down
            let above_pos = VoxelPos::new(x, y + 1, z);
            let above_light = world.get_sky_light(above_pos);
            
            if above_light == MAX_LIGHT_LEVEL {
                // Propagate full skylight down through air
                let mut check_y = y;
                while check_y >= 0 {
                    let check_pos = VoxelPos::new(x, check_y, z);
                    if world.get_block(check_pos) == BlockId::AIR {
                        world.set_sky_light(check_pos, MAX_LIGHT_LEVEL);
                        check_y -= 1;
                    } else {
                        break;
                    }
                }
            }
        } else {
            // Block was placed - remove skylight below
            let mut check_y = y - 1;
            while check_y >= 0 {
                let check_pos = VoxelPos::new(x, check_y, z);
                let current_light = world.get_sky_light(check_pos);
                
                if current_light == MAX_LIGHT_LEVEL {
                    world.set_sky_light(check_pos, 0);
                    check_y -= 1;
                } else {
                    break;
                }
            }
        }
    }
}