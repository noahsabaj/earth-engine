use crate::{BlockId, VoxelPos};
use crate::world::WorldInterface;
use cgmath::Point3;

/// Utility for finding safe spawn positions in the world
pub struct SpawnFinder;

impl SpawnFinder {
    /// Find a safe spawn position using noise-based terrain height
    /// Returns the position where the player's feet should be (standing on solid ground)
    pub fn find_safe_spawn(
        world: &dyn WorldInterface,
        start_x: f32,
        start_z: f32,
        search_radius: i32,
    ) -> Result<Point3<f32>, String> {
        log::info!("[SpawnFinder] Starting spawn search at ({}, ??, {})", start_x, start_z);
        
        // Get the surface height at the exact spawn position
        let surface_height = world.get_surface_height(start_x as f64, start_z as f64) as f32;
        
        // Spawn player's body center just above the surface
        // Player body is 1.8m tall, so center is 0.9m above feet
        // Add 1 block (1.0m) clearance above the surface for safety
        let feet_y = surface_height + 1.0;
        let body_center_y = feet_y + 0.9; // Body center is 0.9m above feet
        
        let spawn_pos = Point3::new(start_x, body_center_y.clamp(20.0, 250.0), start_z);
        
        log::info!("[SpawnFinder] Surface height at ({}, {}): y={}", start_x, start_z, surface_height);
        log::info!("[SpawnFinder] Spawning player with feet at y={}, body center at y={}", feet_y, body_center_y);
        log::info!("[SpawnFinder] Player will be standing {} blocks above surface", feet_y - surface_height);
        
        // ALWAYS verify the spawn is safe by checking actual blocks
        // This is critical to avoid spawning inside blocks when GPU generation fails
        match Self::find_safe_height_at(world, start_x, start_z) {
            Some(verified_height) => {
                let verified_center_y = verified_height + 0.9; // Body center 0.9m above feet
                log::info!("[SpawnFinder] Verified spawn height: feet at y={}, body center at y={}", verified_height, verified_center_y);
                
                // Always use the verified height to avoid spawning in blocks
                Ok(Point3::new(start_x, verified_center_y.clamp(20.0, 250.0), start_z))
            }
            None => {
                log::error!("[SpawnFinder] CRITICAL: Could not verify safe spawn position!");
                log::error!("[SpawnFinder] GPU terrain generation may have failed - using emergency spawn");
                
                // Emergency spawn: high in the air to avoid being stuck
                let emergency_y = 100.0;
                log::warn!("[SpawnFinder] Using emergency spawn at y={} to avoid being stuck in terrain", emergency_y);
                Ok(Point3::new(start_x, emergency_y, start_z))
            }
        }
    }
    
    /// Find safe height at a specific x,z position by checking actual blocks
    /// Returns Y coordinate where player's feet should be (on top of solid block)
    fn find_safe_height_at(world: &dyn WorldInterface, x: f32, z: f32) -> Option<f32> {
        let block_x = x.floor() as i32;
        let block_z = z.floor() as i32;
        
        // Get estimated height to check the right chunk
        let estimated_y = world.get_surface_height(x as f64, z as f64);
        
        log::info!("[SpawnFinder] Verifying spawn safety at ({}, {}) with estimated height {}", x, z, estimated_y);
        
        // Check blocks around the estimated height first
        let search_start = (estimated_y as i32 - 10).max(1);
        let search_end = (estimated_y as i32 + 20).min(254);
        
        // First pass: look for the topmost solid block
        let mut highest_solid_y = None;
        
        for y in (search_start..search_end).rev() {
            let pos = VoxelPos::new(block_x, y, block_z);
            let block = world.get_block(pos);
            
            // Check if this is a solid block
            let is_solid = block != BlockId::AIR && block != BlockId(6); // Not air or water
            
            if is_solid && highest_solid_y.is_none() {
                // Found the highest solid block in the search range
                highest_solid_y = Some(y);
                
                // Check if we have 2 blocks of air above for player height
                let above1_pos = VoxelPos::new(block_x, y + 1, block_z);
                let above2_pos = VoxelPos::new(block_x, y + 2, block_z);
                let above1_block = world.get_block(above1_pos);
                let above2_block = world.get_block(above2_pos);
                
                let is_air_above1 = above1_block == BlockId::AIR || above1_block == BlockId(6);
                let is_air_above2 = above2_block == BlockId::AIR || above2_block == BlockId(6);
                
                if is_air_above1 && is_air_above2 {
                    // Safe spawn: standing on y, with 2 air blocks above
                    let spawn_y = y as f32 + 1.0; // Feet position on top of block
                    
                    log::info!(
                        "[SpawnFinder] Found safe spawn at ({}, {}, {}): solid at y={}, clear air above",
                        x, spawn_y, z, y
                    );
                    
                    return Some(spawn_y);
                } else {
                    log::warn!(
                        "[SpawnFinder] Found solid at y={} but no clearance above (blocks: {:?}, {:?})",
                        y, above1_block, above2_block
                    );
                }
            }
        }
        
        // If we couldn't find a safe spot in the initial range, do a full search
        log::warn!("[SpawnFinder] No safe spawn found in range {}..{}, doing full search", search_start, search_end);
        
        // Full search from top to bottom
        for y in (1..255).rev() {
            let pos = VoxelPos::new(block_x, y, block_z);
            let block = world.get_block(pos);
            
            // Check if this is a solid block
            let is_solid = block != BlockId::AIR && block != BlockId(6);
            
            if is_solid {
                // Check clearance above
                let above1_pos = VoxelPos::new(block_x, y + 1, block_z);
                let above2_pos = VoxelPos::new(block_x, y + 2, block_z);
                let above1_block = world.get_block(above1_pos);
                let above2_block = world.get_block(above2_pos);
                
                if (above1_block == BlockId::AIR || above1_block == BlockId(6)) &&
                   (above2_block == BlockId::AIR || above2_block == BlockId(6)) {
                    let spawn_y = y as f32 + 1.0;
                    log::info!("[SpawnFinder] Found safe spawn in full search at y={}", spawn_y);
                    return Some(spawn_y);
                }
            }
        }
        
        log::error!("[SpawnFinder] CRITICAL: No safe spawn found at ({}, {}) after full search!", x, z);
        None
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