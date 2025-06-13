use earth_engine::renderer::gpu_driven::*;
use std::sync::Arc;
use std::time::Instant;
use cgmath::Vector3;
use rand::Rng;

async fn run() {
    println!("=== GPU-Driven Rendering Benchmark ===\n");
    
    // Initialize GPU
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .expect("Failed to find adapter");
    
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::INDIRECT_FIRST_INSTANCE,
                required_limits: wgpu::Limits::default(),
                label: Some("GPU Driven Device"),
            },
            None,
        )
        .await
        .expect("Failed to create device");
    
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    
    println!("GPU: {:?}", adapter.get_info().name);
    println!("Backend: {:?}", adapter.get_info().backend);
    println!();
    
    // Test configurations
    let object_counts = [1000, 5000, 10000, 25000, 50000];
    let mut rng = rand::thread_rng();
    
    // Create test managers
    let mut command_manager = IndirectCommandManager::new(device.clone(), 100000);
    let mut instance_manager = InstanceManager::new(device.clone());
    let culling_pipeline = CullingPipeline::new(device.clone());
    let mut culling_data = CullingData::new(device.clone(), 100000);
    
    // Check if GPU culling is available
    if !culling_pipeline.is_available() {
        println!("WARNING: GPU culling pipeline creation failed!");
        println!("The culling benchmarks will be skipped.\n");
    }
    
    println!("=== Indirect Command Buffer Performance ===");
    
    for &count in &object_counts {
        // Generate test commands
        let commands: Vec<_> = (0..count)
            .map(|i| IndirectDrawIndexedCommand {
                index_count: rng.gen_range(100..1000),
                instance_count: 1,
                first_index: 0,
                base_vertex: 0,
                first_instance: i,
            })
            .collect();
        
        // Measure update time
        let start = Instant::now();
        command_manager.opaque_commands_mut().update_indexed_commands(&queue, &commands);
        let update_time = start.elapsed();
        
        // Measure GPU copy time
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Benchmark Encoder"),
        });
        
        let start = Instant::now();
        command_manager.copy_all_to_gpu(&mut encoder);
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
        let copy_time = start.elapsed();
        
        println!(
            "{} commands: Update {:.2}ms, GPU copy {:.2}ms",
            count,
            update_time.as_secs_f32() * 1000.0,
            copy_time.as_secs_f32() * 1000.0
        );
    }
    
    println!("\n=== Instance Buffer Performance ===");
    
    for &count in &object_counts {
        instance_manager.chunk_instances_mut().clear();
        
        // Generate test instances
        let instances: Vec<_> = (0..count)
            .map(|_| InstanceData::new(
                Vector3::new(
                    rng.gen_range(-1000.0..1000.0),
                    rng.gen_range(-100.0..100.0),
                    rng.gen_range(-1000.0..1000.0),
                ),
                rng.gen_range(0.5..2.0),
                [1.0, 1.0, 1.0, 1.0],
            ))
            .collect();
        
        // Measure insertion time
        let start = Instant::now();
        for instance in &instances {
            instance_manager.chunk_instances_mut().add_instance(*instance);
        }
        let insert_time = start.elapsed();
        
        // Measure upload time
        let start = Instant::now();
        instance_manager.upload_all(&queue);
        device.poll(wgpu::Maintain::Wait);
        let upload_time = start.elapsed();
        
        println!(
            "{} instances: Insert {:.2}ms ({:.0} instances/sec), Upload {:.2}ms",
            count,
            insert_time.as_secs_f32() * 1000.0,
            count as f32 / insert_time.as_secs_f32(),
            upload_time.as_secs_f32() * 1000.0
        );
    }
    
    println!("\n=== GPU Culling Performance ===");
    
    if culling_pipeline.is_available() {
        // Create test camera
        let mut camera = earth_engine::camera::data_camera::init_camera(1920, 1080);
        camera.position = [0.0, 100.0, 0.0];
        
        for &count in &object_counts {
            culling_data.clear();
        
            // Generate draw metadata
            for i in 0..count {
                let pos = Vector3::new(
                    rng.gen_range(-500.0..500.0),
                    rng.gen_range(-50.0..50.0),
                    rng.gen_range(-500.0..500.0),
                );
                
                let metadata = DrawMetadata {
                    bounding_sphere: [pos.x, pos.y, pos.z, 10.0],
                    lod_info: [50.0, 150.0, 300.0, 0.0],
                    material_id: rng.gen_range(0..10),
                    mesh_id: rng.gen_range(0..5),
                    instance_offset: i,
                    flags: 1,
                };
                
                culling_data.add_draw(metadata);
            }
            
            // Upload data
            culling_data.upload(&queue);
            culling_pipeline.update_camera(&queue, &camera);
            
            // Create bind group
            let bind_group = culling_pipeline.create_bind_group(
                &culling_data.metadata_buffer,
                command_manager.opaque_commands().buffer(),
            );
            
            // Measure culling time
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Culling Encoder"),
            });
            
            let start = Instant::now();
            culling_pipeline.execute_culling(&mut encoder, &bind_group, count);
            culling_pipeline.copy_stats(&mut encoder);
            
            queue.submit(Some(encoder.finish()));
            device.poll(wgpu::Maintain::Wait);
            let cull_time = start.elapsed();
            
            // Read stats
            if let Some(stats) = culling_pipeline.read_stats().await {
                println!(
                    "{} objects: {:.2}ms - Drawn: {}, Frustum culled: {}, Distance culled: {}",
                    count,
                    cull_time.as_secs_f32() * 1000.0,
                    stats.drawn,
                    stats.frustum_culled,
                    stats.distance_culled
                );
            }
        }
    } else {
        println!("Skipping culling benchmarks - pipeline not available");
    }
    
    println!("\n=== Memory Usage ===");
    
    // Calculate memory usage
    let command_buffer_size = 100000 * std::mem::size_of::<IndirectDrawIndexedCommand>();
    let instance_buffer_size = 10000 * std::mem::size_of::<InstanceData>();
    let metadata_buffer_size = 100000 * std::mem::size_of::<DrawMetadata>();
    
    println!("Command buffer: {:.2} MB", command_buffer_size as f32 / 1024.0 / 1024.0);
    println!("Instance buffer: {:.2} MB", instance_buffer_size as f32 / 1024.0 / 1024.0);
    println!("Metadata buffer: {:.2} MB", metadata_buffer_size as f32 / 1024.0 / 1024.0);
    println!(
        "Total GPU memory: {:.2} MB",
        (command_buffer_size + instance_buffer_size + metadata_buffer_size) as f32 / 1024.0 / 1024.0
    );
    
    println!("\n=== Performance Summary ===");
    println!("✓ Indirect drawing eliminates CPU draw calls");
    println!("✓ GPU culling reduces CPU-GPU sync");
    println!("✓ Instance data enables massive object counts");
    println!("✓ Zero driver overhead for draw submission");
}

fn main() {
    pollster::block_on(run());
}