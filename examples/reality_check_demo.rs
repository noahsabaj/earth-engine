//! Reality Check Demo - Exposing the truth about Hearth Engine performance
//! 
//! This example shows how to use the RealityCheckProfiler to measure
//! ACTUAL performance, not marketing claims.

use earth_engine::profiling::{
    RealityCheckProfiler, BlockingType, SystemMetrics,
    reality_begin_frame, reality_end_frame, time_cpu_operation,
    record_draw_call, record_compute_dispatch, write_gpu_timestamp,
    generate_reality_report, TrackingAllocator,
};
use earth_engine::renderer::gpu_driven::GpuDrivenRenderer;
use earth_engine::world::{ParallelWorld, ParallelWorldConfig};
use earth_engine::camera::{Camera, CameraData, init_camera};
use earth_engine::EngineConfig;

use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

// Note: TrackingAllocator would need to be set up differently for actual use
// For this demo, we'll create it at runtime

async fn run() {
    env_logger::init();
    
    // Create window
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Hearth Engine Reality Check")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)
            .expect("Failed to create window")
    );
    
    // Initialize renderer
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let surface = unsafe { instance.create_surface(&*window) }.expect("Failed to create surface");
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find adapter");
    
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Reality Check Device"),
                features: wgpu::Features::TIMESTAMP_QUERY,
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");
    
    // Create profiler with GPU support
    let mut profiler = RealityCheckProfiler::new(Some(&device), Some(&queue));
    // Note: In a real application, you'd set up the TrackingAllocator as a global allocator
    // For this demo, we'll just use the profiler without memory tracking
    let allocator = Arc::new(TrackingAllocator::new());
    profiler.set_memory_tracker(allocator);
    
    // Initialize world
    let config = EngineConfig::default();
    let world_config = ParallelWorldConfig {
        chunk_size: 32,
        render_distance: 8,
        max_concurrent_chunks: 4,
        chunk_buffer_size: 64,
        enable_gpu_driven: true,
    };
    
    let mut world = time_cpu_operation(&profiler, "world_creation", BlockingType::MemoryAllocation, || {
        ParallelWorld::new(world_config)
    });
    
    // Initialize camera
    let mut camera_data = init_camera([0.0, 100.0, 0.0], config.window_width, config.window_height);
    
    // Create renderer
    let renderer = time_cpu_operation(&profiler, "renderer_creation", BlockingType::MemoryAllocation, || {
        GpuDrivenRenderer::new(&device, &queue, &surface, &window)
    });
    
    // Frame counter
    let mut frame_count = 0;
    let mut last_report = Instant::now();
    
    // Main loop
    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();
        
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                control_flow.set_exit();
            }
            
            Event::MainEventsCleared => {
                // Begin frame profiling
                reality_begin_frame(&profiler);
                
                // Update world (chunk generation, etc.)
                time_cpu_operation(&profiler, "world_update", BlockingType::ChunkGeneration, || {
                    world.update(camera_data.position.into(), &device, &queue);
                });
                
                // Prepare frame
                let surface_texture = surface.get_current_texture().expect("Failed to get surface texture");
                let surface_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
                
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Reality Check Encoder"),
                });
                
                // GPU timestamp at start of GPU work
                let gpu_start = write_gpu_timestamp(&profiler, &mut encoder);
                
                // Simulate some compute work
                time_cpu_operation(&profiler, "compute_dispatch", BlockingType::CpuWork, || {
                    // Record that we're doing compute work
                    record_compute_dispatch(&profiler);
                    // In real engine, dispatch compute shaders here
                });
                
                // Render pass
                time_cpu_operation(&profiler, "render_pass", BlockingType::CpuWork, || {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Reality Check Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &surface_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.1,
                                    b: 0.1,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });
                    
                    // Simulate draw calls
                    for _ in 0..100 {
                        record_draw_call(&profiler);
                        // In real engine, issue draw calls here
                    }
                    
                    drop(render_pass);
                });
                
                // GPU timestamp at end of GPU work
                if gpu_start.is_some() {
                    write_gpu_timestamp(&profiler, &mut encoder);
                }
                
                // Submit GPU work
                time_cpu_operation(&profiler, "gpu_submit", BlockingType::GpuSync, || {
                    queue.submit(Some(encoder.finish()));
                    surface_texture.present();
                });
                
                // End frame profiling
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    reality_end_frame(&profiler, Some(&device), Some(&queue)).await;
                });
                
                frame_count += 1;
                
                // Generate report every 5 seconds
                if last_report.elapsed() > Duration::from_secs(5) {
                    println!("\n{}", generate_reality_report(&profiler));
                    println!("Frame count: {}", frame_count);
                    last_report = Instant::now();
                }
                
                // Record some fake system metrics for demonstration
                profiler.record_system_metrics("chunk_generation", SystemMetrics {
                    system_name: "chunk_generation".to_string(),
                    cpu_time_ms: 120.0, // Simulating slow chunk gen
                    gpu_time_ms: None,
                    memory_allocated: 1024 * 1024 * 4, // 4MB per chunk
                    is_blocking_main_thread: true,
                });
                
                profiler.record_system_metrics("physics", SystemMetrics {
                    system_name: "physics".to_string(),
                    cpu_time_ms: 45.0,
                    gpu_time_ms: Some(2.0),
                    memory_allocated: 1024 * 512,
                    is_blocking_main_thread: true,
                });
                
                profiler.record_system_metrics("rendering", SystemMetrics {
                    system_name: "rendering".to_string(),
                    cpu_time_ms: 5.0,
                    gpu_time_ms: Some(180.0), // GPU bound!
                    memory_allocated: 0,
                    is_blocking_main_thread: false,
                });
            }
            
            _ => {}
        }
    });
}

fn main() {
    // Use tokio for async GPU timestamp resolution
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run());
}