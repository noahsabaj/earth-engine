//! Legacy adapter for world::WorldInterface compatibility
//!
//! This adapter allows world_unified to be used where world::WorldInterface is expected,
//! enabling gradual migration from the old world module to the unified architecture.

use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use cgmath::Point3;

use crate::world::{WorldInterface as LegacyWorldInterface, BlockId, VoxelPos, ChunkPos};
use crate::world_unified::{
    management::UnifiedWorldManager,
    core::Block,
};

/// Adapter that implements the legacy WorldInterface using UnifiedWorldManager
pub struct LegacyWorldAdapter {
    manager: Arc<Mutex<UnifiedWorldManager>>,
    chunk_size: u32,
    dirty_chunks: Arc<Mutex<HashSet<ChunkPos>>>,
}

impl LegacyWorldAdapter {
    /// Create a new legacy adapter
    pub fn new(manager: Arc<Mutex<UnifiedWorldManager>>, chunk_size: u32) -> Self {
        Self {
            manager,
            chunk_size,
            dirty_chunks: Arc::new(Mutex::new(HashSet::new())),
        }
    }
    
    /// Mark a chunk as dirty (needs remeshing)
    fn mark_chunk_dirty(&self, chunk_pos: ChunkPos) {
        if let Ok(mut dirty) = self.dirty_chunks.lock() {
            dirty.insert(chunk_pos);
        }
    }
    
    /// Convert unified BlockId to legacy BlockId
    fn to_legacy_block_id(&self, block: crate::world_unified::core::BlockId) -> BlockId {
        BlockId(block.0)
    }
    
    /// Convert legacy BlockId to unified BlockId
    fn from_legacy_block_id(&self, block: BlockId) -> crate::world_unified::core::BlockId {
        crate::world_unified::core::BlockId(block.0)
    }
    
    /// Convert legacy VoxelPos to unified VoxelPos
    fn from_legacy_voxel_pos(&self, pos: VoxelPos) -> crate::world_unified::core::VoxelPos {
        crate::world_unified::core::VoxelPos::new(pos.x, pos.y, pos.z)
    }
    
    /// Convert legacy ChunkPos to unified ChunkPos
    fn from_legacy_chunk_pos(&self, pos: ChunkPos) -> crate::world_unified::core::ChunkPos {
        crate::world_unified::core::ChunkPos { x: pos.x, y: pos.y, z: pos.z }
    }
}

