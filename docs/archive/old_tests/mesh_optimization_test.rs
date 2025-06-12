/// Performance benchmarks for mesh optimization systems
/// Sprint 29: Mesh Optimization & Advanced LOD

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use earth_engine::renderer::{
    GreedyMesher, MeshOptimizer, TextureAtlas, 
    MeshSimplifier, AdaptiveTessellator, MeshCompressor,
    ProgressiveEncoder, GeomorphLod, Vertex, MeshLod,
    TessellationParams, CompressionOptions,
};
use earth_engine::world::{Chunk, BlockId};
use earth_engine::sdf::SdfBuffer;
use cgmath::Vector3;
use std::collections::HashMap;

fn generate_test_chunk(density: f32) -> Chunk {
    let mut chunk = Chunk::new_empty();
    for x in 0..16 {
        for y in 0..16 {
            for z in 0..16 {
                if rand::random::<f32>() < density {
                    chunk.set_block(x, y, z, BlockId(1));
                }
            }
        }
    }
    chunk
}

fn generate_test_vertices(count: usize) -> Vec<Vertex> {
    (0..count).map(|i| {
        let angle = i as f32 * 0.1;
        Vertex {
            position: [angle.cos() * 10.0, i as f32 * 0.1, angle.sin() * 10.0],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [(i % 16) as f32 / 16.0, (i / 16) as f32 / 16.0],
            color: [1.0, 1.0, 1.0, 1.0],
            ao: 1.0,
        }
    }).collect()
}

fn generate_test_indices(vertex_count: usize) -> Vec<u32> {
    let mut indices = Vec::new();
    for i in 0..(vertex_count - 2) {
        indices.extend_from_slice(&[i as u32, (i + 1) as u32, (i + 2) as u32]);
    }
    indices
}

fn bench_greedy_meshing(c: &mut Criterion) {
    let mesher = GreedyMesher::new();
    
    c.bench_function("greedy_meshing_sparse", |b| {
        let chunk = generate_test_chunk(0.1);
        b.iter(|| {
            black_box(mesher.generate_mesh(&chunk));
        });
    });
    
    c.bench_function("greedy_meshing_medium", |b| {
        let chunk = generate_test_chunk(0.3);
        b.iter(|| {
            black_box(mesher.generate_mesh(&chunk));
        });
    });
    
    c.bench_function("greedy_meshing_dense", |b| {
        let chunk = generate_test_chunk(0.6);
        b.iter(|| {
            black_box(mesher.generate_mesh(&chunk));
        });
    });
}

fn bench_mesh_simplification(c: &mut Criterion) {
    c.bench_function("mesh_simplify_1000_to_100", |b| {
        let vertices = generate_test_vertices(1000);
        let indices = generate_test_indices(1000);
        
        b.iter(|| {
            let mut simplifier = MeshSimplifier::new(&vertices, &indices);
            black_box(simplifier.simplify(100));
        });
    });
    
    c.bench_function("mesh_simplify_10000_to_1000", |b| {
        let vertices = generate_test_vertices(10000);
        let indices = generate_test_indices(10000);
        
        b.iter(|| {
            let mut simplifier = MeshSimplifier::new(&vertices, &indices);
            black_box(simplifier.simplify(1000));
        });
    });
}

fn bench_texture_atlasing(c: &mut Criterion) {
    let mut atlas = TextureAtlas::new(2048, 2048, 4);
    
    c.bench_function("texture_atlas_insertion", |b| {
        let mut texture_id = 0;
        b.iter(|| {
            let data = vec![255u8; 64 * 64 * 4];
            texture_id += 1;
            black_box(atlas.add_texture(texture_id, 64, 64, &data));
        });
    });
    
    c.bench_function("texture_atlas_packing", |b| {
        // Pre-populate atlas
        for i in 0..100 {
            let size = 16 + (i % 8) * 8;
            let data = vec![255u8; size * size * 4];
            atlas.add_texture(i, size, size, &data);
        }
        
        b.iter(|| {
            black_box(atlas.pack());
        });
    });
}

