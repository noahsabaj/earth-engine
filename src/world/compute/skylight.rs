//! Skylight calculation for the unified world system
//!
//! This module handles skylight propagation from the sky downward,
//! updating light levels when blocks are placed or removed.

use crate::world::{
    core::{VoxelPos, ChunkPos, BlockId},
    interfaces::WorldInterface,
};

/// Maximum skylight level (full brightness from sky)
pub const MAX_SKY_LIGHT: u8 = 15;

/// Calculates skylight propagation from the sky downward
pub struct SkylightCalculator;

impl SkylightCalculator {
    /// Calculate skylight for a newly loaded chunk
    pub fn calculate_for_chunk(
        world: &mut dyn WorldInterface,
        chunk_pos: ChunkPos,
        chunk_size: u32,
    ) -> Result<(), crate::world::interfaces::WorldError> {
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
                        // TODO: Add set_sky_light to WorldInterface
                        // world.set_sky_light(pos, MAX_SKY_LIGHT)?;
                    } else {
                        // Check if we need to propagate light down
                        let block = world.get_block(pos);
                        
                        if block == BlockId::AIR {
                            // Air blocks get skylight from above
                            // TODO: Add skylight methods to WorldInterface
                            // let above_pos = VoxelPos::new(world_x, world_y + 1, world_z);
                            // let above_light = world.get_sky_light(above_pos);
                            // world.set_sky_light(pos, above_light)?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Update skylight when a block is placed or removed
    pub fn update_column(
        world: &mut dyn WorldInterface,
        x: i32,
        y: i32,
        z: i32,
    ) -> Result<(), crate::world::interfaces::WorldError> {
        let pos = VoxelPos::new(x, y, z);
        
        if world.get_block(pos) == BlockId::AIR {
            // Block was removed - skylight needs to propagate down
            // TODO: Implement skylight propagation when methods are added to WorldInterface
        } else {
            // Block was placed - remove skylight below
            // TODO: Implement skylight removal when methods are added to WorldInterface
        }
        
        Ok(())
    }
}