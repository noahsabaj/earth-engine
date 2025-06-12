use earth_engine::{
    ChunkPos, Chunk, BlockId, BlockRegistry, Camera,
    renderer::{SimpleAsyncRenderer, AsyncMeshBuilder},
    world::{World, DefaultWorldGenerator},
};
use std::sync::Arc;
use cgmath::Point3;

// Simple test blocks
struct TestBlock {
    id: BlockId,
    name: &'static str,
    color: [f32; 3],
}

impl earth_engine::Block for TestBlock {
    fn get_id(&self) -> BlockId { self.id }
    fn get_render_data(&self) -> earth_engine::RenderData {
        earth_engine::RenderData {
            color: self.color,
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> earth_engine::PhysicsProperties {
        earth_engine::PhysicsProperties {
            solid: true,
            density: 1000.0,
        }
    }
    fn get_name(&self) -> &str { self.name }
}

fn main() {
    // Initialize logging
    env_logger::init();
    
    println!("Testing async mesh building integration...");
    
    // Create block registry
    let mut registry = BlockRegistry::new();
    let grass_id = registry.register("test:grass", TestBlock {
        id: BlockId(1),
        name: "Grass",
        color: [0.3, 0.7, 0.2],
    });
    let dirt_id = registry.register("test:dirt", TestBlock {
        id: BlockId(2),
        name: "Dirt",
        color: [0.5, 0.3, 0.1],
    });
    let stone_id = registry.register("test:stone", TestBlock {
        id: BlockId(3),
        name: "Stone",
        color: [0.6, 0.6, 0.6],
    });
    
    let registry = Arc::new(registry);
    
    // Create world
    let chunk_size = 32;
    let view_distance = 4;
    let generator = Box::new(DefaultWorldGenerator::new(
        12345, // seed
        grass_id,
        dirt_id,
        stone_id,
        BlockId(4), // water
        BlockId(5), // sand
    ));
    
    let mut world = World::new_with_generator(chunk_size, view_distance, generator);
    
    // Create camera
    let mut camera = Camera::new(800, 600);
    camera.position = Point3::new(0.0, 64.0, 0.0);
    
    // Load initial chunks
    world.update_loaded_chunks(camera.position);
    
    // Create async renderer
    let mut renderer = SimpleAsyncRenderer::new(
        Arc::clone(&registry),
        chunk_size,
        Some(4), // Use 4 threads
    );
    
    println!("Initial world chunks: {}", world.chunks().len());
    
    // Simulate a few frames of updates
    for frame in 0..5 {
        println!("\nFrame {}:", frame);
        
        // Simulate some block changes to create dirty chunks
        if frame == 2 {
            // Place some blocks
            world.set_block(earth_engine::VoxelPos::new(10, 20, 10), stone_id);
            world.set_block(earth_engine::VoxelPos::new(11, 20, 10), stone_id);
            world.set_block(earth_engine::VoxelPos::new(12, 20, 10), stone_id);
            println!("  Modified 3 blocks");
        }
        
        // Queue dirty chunks for async processing
        renderer.queue_dirty_chunks(&mut world, &camera);
        
        // Simulate GPU device (we won't actually upload, just process)
        // In real usage, this would upload meshes to GPU
        println!("  Queued builds: {}", renderer.queued_builds());
        
        // Sleep to simulate frame time and let async threads work
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // Check mesh builder stats
        println!("  Active builds: {}", renderer.queued_builds());
        println!("  GPU meshes: {}", renderer.mesh_count());
    }
    
    println!("\nAsync mesh building test completed successfully!");
    println!("Final stats:");
    println!("  Total GPU meshes: {}", renderer.mesh_count());
    println!("  Queued builds remaining: {}", renderer.queued_builds());
}