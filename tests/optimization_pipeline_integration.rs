/// Integration test for optimization pipeline
/// 
/// Tests that Morton encoding, GPU culling, and mesh optimization
/// work together correctly from Sprints 27-29.

use earth_engine::world::{Chunk, BlockId, ChunkPos};
use earth_engine::morton::{morton_encode_chunk, morton_decode_chunk};
use earth_engine::renderer::{MeshOptimizer, MeshLod};

#[test]
fn test_morton_chunk_to_mesh_pipeline() {
    // Create a test chunk with some blocks
    let mut chunk = Chunk::new_empty();
    
    // Add some blocks in a pattern
    for x in 10..20 {
        for y in 10..20 {
            for z in 10..20 {
                chunk.set_block(x, y, z, BlockId(1));
            }
        }
    }
    
    // Test Morton encoding/decoding
    for x in 0..32 {
        for y in 0..32 {
            for z in 0..32 {
                let pos = earth_engine::world::VoxelPos { x, y, z };
                let morton = morton_encode_chunk(pos);
                let decoded = morton_decode_chunk(morton);
                
                assert_eq!(pos, decoded, "Morton encoding/decoding failed at {:?}", pos);
            }
        }
    }
    
    // Test mesh generation (without GPU)
    let mesh_optimizer = MeshOptimizer::new_cpu_only();
    let mesh_data = mesh_optimizer.optimize_chunk_mesh(&chunk, 100.0);
    
    // Verify mesh was generated
    assert!(!mesh_data.vertices.is_empty(), "No vertices generated");
    assert!(!mesh_data.indices.is_empty(), "No indices generated");
    
    // Verify greedy meshing reduced triangle count
    // A 10x10x10 cube should have much fewer than 10*10*10*12 triangles
    let triangle_count = mesh_data.indices.len() / 3;
    assert!(triangle_count < 1000, "Greedy meshing didn't reduce triangles enough");
    
    println!("Generated {} triangles for 10x10x10 cube", triangle_count);
}

#[test]
fn test_lod_generation() {
    let mut chunk = Chunk::new_empty();
    
    // Create a more complex pattern
    for x in 0..32 {
        for y in 0..16 {
            for z in 0..32 {
                if (x + z) % 4 == 0 {
                    chunk.set_block(x, y, z, BlockId(1));
                }
            }
        }
    }
    
    let mesh_optimizer = MeshOptimizer::new_cpu_only();
    
    // Generate meshes at different LODs
    let lod0 = mesh_optimizer.generate_lod_mesh(&chunk, MeshLod::Lod0);
    let lod1 = mesh_optimizer.generate_lod_mesh(&chunk, MeshLod::Lod1);
    let lod2 = mesh_optimizer.generate_lod_mesh(&chunk, MeshLod::Lod2);
    
    // Verify LODs have decreasing complexity
    assert!(lod0.indices.len() > lod1.indices.len(), "LOD1 should have fewer triangles than LOD0");
    assert!(lod1.indices.len() > lod2.indices.len(), "LOD2 should have fewer triangles than LOD1");
    
    println!("LOD0: {} triangles", lod0.indices.len() / 3);
    println!("LOD1: {} triangles", lod1.indices.len() / 3);
    println!("LOD2: {} triangles", lod2.indices.len() / 3);
}

#[test]
fn test_chunk_cache_with_morton_key() {
    let mesh_optimizer = MeshOptimizer::new_cpu_only();
    let mut chunk = Chunk::new_empty();
    chunk.set_block(16, 16, 16, BlockId(1));
    
    let pos = ChunkPos { x: 10, y: 5, z: 20 };
    
    // First generation should create mesh
    let mesh1 = mesh_optimizer.optimize_chunk_mesh_at_pos(&chunk, pos, 50.0);
    
    // Second generation should hit cache
    let mesh2 = mesh_optimizer.optimize_chunk_mesh_at_pos(&chunk, pos, 50.0);
    
    // Meshes should be identical (same vertices and indices)
    assert_eq!(mesh1.vertices.len(), mesh2.vertices.len());
    assert_eq!(mesh1.indices.len(), mesh2.indices.len());
}