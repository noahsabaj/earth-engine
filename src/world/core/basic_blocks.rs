use super::{Block, BlockId, BlockRegistry, PhysicsProperties, RenderData};

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
            light_emission: 0,
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
            light_emission: 0,
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
            light_emission: 0,
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

/// Basic Dirt block
#[derive(Debug, Clone)]
pub struct DirtBlock;

impl Block for DirtBlock {
    fn get_id(&self) -> BlockId {
        BlockId::DIRT
    }

    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.5, 0.3, 0.1],
            texture_id: 3,
            light_emission: 0,
        }
    }

    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1.5,
        }
    }

    fn get_name(&self) -> &str {
        "Dirt"
    }

    fn get_hardness(&self) -> f32 {
        1.0
    }
}

/// Basic Sand block
#[derive(Debug, Clone)]
pub struct SandBlock;

impl Block for SandBlock {
    fn get_id(&self) -> BlockId {
        BlockId::SAND
    }

    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.9, 0.8, 0.6],
            texture_id: 4,
            light_emission: 0,
        }
    }

    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1.6,
        }
    }

    fn get_name(&self) -> &str {
        "Sand"
    }

    fn get_hardness(&self) -> f32 {
        0.8
    }
}

/// Basic Water block
#[derive(Debug, Clone)]
pub struct WaterBlock;

impl Block for WaterBlock {
    fn get_id(&self) -> BlockId {
        BlockId::WATER
    }

    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.1, 0.3, 0.8],
            texture_id: 5,
            light_emission: 0,
        }
    }

    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: false,
            density: 1.0,
        }
    }

    fn get_name(&self) -> &str {
        "Water"
    }

    fn is_transparent(&self) -> bool {
        true
    }
}

/// Basic Bedrock block (unbreakable)
#[derive(Debug, Clone)]
pub struct BedrockBlock;

impl Block for BedrockBlock {
    fn get_id(&self) -> BlockId {
        BlockId::BEDROCK
    }

    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: [0.2, 0.2, 0.2],
            texture_id: 6,
            light_emission: 0,
        }
    }

    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 10.0,
        }
    }

    fn get_name(&self) -> &str {
        "Bedrock"
    }

    fn get_hardness(&self) -> f32 {
        f32::INFINITY // Unbreakable
    }
}

/// Register all basic engine blocks in the given registry
/// This should be called before games register their custom blocks
pub fn register_basic_blocks(registry: &mut BlockRegistry) {
    // Note: Air (BlockId 0) is handled specially by the engine and doesn't need registration

    // Register terrain blocks
    registry.register("engine:grass", GrassBlock);
    registry.register("engine:dirt", DirtBlock);
    registry.register("engine:stone", StoneBlock);
    registry.register("engine:sand", SandBlock);
    registry.register("engine:water", WaterBlock);
    registry.register("engine:bedrock", BedrockBlock);

    log::info!("[BasicBlocks] Registered 6 basic engine blocks");
}