fn bench_mesh_compression(c: &mut Criterion) {
    let compressor = MeshCompressor::new(CompressionOptions::default());
    
    c.bench_function("mesh_compress_1000_vertices", |b| {
        let vertices = generate_test_vertices(1000);
        let indices = generate_test_indices(1000);
        
        b.iter(|| {
            black_box(compressor.compress(&vertices, &indices).unwrap());
        });
    });
    
    c.bench_function("mesh_decompress_1000_vertices", |b| {
        let vertices = generate_test_vertices(1000);
        let indices = generate_test_indices(1000);
        let compressed = compressor.compress(&vertices, &indices).unwrap();
        
        b.iter(|| {
            black_box(MeshDecompressor::decompress(&compressed).unwrap());
        });
    });
}

fn bench_adaptive_tessellation(c: &mut Criterion) {
    let tessellator = AdaptiveTessellator::new(TessellationParams::default());
    let sdf = SdfBuffer::new(128, 128, 128);
    let region = (Vector3::new(0.0, 0.0, 0.0), Vector3::new(100.0, 100.0, 100.0));
    let view_pos = Vector3::new(50.0, 50.0, 150.0);
    
    c.bench_function("adaptive_tessellation_near", |b| {
        let near_view = Vector3::new(50.0, 50.0, 20.0);
        b.iter(|| {
            black_box(tessellator.tessellate_sdf(
                &sdf,
                region,
                near_view,
                (1920.0, 1080.0),
                45.0f32.to_radians(),
            ));
        });
    });
    
    c.bench_function("adaptive_tessellation_far", |b| {
        let far_view = Vector3::new(50.0, 50.0, 500.0);
        b.iter(|| {
            black_box(tessellator.tessellate_sdf(
                &sdf,
                region,
                far_view,
                (1920.0, 1080.0),
                45.0f32.to_radians(),
            ));
        });
    });
}

fn bench_progressive_streaming(c: &mut Criterion) {
    let encoder = ProgressiveEncoder::new(4096);
    
    c.bench_function("progressive_encode_multi_lod", |b| {
        let mut lod_meshes = HashMap::new();
        lod_meshes.insert(MeshLod::Lod4, (generate_test_vertices(100), generate_test_indices(100)));
        lod_meshes.insert(MeshLod::Lod3, (generate_test_vertices(200), generate_test_indices(200)));
        lod_meshes.insert(MeshLod::Lod2, (generate_test_vertices(400), generate_test_indices(400)));
        lod_meshes.insert(MeshLod::Lod1, (generate_test_vertices(800), generate_test_indices(800)));
        lod_meshes.insert(MeshLod::Lod0, (generate_test_vertices(1600), generate_test_indices(1600)));
        
        b.iter(|| {
            black_box(encoder.encode_progressive(12345, lod_meshes.clone()));
        });
    });
}

fn bench_lod_transitions(c: &mut Criterion) {
    let mut geomorph = GeomorphLod::new(10.0, 0.5);
    
    // Pre-compute morph targets
    let vertices_high = generate_test_vertices(1000);
    let vertices_low = generate_test_vertices(500);
    let indices_high = generate_test_indices(1000);
    let indices_low = generate_test_indices(500);
    
    geomorph.compute_morph_targets(
        MeshLod::Lod0,
        MeshLod::Lod1,
        &vertices_high,
        &vertices_low,
        &indices_high,
        &indices_low,
    );
    
    c.bench_function("lod_geomorph_apply", |b| {
        let mut vertices = vertices_high.clone();
        geomorph.start_transition(0, MeshLod::Lod0, MeshLod::Lod1);
        
        b.iter(|| {
            let mut work_vertices = vertices.clone();
            black_box(geomorph.apply_morph(0, &mut work_vertices, MeshLod::Lod0, MeshLod::Lod1));
        });
    });
}

fn bench_mesh_optimizer_integration(c: &mut Criterion) {
    let optimizer = MeshOptimizer::new();
    
    c.bench_function("mesh_optimizer_full_pipeline", |b| {
        let chunk = generate_test_chunk(0.3);
        let view_distance = 100.0;
        
        b.iter(|| {
            black_box(optimizer.optimize_chunk_mesh(&chunk, view_distance));
        });
    });
}

criterion_group!(
    benches,
    bench_greedy_meshing,
    bench_mesh_simplification,
    bench_texture_atlasing,
    bench_mesh_compression,
    bench_adaptive_tessellation,
    bench_progressive_streaming,
    bench_lod_transitions,
    bench_mesh_optimizer_integration
);

criterion_main!(benches);