impl LegacyWorldInterface for LegacyWorldAdapter {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        if let Ok(manager) = self.manager.lock() {
            let unified_pos = self.from_legacy_voxel_pos(pos);
            let unified_block = manager.get_block(unified_pos);
            self.to_legacy_block_id(unified_block)
        } else {
            BlockId::AIR
        }
    }
    
    fn set_block(&mut self, pos: VoxelPos, block: BlockId) {
        if let Ok(mut manager) = self.manager.lock() {
            let unified_pos = self.from_legacy_voxel_pos(pos);
            let unified_block = self.from_legacy_block_id(block);
            
            // Set the block in the unified manager
            if manager.set_block(unified_pos, unified_block).is_ok() {
                // Mark the chunk as dirty for remeshing
                let chunk_pos = ChunkPos {
                    x: pos.x.div_euclid(self.chunk_size as i32),
                    y: pos.y.div_euclid(self.chunk_size as i32),
                    z: pos.z.div_euclid(self.chunk_size as i32),
                };
                self.mark_chunk_dirty(chunk_pos);
            }
        }
    }
    
    fn update_loaded_chunks(&mut self, player_pos: Point3<f32>) {
        // Convert player position to chunk position
        let chunk_pos = ChunkPos {
            x: (player_pos.x / self.chunk_size as f32).floor() as i32,
            y: (player_pos.y / self.chunk_size as f32).floor() as i32,
            z: (player_pos.z / self.chunk_size as f32).floor() as i32,
        };
        
        if let Ok(mut manager) = self.manager.lock() {
            // Load chunks around the player
            let view_distance = 8; // Default view distance
            for x in -view_distance..=view_distance {
                for y in -2..=2 { // Limited vertical range
                    for z in -view_distance..=view_distance {
                        let load_pos = self.from_legacy_chunk_pos(ChunkPos {
                            x: chunk_pos.x + x,
                            y: chunk_pos.y + y,
                            z: chunk_pos.z + z,
                        });
                        
                        // Attempt to load the chunk if not already loaded
                        let _ = manager.load_chunk(load_pos);
                    }
                }
            }
        }
    }
    
    fn chunk_size(&self) -> u32 {
        self.chunk_size
    }
    
    fn is_block_in_bounds(&self, _pos: VoxelPos) -> bool {
        true // Infinite world
    }
    
    fn get_sky_light(&self, pos: VoxelPos) -> u8 {
        // TODO: Implement lighting in unified world
        // For now, return full sky light for air blocks, 0 for solid
        if self.get_block(pos) == BlockId::AIR {
            15
        } else {
            0
        }
    }
    
    fn set_sky_light(&mut self, pos: VoxelPos, _level: u8) {
        // TODO: Implement lighting in unified world
        // Mark chunk as dirty for now
        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(self.chunk_size as i32),
            y: pos.y.div_euclid(self.chunk_size as i32),
            z: pos.z.div_euclid(self.chunk_size as i32),
        };
        self.mark_chunk_dirty(chunk_pos);
    }
    
    fn get_block_light(&self, _pos: VoxelPos) -> u8 {
        // TODO: Implement lighting in unified world
        0
    }
    
    fn set_block_light(&mut self, pos: VoxelPos, _level: u8) {
        // TODO: Implement lighting in unified world
        // Mark chunk as dirty for now
        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(self.chunk_size as i32),
            y: pos.y.div_euclid(self.chunk_size as i32),
            z: pos.z.div_euclid(self.chunk_size as i32),
        };
        self.mark_chunk_dirty(chunk_pos);
    }
    
    fn is_chunk_loaded(&self, pos: ChunkPos) -> bool {
        if let Ok(manager) = self.manager.lock() {
            manager.is_chunk_loaded(self.from_legacy_chunk_pos(pos))
        } else {
            false
        }
    }
    
    fn take_dirty_chunks(&mut self) -> HashSet<ChunkPos> {
        if let Ok(mut dirty) = self.dirty_chunks.lock() {
            std::mem::take(&mut *dirty)
        } else {
            HashSet::new()
        }
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        if let Ok(manager) = self.manager.lock() {
            manager.get_surface_height(world_x, world_z)
        } else {
            64 // Default sea level
        }
    }
    
    fn is_block_transparent(&self, pos: VoxelPos) -> bool {
        // Check if block is air or has transparency
        let block_id = self.get_block(pos);
        
        // TODO: Use block registry to check transparency properly
        // For now, only air is transparent
        block_id == BlockId::AIR
    }
    
    fn ensure_camera_chunk_loaded(&mut self, camera_pos: Point3<f32>) -> bool {
        let chunk_pos = ChunkPos {
            x: (camera_pos.x / self.chunk_size as f32).floor() as i32,
            y: (camera_pos.y / self.chunk_size as f32).floor() as i32,
            z: (camera_pos.z / self.chunk_size as f32).floor() as i32,
        };
        
        if let Ok(mut manager) = self.manager.lock() {
            let unified_pos = self.from_legacy_chunk_pos(chunk_pos);
            
            // Check if already loaded
            if manager.is_chunk_loaded(unified_pos) {
                return true;
            }
            
            // Try to load the chunk
            manager.load_chunk(unified_pos).is_ok()
        } else {
            false
        }
    }
}

// Ensure thread safety
unsafe impl Send for LegacyWorldAdapter {}
unsafe impl Sync for LegacyWorldAdapter {}