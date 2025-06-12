use earth_engine::{
    BlockId, BlockRegistry, VoxelPos, ChunkPos,
    world::{DefaultWorldGenerator, ParallelWorld, ParallelWorldConfig, SpawnFinder, WorldInterface},
};
use cgmath::Point3;
use std::sync::Arc;

fn main() {
    env_logger::init();
    
    // Create block registry
    let mut block_registry = BlockRegistry::new();
    
    // Register basic blocks
    let grass_id = block_registry.register("grass", TestGrassBlock);
    let dirt_id = block_registry.register("dirt", TestDirtBlock);
    let stone_id = block_registry.register("stone", TestStoneBlock);
    let water_id = block_registry.register("water", TestWaterBlock);
    let sand_id = block_registry.register("sand", TestSandBlock);
    
    println!("Registered blocks:");
    println!("  Grass: {:?}", grass_id);
    println!("  Dirt: {:?}", dirt_id);
    println!("  Stone: {:?}", stone_id);
    println!("  Water: {:?}", water_id);
    println!("  Sand: {:?}", sand_id);
    
    // Create world generator
    let generator = Box::new(DefaultWorldGenerator::new(
        12345, // seed
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    // Create parallel world with minimal config
    let config = ParallelWorldConfig {
        generation_threads: 2,
        mesh_threads: 2,
        chunks_per_frame: 4,
        view_distance: 2,
        chunk_size: 32,
    };
    
    println!("\nCreating parallel world...");
    let world = ParallelWorld::new(generator, config);
    
    // Test spawn finding at origin
    println!("\nFinding safe spawn position at (0, 0)...");
    match SpawnFinder::find_safe_spawn(&world, 0.0, 0.0, 5) {
        Ok(spawn_pos) => {
            println!("Found safe spawn position: {:?}", spawn_pos);
            println!("\nDebugging blocks at spawn position:");
            SpawnFinder::debug_blocks_at_position(&world, spawn_pos);
            
            // Also check what the noise-based generator would have returned
            let noise_height = world.get_surface_height(0.0, 0.0);
            println!("\nNoise-based surface height: {}", noise_height);
            println!("Safe spawn Y: {}", spawn_pos.y);
            println!("Difference: {} blocks", spawn_pos.y - noise_height as f32);
        }
        Err(e) => {
            eprintln!("Failed to find spawn: {}", e);
        }
    }
    
    // Test a few more positions
    println!("\n\nTesting additional positions:");
    for (x, z) in [(10.0, 10.0), (-20.0, 5.0), (30.0, -15.0)] {
        println!("\nFinding spawn at ({}, {})...", x, z);
        match SpawnFinder::find_safe_spawn(&world, x, z, 3) {
            Ok(pos) => {
                println!("  Found: {:?}", pos);
                let noise_height = world.get_surface_height(x as f64, z as f64);
                println!("  Noise height: {}, Safe Y: {}, Diff: {}", 
                        noise_height, pos.y, pos.y - noise_height as f32);
            }
            Err(e) => {
                println!("  Failed: {}", e);
            }
        }
    }
}

// Test block implementations
struct TestGrassBlock;
impl earth_engine::Block for TestGrassBlock {
    fn get_id(&self) -> BlockId { BlockId(1) }
    fn get_render_data(&self) -> earth_engine::RenderData {
        earth_engine::RenderData {
            color: [0.3, 0.7, 0.2],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> earth_engine::PhysicsProperties {
        earth_engine::PhysicsProperties {
            solid: true,
            density: 1200.0,
        }
    }
    fn get_name(&self) -> &str { "Grass" }
}

struct TestDirtBlock;
impl earth_engine::Block for TestDirtBlock {
    fn get_id(&self) -> BlockId { BlockId(2) }
    fn get_render_data(&self) -> earth_engine::RenderData {
        earth_engine::RenderData {
            color: [0.5, 0.3, 0.1],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> earth_engine::PhysicsProperties {
        earth_engine::PhysicsProperties {
            solid: true,
            density: 1500.0,
        }
    }
    fn get_name(&self) -> &str { "Dirt" }
}

struct TestStoneBlock;
impl earth_engine::Block for TestStoneBlock {
    fn get_id(&self) -> BlockId { BlockId(3) }
    fn get_render_data(&self) -> earth_engine::RenderData {
        earth_engine::RenderData {
            color: [0.6, 0.6, 0.6],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> earth_engine::PhysicsProperties {
        earth_engine::PhysicsProperties {
            solid: true,
            density: 2500.0,
        }
    }
    fn get_name(&self) -> &str { "Stone" }
}

struct TestWaterBlock;
impl earth_engine::Block for TestWaterBlock {
    fn get_id(&self) -> BlockId { BlockId(6) }
    fn get_render_data(&self) -> earth_engine::RenderData {
        earth_engine::RenderData {
            color: [0.1, 0.4, 0.8],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> earth_engine::PhysicsProperties {
        earth_engine::PhysicsProperties {
            solid: false,
            density: 1000.0,
        }
    }
    fn get_name(&self) -> &str { "Water" }
}

struct TestSandBlock;
impl earth_engine::Block for TestSandBlock {
    fn get_id(&self) -> BlockId { BlockId(5) }
    fn get_render_data(&self) -> earth_engine::RenderData {
        earth_engine::RenderData {
            color: [0.9, 0.8, 0.6],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> earth_engine::PhysicsProperties {
        earth_engine::PhysicsProperties {
            solid: true,
            density: 1600.0,
        }
    }
    fn get_name(&self) -> &str { "Sand" }
}