/// Example of integrating the data-oriented mesh builder with the rendering pipeline
/// 
/// This demonstrates:
/// - Using the mesh buffer pool for zero-allocation mesh building
/// - Converting between MeshBuffer and ChunkMesh formats
/// - Batch processing multiple chunks
/// - Proper neighbor handling for face culling

use earth_engine::{
    ChunkPos, BlockId, BlockRegistry,
    world::{ChunkSoA, DefaultWorldGenerator, WorldGenerator},
    renderer::{
        build_chunk_mesh_dop, mesh_buffer_to_chunk_mesh,
        NeighborData, ChunkMeshBatch, MESH_BUFFER_POOL,
    },
};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Instant;

fn main() {
    env_logger::init();
    
    // Create block registry
    let mut registry = BlockRegistry::new();
    registry.register("earth:stone", BlockId(1), Default::default());
    registry.register("earth:dirt", BlockId(2), Default::default());
    registry.register("earth:grass", BlockId(3), Default::default());
    let registry = Arc::new(registry);
    
    // Create world generator
    let generator = DefaultWorldGenerator::new(12345);
    
    // Generate a 3x3 grid of chunks
    let mut chunks = Vec::new();
    for x in -1..=1 {
        for z in -1..=1 {
            let pos = ChunkPos::new(x, 0, z);
            let mut chunk = ChunkSoA::new(pos, 32);
            
            // Generate terrain
            generator.generate_chunk(&mut chunk);
            
            chunks.push((pos, Arc::new(RwLock::new(chunk))));
        }
    }
    
    println!("Generated {} chunks", chunks.len());
    
    // Example 1: Build a single chunk mesh with neighbors
    example_single_chunk(&chunks, &registry);
    
    // Example 2: Batch build multiple chunks
    example_batch_build(&chunks, &registry);
    
    // Show pool statistics
    println!("\nMesh buffer pool created {} buffers total", 
        MESH_BUFFER_POOL.total_created());
}

fn example_single_chunk(chunks: &[(ChunkPos, Arc<RwLock<ChunkSoA>>)], registry: &BlockRegistry) {
    println!("\n=== Single Chunk Mesh Building ===");
    
    // Get the center chunk (0, 0, 0)
    let center_chunk = chunks.iter()
        .find(|(pos, _)| pos.x == 0 && pos.y == 0 && pos.z == 0)
        .unwrap();
    
    // Find neighbors
    let north = chunks.iter().find(|(pos, _)| pos.x == 0 && pos.z == -1).map(|(_, c)| c);
    let south = chunks.iter().find(|(pos, _)| pos.x == 0 && pos.z == 1).map(|(_, c)| c);
    let east = chunks.iter().find(|(pos, _)| pos.x == 1 && pos.z == 0).map(|(_, c)| c);
    let west = chunks.iter().find(|(pos, _)| pos.x == -1 && pos.z == 0).map(|(_, c)| c);
    
    let start = Instant::now();
    
    // Build mesh with neighbors
    let mesh_buffer = {
        let chunk = center_chunk.1.read();
        let neighbors = NeighborData {
            north: north.map(|c| &*c.read()).as_deref(),
            south: south.map(|c| &*c.read()).as_deref(),
            east: east.map(|c| &*c.read()).as_deref(),
            west: west.map(|c| &*c.read()).as_deref(),
            up: None,
            down: None,
        };
        
        build_chunk_mesh_dop(&*chunk, neighbors, registry)
    };
    
    let build_time = start.elapsed();
    
    println!("Built mesh for chunk (0, 0, 0):");
    println!("  Vertices: {}", mesh_buffer.vertex_count);
    println!("  Indices: {}", mesh_buffer.index_count);
    println!("  Build time: {:?}", build_time);
    println!("  Generation time from metadata: {} µs", mesh_buffer.metadata.generation_time_us);
    
    // Convert to ChunkMesh if needed for rendering
    let chunk_mesh = mesh_buffer_to_chunk_mesh(&mesh_buffer);
    println!("  Converted to ChunkMesh: {} vertices, {} indices", 
        chunk_mesh.vertices.len(), chunk_mesh.indices.len());
    
    // Return buffer to pool
    MESH_BUFFER_POOL.release(mesh_buffer);
}

fn example_batch_build(chunks: &[(ChunkPos, Arc<RwLock<ChunkSoA>>)], registry: &BlockRegistry) {
    println!("\n=== Batch Chunk Mesh Building ===");
    
    let start = Instant::now();
    
    // Create batch processor
    let mut batch = ChunkMeshBatch::new(chunks.len());
    
    // Add all chunks to batch
    for (pos, chunk) in chunks {
        batch.add_chunk(*pos, Arc::clone(chunk));
    }
    
    // Build all meshes in parallel
    batch.build_all(registry);
    
    let build_time = start.elapsed();
    
    // Get results
    let meshes = batch.take_meshes();
    
    println!("Built {} chunk meshes in {:?}", meshes.len(), build_time);
    
    // Statistics
    let total_vertices: usize = meshes.iter().map(|m| m.vertex_count).sum();
    let total_indices: usize = meshes.iter().map(|m| m.index_count).sum();
    let avg_build_time: u32 = meshes.iter()
        .map(|m| m.metadata.generation_time_us)
        .sum::<u32>() / meshes.len() as u32;
    
    println!("Statistics:");
    println!("  Total vertices: {}", total_vertices);
    println!("  Total indices: {}", total_indices);
    println!("  Average vertices per chunk: {}", total_vertices / meshes.len());
    println!("  Average build time per chunk: {} µs", avg_build_time);
    
    // Return all buffers to pool
    for buffer in meshes {
        MESH_BUFFER_POOL.release(buffer);
    }
}

/// Extension to MeshBufferPool for statistics
impl earth_engine::renderer::MeshBufferPool {
    pub fn total_created(&self) -> usize {
        // This would need to be added to the actual implementation
        // For now, just return a placeholder
        0
    }
}