use crate::{BlockId, VoxelPos, ChunkPos};
use crate::world::{WorldInterface, ParallelWorld};
use cgmath::Point3;

/// Utility for finding safe spawn positions in the world
pub struct SpawnFinder;

impl SpawnFinder {
    /// Find a safe spawn position using noise-based terrain height
    /// Returns the position where the player's feet should be (standing on solid ground)
    pub fn find_safe_spawn(
        world: &ParallelWorld,
        start_x: f32,
        start_z: f32,
        search_radius: i32,
    ) -> Result<Point3<f32>, String> {
        log::info!("[SpawnFinder] Starting spawn search at ({}, ??, {})", start_x, start_z);
        
        // Since chunk loading seems to be async and not working reliably for spawn,
        // we'll use the noise-based approach with extra safety margin
        let mut best_pos = None;
        let mut highest_ground = -1000.0;
        
        // Search in a small area for the highest ground
        for dx in -search_radius..=search_radius {
            for dz in -search_radius..=search_radius {
                let check_x = start_x + dx as f32;
                let check_z = start_z + dz as f32;
                
                let surface_height = world.get_surface_height(check_x as f64, check_z as f64) as f32;
                
                // Prefer higher ground that's not underwater
                if surface_height > highest_ground && surface_height >= 64.0 {
                    highest_ground = surface_height;
                    best_pos = Some((check_x, check_z));
                }
            }
        }
        
        // Use the best position found, or fallback to start position
        let (spawn_x, spawn_z) = best_pos.unwrap_or((start_x, start_z));
        let surface_y = world.get_surface_height(spawn_x as f64, spawn_z as f64) as f32;
        
        // Add safety margin above the surface (10 blocks to ensure we're above ground)
        // The physics body center will be placed here, and camera is 0.72 above that
        // With body height of 1.8m, feet will be at surface_y + 10.0 - 0.9 = surface_y + 9.1
        let safe_y = surface_y + 10.0;
        let spawn_pos = Point3::new(spawn_x, safe_y.clamp(20.0, 250.0), spawn_z);
        
        log::info!("[SpawnFinder] Selected spawn position at {:?} (surface height: {})", spawn_pos, surface_y);
        
        // Try to pregenerate the spawn area (non-blocking, best effort)
        let _ = world.pregenerate_spawn_area(spawn_pos, 2);
        
        Ok(spawn_pos)
    }
    
    /// Find safe height at a specific x,z position by checking actual blocks
    /// Returns Y coordinate where player's feet should be (on top of solid block)
    fn find_safe_height_at(world: &dyn WorldInterface, x: f32, z: f32) -> Option<f32> {
        let block_x = x.floor() as i32;
        let block_z = z.floor() as i32;
        
        // Get estimated height to check the right chunk
        let estimated_y = world.get_surface_height(x as f64, z as f64);
        
        // Try to get blocks anyway - the world might generate them on demand
        log::debug!("[SpawnFinder] Checking column at ({}, {}) with estimated height {}", x, z, estimated_y);
        
        // Start from estimated height and search both up and down
        let mut last_solid_y = None;
        
        for y in 1..255 {
            let pos = VoxelPos::new(block_x, y, block_z);
            let block = world.get_block(pos);
            
            // Check if this is a solid block
            let is_solid = block != BlockId::AIR && block != BlockId(6); // Not air or water
            
            if is_solid {
                last_solid_y = Some(y);
            } else if let Some(solid_y) = last_solid_y {
                // We found air above solid ground
                // Check if there's enough space for the player (2 blocks high)
                let above_pos = VoxelPos::new(block_x, y + 1, block_z);
                let above_block = world.get_block(above_pos);
                
                if above_block == BlockId::AIR || above_block == BlockId(6) {
                    // Safe spawn: standing on solid_y, with 2 air blocks above
                    // Return the position where feet are (on top of the solid block)
                    let spawn_y = solid_y as f32 + 1.0;
                    
                    log::debug!(
                        "[SpawnFinder] Found safe spawn at ({}, {}, {}): solid at y={}, air at y={} and y={}",
                        x, spawn_y, z, solid_y, y, y + 1
                    );
                    
                    return Some(spawn_y);
                }
            }
        }
        
        // If we couldn't find a safe spot, check if we at least found solid ground
        if let Some(solid_y) = last_solid_y {
            log::warn!(
                "[SpawnFinder] No safe spawn with clearance at ({}, {}), but found solid at y={}",
                x, z, solid_y
            );
            // Return position above the highest solid block + safety margin
            Some(solid_y as f32 + 3.0)
        } else {
            log::warn!("[SpawnFinder] No solid ground found at ({}, {})", x, z);
            None
        }
    }
    
