use crate::world::{BlockId, BlockRegistry, VoxelPos, WorldInterface, Ray, RaycastHit, cast_ray};
use crate::camera::{CameraData, calculate_forward_vector};
use crate::input::InputState;
use cgmath::Point3;

/// Main game trait that games must implement
pub trait Game: Send + Sync {
    /// Called once at startup to register blocks
    fn register_blocks(&mut self, registry: &mut BlockRegistry);
    
    /// Called every frame
    fn update(&mut self, ctx: &mut GameContext, delta_time: f32);
    
    /// Called when a block is broken
    fn on_block_break(&mut self, pos: VoxelPos, block: BlockId) {
        let _ = (pos, block); // Default implementation does nothing
    }
    
    /// Called when a block is placed
    fn on_block_place(&mut self, pos: VoxelPos, block: BlockId) {
        let _ = (pos, block); // Default implementation does nothing
    }
    
    /// Get the block ID that should be placed when right-clicking
    fn get_active_block(&self) -> BlockId {
        BlockId(1) // Default to first registered block
    }
}

/// Context passed to game update functions
pub struct GameContext<'a> {
    pub world: &'a mut dyn WorldInterface,
    pub registry: &'a BlockRegistry,
    pub camera: &'a CameraData,
    pub input: &'a InputState,
    pub selected_block: Option<RaycastHit>,
}

impl<'a> GameContext<'a> {
    /// Cast a ray from the camera and find what block is being looked at
    pub fn cast_camera_ray(&self, max_distance: f32) -> Option<RaycastHit> {
        let position = Point3::new(
            self.camera.position[0], 
            self.camera.position[1], 
            self.camera.position[2]
        );
        let forward = calculate_forward_vector(self.camera);
        let ray = Ray::new(position, forward);
        cast_ray(&*self.world, ray, max_distance)
    }
    
    /// Break a block at the given position
    pub fn break_block(&mut self, pos: VoxelPos) -> bool {
        let block = self.world.get_block(pos);
        if block != BlockId::AIR {
            self.world.set_block(pos, BlockId::AIR);
            true
        } else {
            false
        }
    }
    
    /// Place a block at the given position
    pub fn place_block(&mut self, pos: VoxelPos, block_id: BlockId) -> bool {
        let current = self.world.get_block(pos);
        if current == BlockId::AIR {
            self.world.set_block(pos, block_id);
            true
        } else {
            false
        }
    }
}