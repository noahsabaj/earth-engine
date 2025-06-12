// Demonstration of async mesh building integration
use earth_engine::{
    ChunkPos, Chunk, BlockId, BlockRegistry,
    renderer::{AsyncMeshBuilder, MeshBuildRequest, ChunkMesher},
};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, Instant};

// Simple test block
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
    println!("=== Async Mesh Building Demo ===\n");
    
    // 1. Create block registry
    let mut registry = BlockRegistry::new();
    let stone_id = registry.register("test:stone", TestBlock {
        id: BlockId(1),
        name: "Stone",
        color: [0.6, 0.6, 0.6],
    });
    let registry = Arc::new(registry);
    
    // 2. Create async mesh builder
    let chunk_size = 32;
    let mesh_builder = AsyncMeshBuilder::new(
        Arc::clone(&registry),
        chunk_size,
        Some(4), // Use 4 threads
    );
    
    println!("Created async mesh builder with 4 threads");
    
    // 3. Create test chunks
    let mut chunks = Vec::new();
    for x in 0..3 {
        for z in 0..3 {
            let pos = ChunkPos::new(x, 0, z);
            let mut chunk = Chunk::new(pos, chunk_size);
            
            // Fill bottom layer with stone
            for bx in 0..chunk_size {
                for bz in 0..chunk_size {
                    chunk.set_block(bx, 0, bz, stone_id);
                }
            }
            
            chunks.push((pos, Arc::new(RwLock::new(chunk))));
        }
    }
    
    println!("Created {} test chunks\n", chunks.len());
    
    // 4. Queue chunks for async mesh building
    println!("Queueing chunks for mesh building...");
    let queue_start = Instant::now();
    
    for (i, (pos, chunk)) in chunks.iter().enumerate() {
        let priority = i as i32; // Lower = higher priority
        
        // Create empty neighbor array (no neighbors for this demo)
        let neighbors: [Option<Arc<RwLock<Chunk>>>; 6] = Default::default();
        
        mesh_builder.queue_chunk(
            *pos,
            Arc::clone(chunk),
            priority,
            neighbors,
        );
    }
    
    let queue_time = queue_start.elapsed();
    println!("Queued {} chunks in {:?}", chunks.len(), queue_time);
    
    // 5. Process mesh building
    println!("\nProcessing mesh queue...");
    let process_start = Instant::now();
    
    // Process all chunks
    mesh_builder.process_queue(chunks.len());
    
    // Wait for completion (in a real app, this would be non-blocking)
    std::thread::sleep(Duration::from_millis(100));
    
    // 6. Collect completed meshes
    let completed_meshes = mesh_builder.get_completed_meshes();
    let process_time = process_start.elapsed();
    
    println!("Processed {} meshes in {:?}", completed_meshes.len(), process_time);
    
    // 7. Show statistics
    let stats = mesh_builder.get_stats();
    println!("\n=== Mesh Building Statistics ===");
    println!("Total meshes built: {}", stats.meshes_built);
    println!("Average build time: {:?}", stats.average_build_time);
    println!("Meshes per second: {:.2}", stats.meshes_per_second);
    println!("Total vertices: {}", stats.total_vertices);
    println!("Total faces: {}", stats.total_faces);
    
    // 8. Show individual mesh details
    println!("\n=== Individual Mesh Results ===");
    for (i, mesh) in completed_meshes.iter().enumerate() {
        println!("Chunk {:?}:", mesh.chunk_pos);
        println!("  Build time: {:?}", mesh.build_time);
        println!("  Vertices: {}", mesh.mesh.vertices.len());
        println!("  Indices: {}", mesh.mesh.indices.len());
        println!("  Triangles: {}", mesh.mesh.indices.len() / 3);
    }
    
    println!("\n=== Demo Complete ===");
    
    // Show how this integrates with rendering:
    println!("\nIntegration with rendering:");
    println!("1. Queue dirty chunks when blocks change");
    println!("2. Process queue each frame (non-blocking)");
    println!("3. Upload completed meshes to GPU");
    println!("4. Render uploaded meshes with frustum culling");
    
    println!("\nKey benefits:");
    println!("- Main thread never blocks on mesh generation");
    println!("- Multiple chunks processed in parallel");
    println!("- Priority system ensures nearby chunks built first");
    println!("- Easy to integrate with existing renderer");
}