    /// Verify spawn position after chunks are loaded and adjust if needed
    /// This can be called after the initial spawn to ensure player isn't stuck
    pub fn verify_spawn_position(world: &dyn WorldInterface, current_pos: Point3<f32>) -> Point3<f32> {
        let x = current_pos.x;
        let z = current_pos.z;
        
        // Check if we can read blocks at this position now
        if let Some(safe_y) = Self::find_safe_height_at(world, x, z) {
            if (safe_y - current_pos.y).abs() > 2.0 {
                log::info!("[SpawnFinder] Adjusting spawn height from {} to {} after chunk load", current_pos.y, safe_y);
                return Point3::new(x, safe_y, z);
            }
        }
        
        // Check current position is safe
        // Note: current_pos is the physics body center, feet are 0.9m below
        let feet_y = current_pos.y - 0.9;
        let feet_block = world.get_block(VoxelPos::new(x.floor() as i32, feet_y.floor() as i32, z.floor() as i32));
        let below_block = world.get_block(VoxelPos::new(x.floor() as i32, (feet_y - 1.0).floor() as i32, z.floor() as i32));
        
        if feet_block != BlockId::AIR {
            log::warn!("[SpawnFinder] Player feet inside block! Moving up...");
            return Point3::new(x, current_pos.y + 2.0, z);
        }
        
        current_pos
    }
    
    /// Debug function to log what blocks are at a position
    pub fn debug_blocks_at_position(world: &dyn WorldInterface, pos: Point3<f32>) {
        let x = pos.x.floor() as i32;
        let y = pos.y.floor() as i32;
        let z = pos.z.floor() as i32;
        
        // Check chunk status
        let chunk_pos = VoxelPos::new(x, y, z).to_chunk_pos(world.chunk_size());
        let chunk_loaded = world.is_chunk_loaded(chunk_pos);
        
        log::info!("[SpawnFinder] Blocks around position {:?} (chunk {:?} loaded: {})", 
                  pos, chunk_pos, chunk_loaded);
        
        // Show player position relative to block
        // Note: pos is the physics body center position
        let player_feet_y = pos.y - 0.9;  // Body is 1.8m tall, center to feet is 0.9m
        let player_head_y = pos.y + 0.9;  // Center to head is 0.9m
        log::info!("[SpawnFinder] Physics body center at y={:.2}, feet at y={:.2}, head at y={:.2}", 
                  pos.y, player_feet_y, player_head_y);
        
        for dy in -3..=3 {
            let check_y = y + dy;
            let voxel_pos = VoxelPos::new(x, check_y, z);
            let block = world.get_block(voxel_pos);
            let block_name = match block {
                BlockId::AIR => "AIR",
                BlockId(1) => "GRASS",
                BlockId(2) => "DIRT", 
                BlockId(3) => "STONE",
                BlockId(5) => "SAND",
                BlockId(6) => "WATER",
                _ => "UNKNOWN",
            };
            
            // Mark which blocks the player intersects with
            let marker = if check_y as f32 >= player_feet_y && (check_y as f32) < player_head_y {
                " <- PLAYER HERE"
            } else {
                ""
            };
            
            log::info!("  y={}: {} (id={}){}", check_y, block_name, block.0, marker);
        }
    }
}