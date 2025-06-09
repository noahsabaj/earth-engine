use earth_engine::{
    world::{ParallelWorld, ParallelWorldConfig, DefaultWorldGenerator, Chunk},
    renderer::{ChunkMesher, AsyncMeshBuilder, MeshBuildRequest},
    BlockId, BlockRegistry, Block, RenderData, PhysicsProperties,
};
use cgmath::Point3;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

// Test blocks
struct TestBlock {
    id: BlockId,
    color: [f32; 3],
}

impl Block for TestBlock {
    fn get_id(&self) -> BlockId { self.id }
    fn get_render_data(&self) -> RenderData {
        RenderData {
            color: self.color,
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> PhysicsProperties {
        PhysicsProperties {
            solid: true,
            density: 1000.0,
        }
    }
    fn get_name(&self) -> &str { "Test" }
}

fn main() {
    println!("Earth Engine - Async Mesh Building Benchmark");
    println!("==========================================");
    
    // Create block registry
    let mut registry = BlockRegistry::new();
    let grass_id = registry.register("test:grass", TestBlock { 
        id: BlockId(1), 
        color: [0.3, 0.7, 0.2] 
    });
    let dirt_id = registry.register("test:dirt", TestBlock { 
        id: BlockId(2), 
        color: [0.5, 0.3, 0.1] 
    });
    let stone_id = registry.register("test:stone", TestBlock { 
        id: BlockId(3), 
        color: [0.6, 0.6, 0.6] 
    });
    
    let registry = Arc::new(registry);
    let chunk_size = 32;
    
    // Create test world
    let generator = Box::new(DefaultWorldGenerator::new(
        12345,
        grass_id,
        dirt_id,
        stone_id,
        BlockId(4), // water
        BlockId(5), // sand
    ));
    
    let config = ParallelWorldConfig {
        generation_threads: 4,
        mesh_threads: 4,
        chunks_per_frame: 16,
        view_distance: 4,
        chunk_size,
    };
    
    let world = Arc::new(ParallelWorld::new(generator, config));
    
    // Pregenerate chunks
    println!("Generating test chunks...");
    world.pregenerate_spawn_area(Point3::new(0.0, 100.0, 0.0), 4);
    
    let chunk_manager = world.chunk_manager();
    let chunks_to_mesh: Vec<_> = chunk_manager
        .chunks_iter()
        .take(100)
        .collect();
    
    println!("Generated {} chunks to mesh", chunks_to_mesh.len());
    println!();
    
    // Test 1: Synchronous mesh building
    println!("Test 1: Synchronous Mesh Building");
    println!("---------------------------------");
    
    let start_time = Instant::now();
    let mut sync_meshes = 0;
    let mut sync_vertices = 0;
    
    for (_, chunk_lock) in &chunks_to_mesh {
        let chunk = chunk_lock.read();
        let mesh = ChunkMesher::generate_mesh(&chunk, &registry);
        sync_meshes += 1;
        sync_vertices += mesh.vertices.len();
    }
    
    let sync_time = start_time.elapsed();
    println!("  Meshes built: {}", sync_meshes);
    println!("  Total vertices: {}", sync_vertices);
    println!("  Time: {:.2}s", sync_time.as_secs_f32());
    println!("  Meshes/second: {:.2}", sync_meshes as f32 / sync_time.as_secs_f32());
    println!();
    
    // Test 2: Async mesh building with 1 thread
    println!("Test 2: Async Mesh Building (1 thread)");
    println!("---------------------------------------");
    
    let async_builder = AsyncMeshBuilder::new(
        Arc::clone(&registry),
        chunk_size,
        Some(1),
    );
    
    // Queue all chunks
    for (pos, chunk) in &chunks_to_mesh {
        async_builder.queue_chunk(
            *pos,
            Arc::clone(chunk),
            0,
            Default::default(),
        );
    }
    
    let start_time = Instant::now();
    let mut async1_meshes = 0;
    let mut async1_vertices = 0;
    
    // Process all chunks
    while async1_meshes < chunks_to_mesh.len() {
        async_builder.process_queue(16);
        
        let completed = async_builder.get_completed_meshes();
        for mesh in completed {
            async1_meshes += 1;
            async1_vertices += mesh.vertex_count;
        }
        
        std::thread::sleep(Duration::from_millis(10));
    }
    
    let async1_time = start_time.elapsed();
    println!("  Meshes built: {}", async1_meshes);
    println!("  Total vertices: {}", async1_vertices);
    println!("  Time: {:.2}s", async1_time.as_secs_f32());
    println!("  Meshes/second: {:.2}", async1_meshes as f32 / async1_time.as_secs_f32());
    println!();
    
    // Test 3: Async mesh building with optimal threads
    println!("Test 3: Async Mesh Building (optimal threads)");
    println!("--------------------------------------------");
    
    let async_builder_opt = AsyncMeshBuilder::new(
        Arc::clone(&registry),
        chunk_size,
        None, // Auto-detect optimal thread count
    );
    
    // Queue all chunks
    for (pos, chunk) in &chunks_to_mesh {
        async_builder_opt.queue_chunk(
            *pos,
            Arc::clone(chunk),
            0,
            Default::default(),
        );
    }
    
    let start_time = Instant::now();
    let mut async_opt_meshes = 0;
    let mut async_opt_vertices = 0;
    
    // Process all chunks
    while async_opt_meshes < chunks_to_mesh.len() {
        async_builder_opt.process_queue(32);
        
        let completed = async_builder_opt.get_completed_meshes();
        for mesh in completed {
            async_opt_meshes += 1;
            async_opt_vertices += mesh.vertex_count;
        }
        
        std::thread::sleep(Duration::from_millis(5));
    }
    
    let async_opt_time = start_time.elapsed();
    let stats = async_builder_opt.get_stats();
    
    println!("  Meshes built: {}", async_opt_meshes);
    println!("  Total vertices: {}", async_opt_vertices);
    println!("  Time: {:.2}s", async_opt_time.as_secs_f32());
    println!("  Meshes/second: {:.2}", async_opt_meshes as f32 / async_opt_time.as_secs_f32());
    println!("  Average build time: {:.2}ms", stats.average_build_time.as_millis());
    println!();
    
    // Summary
    println!("Summary");
    println!("-------");
    let speedup_1thread = sync_time.as_secs_f32() / async1_time.as_secs_f32();
    let speedup_optimal = sync_time.as_secs_f32() / async_opt_time.as_secs_f32();
    
    println!("  Synchronous: {:.2}s", sync_time.as_secs_f32());
    println!("  Async (1 thread): {:.2}s ({:.1}x speedup)", async1_time.as_secs_f32(), speedup_1thread);
    println!("  Async (optimal): {:.2}s ({:.1}x speedup)", async_opt_time.as_secs_f32(), speedup_optimal);
    
    // Test 4: Stress test with priority
    println!("\nTest 4: Priority-based Mesh Building");
    println!("------------------------------------");
    
    let priority_builder = AsyncMeshBuilder::new(
        Arc::clone(&registry),
        chunk_size,
        None,
    );
    
    // Queue chunks with different priorities
    for (i, (pos, chunk)) in chunks_to_mesh.iter().enumerate() {
        let priority = if i < 10 { 0 } else if i < 30 { 1 } else { 2 };
        priority_builder.queue_chunk(
            *pos,
            Arc::clone(chunk),
            priority,
            Default::default(),
        );
    }
    
    let start_time = Instant::now();
    let mut priority_order = Vec::new();
    
    // Process and track completion order
    while priority_order.len() < chunks_to_mesh.len() {
        priority_builder.process_queue(8);
        
        let completed = priority_builder.get_completed_meshes();
        for mesh in completed {
            priority_order.push(mesh.chunk_pos);
        }
        
        std::thread::sleep(Duration::from_millis(5));
    }
    
    let priority_time = start_time.elapsed();
    
    println!("  Time: {:.2}s", priority_time.as_secs_f32());
    println!("  High priority chunks completed first: {}", 
        priority_order.iter().take(10).all(|pos| {
            chunks_to_mesh.iter().take(10).any(|(p, _)| p == pos)
        })
    );
}