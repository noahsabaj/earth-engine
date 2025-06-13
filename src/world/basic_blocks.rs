use crate::world::{Block, BlockId, RenderData, PhysicsProperties};

/// Basic Air block
#[derive(Debug, Clone)]
pub struct AirBlock;

impl Block for AirBlock {
    fn get_id(&self) -> BlockId {
        BlockId::AIR
    }
    
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.0, 0.0, 0.0],
            texture_id: 0,
        }
    }
    
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: false,
            density: 0.0,
        }
    }
    
    fn get_name(&self) -> &str {
        "Air"
    }
    
    fn is_transparent(&self) -> bool {
        true
    }
}

/// Basic Stone block
#[derive(Debug, Clone)]
pub struct StoneBlock;

impl Block for StoneBlock {
    fn get_id(&self) -> BlockId {
        BlockId::STONE
    }
    
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.5, 0.5, 0.5],
            texture_id: 1,
        }
    }
    
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 2.5,
        }
    }
    
    fn get_name(&self) -> &str {
        "Stone"
    }
    
    fn get_hardness(&self) -> f32 {
        3.0
    }
}

/// Basic Grass block
#[derive(Debug, Clone)]
pub struct GrassBlock;

impl Block for GrassBlock {
    fn get_id(&self) -> BlockId {
        BlockId::GRASS
    }
    
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.3, 0.7, 0.2],
            texture_id: 2,
        }
    }
    
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1.0,
        }
    }
    
    fn get_name(&self) -> &str {
        "Grass"
    }
    
    fn get_hardness(&self) -> f32 {
        1.5
    }
}