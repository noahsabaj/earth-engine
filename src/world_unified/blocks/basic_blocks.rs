//! Basic engine blocks for the unified world system
//! 
//! This module defines the fundamental blocks that come with the engine.
//! Games can register additional blocks on top of these.

use crate::world_unified::core::{Block, BlockId, BlockRegistry, RenderData, PhysicsProperties};

/// Grass block - the classic surface block
pub struct GrassBlock;

impl Block for GrassBlock {
    fn get_id(&self) -> BlockId {
        BlockId::GRASS
    }
    
    fn get_name(&self) -> &str {
        "grass"
    }
    
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.3, 0.8, 0.2], // Green grass color
            texture_id: 1,
            light_emission: 0,
        }
    }
    
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1500.0, // kg/m³
        }
    }
    
    fn get_light_emission(&self) -> u8 {
        0
    }
    
    fn get_hardness(&self) -> f32 {
        0.6 // Quick to break
    }
}

/// Dirt block - found beneath grass
pub struct DirtBlock;

impl Block for DirtBlock {
    fn get_id(&self) -> BlockId {
        BlockId::DIRT
    }
    
    fn get_name(&self) -> &str {
        "dirt"
    }
    
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.5, 0.3, 0.1], // Brown dirt color
            texture_id: 2,
            light_emission: 0,
        }
    }
    
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1600.0,
        }
    }
    
    fn get_hardness(&self) -> f32 {
        0.5
    }
}

/// Stone block - the foundation of the world
pub struct StoneBlock;

impl Block for StoneBlock {
    fn get_id(&self) -> BlockId {
        BlockId::STONE
    }
    
    fn get_name(&self) -> &str {
        "stone"
    }
    
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.5, 0.5, 0.5], // Gray stone color
            texture_id: 3,
            light_emission: 0,
        }
    }
    
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 2500.0,
        }
    }
    
    fn get_hardness(&self) -> f32 {
        1.5 // Harder to break
    }
}

/// Water block - transparent liquid
pub struct WaterBlock;

impl Block for WaterBlock {
    fn get_id(&self) -> BlockId {
        BlockId::WATER
    }
    
    fn get_name(&self) -> &str {
        "water"
    }
    
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.2, 0.3, 0.8], // Blue water color
            texture_id: 4,
            light_emission: 0,
        }
    }
    
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: false,
            density: 1000.0,
        }
    }
    
    fn is_transparent(&self) -> bool {
        true // Water is transparent
    }
    
    fn get_hardness(&self) -> f32 {
        100.0 // Can't break water
    }
}

/// Sand block - granular material
pub struct SandBlock;

impl Block for SandBlock {
    fn get_id(&self) -> BlockId {
        BlockId::SAND
    }
    
    fn get_name(&self) -> &str {
        "sand"
    }
    
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.9, 0.8, 0.6], // Sandy color
            texture_id: 5,
            light_emission: 0,
        }
    }
    
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1800.0,
        }
    }
    
    fn get_hardness(&self) -> f32 {
        0.5
    }
}

/// Glowstone block - emits light
pub struct GlowstoneBlock;

impl Block for GlowstoneBlock {
    fn get_id(&self) -> BlockId {
        BlockId(6) // First non-engine block ID
    }
    
    fn get_name(&self) -> &str {
        "glowstone"
    }
    
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [1.0, 0.9, 0.6], // Bright yellow color
            texture_id: 6,
            light_emission: 15,
        }
    }
    
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 2000.0,
        }
    }
    
    fn get_light_emission(&self) -> u8 {
        15 // Maximum light level
    }
    
    fn get_hardness(&self) -> f32 {
        0.8
    }
}

/// Register all basic engine blocks
/// 
/// This function registers the fundamental blocks that come with the engine.
/// Games should call this before registering their own blocks.
pub fn register_basic_blocks(registry: &mut BlockRegistry) {
    // Note: Air (BlockId 0) is handled specially by the engine
    
    // Register terrain blocks
    registry.register("engine:grass", GrassBlock);
    registry.register("engine:dirt", DirtBlock);
    registry.register("engine:stone", StoneBlock);
    registry.register("engine:water", WaterBlock);
    registry.register("engine:sand", SandBlock);
    registry.register("engine:glowstone", GlowstoneBlock);
}