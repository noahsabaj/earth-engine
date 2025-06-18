use std::sync::Arc;
use hearth_engine::prelude::*;
use hearth_engine::gpu::soa::TerrainParamsSOA;
use hearth_engine::world_gpu::{TerrainGeneratorSOA, WorldBuffer, WorldBufferDescriptor};

fn main() {
    // Initialize logging
    env_logger::init();
    
    println!("=== Testing SOA Terrain Generation ===");
    
    // Create instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    // Get adapter
    let adapter = pollster::block_on(async {
        instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }).await
    });
    
    let adapter = match adapter {
        Some(a) => a,
        None => {
            eprintln!("Failed to find GPU adapter");
            return;
        }
    };
    
    println!("GPU Adapter: {}", adapter.get_info().name);
    
    // Create device and queue
    let (device, queue) = pollster::block_on(async {
        adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None
        ).await
    }).unwrap();
    
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    
    println!("Creating SOA terrain generator...");
    
    // Create the SOA terrain generator
    let generator = TerrainGeneratorSOA::new(device.clone(), queue.clone());
    
    println!("Creating world buffer...");
    
    // Create world buffer
    let world_buffer_desc = WorldBufferDescriptor {
        max_chunks: 64,
        chunk_size: 50,
    };
    let mut world_buffer = WorldBuffer::new(device.clone(), &world_buffer_desc);
    
    println!("Generating terrain at origin...");
    
    // Create command encoder
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Test Encoder"),
    });
    
    // Generate a chunk at origin
    let chunk_pos = ChunkPos::new(0, 0, 0);
    generator.generate_chunk(&mut encoder, &mut world_buffer, chunk_pos);
    
    // Submit
    queue.submit(std::iter::once(encoder.finish()));
    
    // Wait for GPU
    device.poll(wgpu::Maintain::Wait);
    
    println!("Terrain generation complete!");
    
    // Try to read back some data to verify
    println!("\nReading back chunk data...");
    
    // Create a staging buffer to read back results
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: 50 * 50 * 50 * 4, // One chunk worth of u32s
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    // Copy from world buffer to staging
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Copy Encoder"),
    });
    
    encoder.copy_buffer_to_buffer(
        world_buffer.voxel_buffer(),
        0,
        &staging_buffer,
        0,
        50 * 50 * 50 * 4,
    );
    
    queue.submit(std::iter::once(encoder.finish()));
    
    // Map and read
    let buffer_slice = staging_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    
    {
        let data = buffer_slice.get_mapped_range();
        let voxels: &[u32] = bytemuck::cast_slice(&data);
        
        // Count non-air blocks
        let non_air_count = voxels.iter().filter(|&&v| v != 0).count();
        println!("Non-air blocks in chunk: {}", non_air_count);
        
        // Show first few blocks
        println!("First 10 voxels: {:?}", &voxels[..10]);
    }
    
    drop(staging_buffer);
    
    println!("\n=== Test Complete ===");
}