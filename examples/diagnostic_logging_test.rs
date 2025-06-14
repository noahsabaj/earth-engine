/// Diagnostic Logging Test
/// 
/// This example demonstrates the comprehensive diagnostic logging system
/// added to the terrain rendering pipeline. It tests all logging components:
/// 
/// 1. GPU Terrain Generation logging
/// 2. CPU Mesh Building logging  
/// 3. GPU Rendering logging
/// 4. GPU-CPU Data Transfer logging
/// 5. Camera Spatial Context logging

use earth_engine::{
    world_gpu::{terrain_generator::TerrainGenerator, world_buffer::{WorldBuffer, WorldBufferDescriptor}},
    renderer::data_mesh_builder::{MESH_BUFFER_POOL, operations::build_chunk_mesh},
    camera::data_camera::{init_camera_with_spawn, diagnostics},
    world::ChunkPos,
    BlockId,
};
use std::sync::Arc;
use wgpu;

fn main() {
    // Initialize logging to see diagnostic output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    
    println!("==================== DIAGNOSTIC LOGGING TEST ====================");
    println!("Testing comprehensive diagnostic logging throughout terrain pipeline");
    
    // Test camera spatial context logging
    test_camera_spatial_context();
    
    // Test mesh builder logging (CPU operations)
    test_mesh_builder_logging();
    
    // Test GPU operations if available
    if let Err(e) = futures::executor::block_on(test_gpu_operations()) {
        println!("❌ GPU operations test failed: {}", e);
        println!("This is expected in environments without GPU compute support");
    }
    
    println!("\n==================== DIAGNOSTIC TEST COMPLETE ====================");
    println!("Check the log output above to verify diagnostic logging is working");
}

fn test_camera_spatial_context() {
    println!("\n=== Testing Camera Spatial Context Logging ===");
    
    // Create camera at various positions to test spatial logging
    let cameras = vec![
        init_camera_with_spawn(1920, 1080, 0.0, 64.0, 0.0),      // Origin chunk
        init_camera_with_spawn(1920, 1080, 100.0, 50.0, 200.0),  // Positive coordinates
        init_camera_with_spawn(1920, 1080, -50.0, 10.0, -80.0),  // Negative coordinates
    ];
    
    for (i, camera) in cameras.iter().enumerate() {
        diagnostics::log_camera_context(camera, &format!("Camera Position Test {}", i + 1));
        
        let chunk_pos = diagnostics::camera_chunk_position(camera);
        let distance = diagnostics::distance_to_chunk(camera, chunk_pos);
        
        println!("Camera {} is in chunk {:?}, distance to chunk center: {:.1} blocks", 
                 i + 1, chunk_pos, distance);
        
        // Test view distance calculation
        let chunks_in_view = diagnostics::chunks_in_view_distance(camera, 2);
        println!("Camera {} can see {} chunks within view distance 2", 
                 i + 1, chunks_in_view.len());
    }
    
    // Test performance logging
    let start = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let duration = start.elapsed();
    
    diagnostics::log_performance_context(
        &cameras[0], 
        "Test Operation", 
        duration.as_secs_f64() * 1000.0, 
        Some(5)
    );
}

fn test_mesh_builder_logging() {
    println!("\n=== Testing Mesh Builder Logging ===");
    
    // Acquire a mesh buffer from the pool (tests pool logging)
    let mut buffer = MESH_BUFFER_POOL.acquire();
    
    // Create a simple test chunk with some blocks
    let chunk_pos = ChunkPos::new(1, 0, 1);
    
    // Test mesh building with logging
    build_chunk_mesh(&mut buffer, chunk_pos, 32, |x, y, z| {
        // Create a simple pattern: ground layer + some scattered blocks
        if y < 5 {
            BlockId::STONE
        } else if y == 5 && x % 4 == 0 && z % 4 == 0 {
            BlockId::GRASS
        } else {
            BlockId::AIR
        }
    });
    
    println!("Mesh building completed: {} vertices, {} indices", 
             buffer.vertex_count, buffer.index_count);
    
    // Return buffer to pool (tests release logging)
    MESH_BUFFER_POOL.release(buffer);
}

async fn test_gpu_operations() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing GPU Operations Logging ===");
    
    // Create GPU context
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or("Failed to find suitable adapter")?;
    
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Diagnostic Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        )
        .await?;
    
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    
    // Test world buffer logging
    println!("Testing WorldBuffer diagnostic logging...");
    let desc = WorldBufferDescriptor {
        view_distance: 2, // Small for testing
        enable_atomics: true,
        enable_readback: true,
    };
    
    let mut world_buffer = WorldBuffer::new(device.clone(), &desc);
    
    // Test terrain generator logging
    println!("Testing TerrainGenerator diagnostic logging...");
    let terrain_generator = TerrainGenerator::new(device.clone());
    
    // Create command encoder for GPU operations
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Diagnostic Test Encoder"),
    });
    
    // Test terrain generation with logging
    let test_chunks = vec![
        ChunkPos::new(0, 0, 0),
        ChunkPos::new(1, 0, 0),
        ChunkPos::new(0, 0, 1),
    ];
    
    terrain_generator.generate_chunks(&mut encoder, &world_buffer, &test_chunks);
    
    // Submit GPU commands
    let submission_start = std::time::Instant::now();
    queue.submit(std::iter::once(encoder.finish()));
    device.poll(wgpu::Maintain::Wait);
    let submission_duration = submission_start.elapsed();
    
    println!("GPU command submission completed in {:.2}ms", 
             submission_duration.as_secs_f64() * 1000.0);
    
    // Test GPU-CPU readback logging
    println!("Testing GPU→CPU readback logging...");
    match world_buffer.read_chunk(&device, &queue, test_chunks[0]) {
        Ok(voxel_data) => {
            println!("Successfully read {} voxels from GPU", voxel_data.len());
        }
        Err(e) => {
            println!("Readback failed (expected in some environments): {}", e);
        }
    }
    
    Ok(())
}