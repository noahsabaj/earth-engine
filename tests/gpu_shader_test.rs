//! Test GPU shader compilation to ensure they compile correctly

use hearth_engine::*;
use std::sync::Arc;

#[test]
fn test_terrain_generation_shader_compiles() {
    // Run async test
    pollster::block_on(async {
        // Create GPU device
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: true, // Use fallback for testing
            })
            .await
            .expect("Failed to find adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Test creating the terrain generator which includes shader compilation
        use hearth_engine::world::generation::TerrainGeneratorSOA;
        use hearth_engine::world::storage::WorldBuffer;
        use hearth_engine::BlockRegistry;

        let world_buffer = WorldBuffer::new(&device, 2);
        let mut registry = BlockRegistry::new();
        hearth_engine::world::register_basic_blocks(&mut registry);

        // This should compile the shader
        let result = TerrainGeneratorSOA::new(
            device.clone(),
            queue.clone(),
            world_buffer,
            registry,
            Default::default(),
        );

        match result {
            Ok(_) => println!("✓ Terrain generation shader compiled successfully"),
            Err(e) => panic!("✗ Shader compilation failed: {}", e),
        }
    });
}

#[test]
fn test_chunk_metadata_structure() {
    use hearth_engine::gpu::buffer_layouts::world::ChunkMetadata;
    use hearth_engine::world::ChunkPos;

    // Test that ChunkMetadata has the expected fields
    let chunk_pos = ChunkPos {
        x: 10,
        y: 5,
        z: -15,
    };
    let metadata = ChunkMetadata::from_chunk_pos(&chunk_pos, 50);

    assert_eq!(metadata.offset[0], 10);
    assert_eq!(metadata.offset[1], 5);
    assert_eq!(metadata.offset[2], -15);
    assert_eq!(metadata.size[0], 50);
    assert_eq!(metadata.size[1], 50);
    assert_eq!(metadata.size[2], 50);
    assert_eq!(metadata.voxel_count, 50 * 50 * 50);

    println!("✓ ChunkMetadata structure is correct");
}

#[test]
fn test_gpu_constants_match() {
    use hearth_engine::core;

    // Verify constants are correct
    assert_eq!(core::CHUNK_SIZE, 50);
    assert_eq!(core::CHUNK_SIZE_F32, 50.0);
    assert_eq!(core::VOXELS_PER_CHUNK, 125000);

    println!("✓ GPU constants match expected values");
}
