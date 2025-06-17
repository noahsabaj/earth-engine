use std::sync::Arc;
use hearth_engine::renderer::gpu_driven::{GpuDrivenRenderer, RenderObject};
use hearth_engine::camera::data_camera::{CameraData, init_camera};
use cgmath::{Vector3, Point3};
use wgpu::TextureFormat;
use log::{info, debug};

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    
    info!("Testing GPU-driven renderer instance buffer fix...");
    
    // Create dummy device and queue for testing
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    })).expect("Failed to find an appropriate adapter");
    
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Test Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        },
        None,
    )).expect("Failed to create device");
    
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    
    // Create camera bind group layout (dummy for testing)
    let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Camera Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        ],
    });
    
    // Create renderer
    let mut renderer = GpuDrivenRenderer::new(
        device.clone(),
        queue.clone(),
        TextureFormat::Bgra8UnormSrgb,
        &camera_bind_group_layout,
    );
    
    info!("GPU-driven renderer created successfully");
    
    // Create a dummy camera (using the data-oriented camera system)
    let camera = init_camera(800, 600);
    
    // Test multiple frames with different object counts
    for frame in 0..5 {
        info!("=== Frame {} ===", frame);
        
        // Begin frame - this should clear instance buffers
        renderer.begin_frame(&camera);
        debug!("Frame {} begun - instance buffers should be cleared", frame);
        
        // Create test objects for this frame
        let num_objects = (frame + 1) * 10; // 10, 20, 30, 40, 50 objects
        let mut objects = Vec::new();
        
        for i in 0..num_objects {
            let angle = (i as f32 / num_objects as f32) * std::f32::consts::TAU;
            let radius = 5.0 + (i as f32 * 0.1);
            
            objects.push(RenderObject {
                position: Vector3::new(
                    angle.cos() * radius,
                    i as f32 * 0.2,
                    angle.sin() * radius,
                ),
                scale: 1.0,
                color: [
                    (i as f32 / num_objects as f32),
                    0.5,
                    1.0 - (i as f32 / num_objects as f32),
                    1.0,
                ],
                bounding_radius: 1.0,
                mesh_id: 0,
                material_id: 0,
            });
        }
        
        info!("Submitting {} objects for frame {}", objects.len(), frame);
        renderer.submit_objects(&objects);
        
        // Build commands
        renderer.build_commands();
        
        // Get stats
        let stats = renderer.stats();
        info!("Frame {} stats:", frame);
        info!("  - Objects submitted: {}", stats.objects_submitted);
        info!("  - Instances added: {}", stats.instances_added);
        info!("  - Objects rejected: {}", stats.objects_rejected);
        
        // Verify that instance count matches what we expect
        if stats.instances_added != num_objects as u32 {
            panic!(
                "Frame {}: Instance count mismatch! Expected {}, got {}",
                frame, num_objects, stats.instances_added
            );
        }
        
        info!("Frame {} completed successfully - instance count correct!", frame);
    }
    
    info!("✅ All tests passed! Instance buffer clearing is working correctly.");
}