use earth_engine::{
    BlockRegistry, BlockId,
    renderer::{
        gpu_driven::gpu_driven_renderer::{GpuDrivenRenderer, RenderObject},
    },
    camera::data_camera::{CameraData, init_camera},
};
use earth_engine::game::{GameData, GameContext};
use cgmath::Vector3;
use std::sync::Arc;

/// Test game data for GPU instance fix test (DOP - no methods)
#[derive(Default)]
struct TestGameData;

impl GameData for TestGameData {}

/// Register blocks for test GPU instance fix game
/// Function - transforms registry for test game
fn register_test_gpu_instance_fix_blocks(_game: &mut TestGameData, _registry: &mut BlockRegistry) {
    // Use default blocks
}

/// Get active block for test GPU instance fix game
/// Pure function - returns active block for test game
fn get_test_gpu_instance_fix_active_block(_game: &TestGameData) -> BlockId {
    BlockId::STONE
}

/// Update test GPU instance fix game
/// Function - no-op for test
fn update_test_gpu_instance_fix_game(_game: &mut TestGameData, _context: &mut GameContext, _delta_time: f32) {
    // No-op for test
}

fn main() {
    env_logger::init();
    
    log::info!("Testing GPU instance persistence fix...");
    
    // Create test objects
    let test_objects = vec![
        RenderObject {
            position: Vector3::new(0.0, 0.0, 0.0),
            scale: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            bounding_radius: 1.0,
            mesh_id: 0,
            material_id: 0,
        },
        RenderObject {
            position: Vector3::new(10.0, 0.0, 0.0),
            scale: 1.0,
            color: [0.0, 1.0, 0.0, 1.0],
            bounding_radius: 1.0,
            mesh_id: 0,
            material_id: 0,
        },
        RenderObject {
            position: Vector3::new(0.0, 10.0, 0.0),
            scale: 1.0,
            color: [0.0, 0.0, 1.0, 1.0],
            bounding_radius: 1.0,
            mesh_id: 0,
            material_id: 0,
        },
    ];
    
    // Simulate renderer workflow
    pollster::block_on(async {
        // Initialize WGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }).await.expect("Failed to request adapter");
        
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ).await.expect("Failed to request device");
        
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        
        // Create camera bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                },
            ],
            label: Some("Camera Bind Group Layout"),
        });
        
        // Create GPU driven renderer
        let mut renderer = GpuDrivenRenderer::new(
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Bgra8UnormSrgb,
            &camera_bind_group_layout,
        );
        
        // Create test camera
        let camera = init_camera(
            800,
            600,
        );
        
        log::info!("=== Frame 1: Submit objects ===");
        
        // Frame 1: Submit objects
        renderer.begin_frame(&camera);
        renderer.submit_objects(&test_objects);
        renderer.upload_instances(&queue);
        
        let stats1 = renderer.stats();
        log::info!("Frame 1 - Submitted: {}, Instances: {}", 
                  stats1.objects_submitted, stats1.instances_added);
        
        // Verify instances are in buffer
        let instance_count1 = renderer.get_instance_count();
        log::info!("Frame 1 - Instance buffer count: {}", instance_count1);
        
        log::info!("\n=== Frame 2: No changes (instances should persist) ===");
        
        // Frame 2: No changes - instances should persist
        renderer.begin_frame(&camera);
        // DO NOT submit objects again - they should persist from frame 1
        
        let stats2 = renderer.stats();
        log::info!("Frame 2 - Submitted: {}, Instances: {}", 
                  stats2.objects_submitted, stats2.instances_added);
        
        // Check if instances persisted
        let instance_count2 = renderer.get_instance_count();
        log::info!("Frame 2 - Instance buffer count: {}", instance_count2);
        
        if instance_count2 == 0 {
            log::error!("FAIL: Instances were cleared! Expected: {}, Got: 0", instance_count1);
        } else if instance_count2 == instance_count1 {
            log::info!("SUCCESS: Instances persisted across frames! Count: {}", instance_count2);
        } else {
            log::warn!("Unexpected: Instance count changed from {} to {}", instance_count1, instance_count2);
        }
        
        log::info!("\n=== Frame 3: Clear and resubmit ===");
        
        // Frame 3: Clear and resubmit
        renderer.begin_frame(&camera);
        renderer.clear_instances(); // Explicitly clear
        renderer.submit_objects(&test_objects);
        renderer.upload_instances(&queue);
        
        let stats3 = renderer.stats();
        log::info!("Frame 3 - Submitted: {}, Instances: {}", 
                  stats3.objects_submitted, stats3.instances_added);
        
        let instance_count3 = renderer.get_instance_count();
        log::info!("Frame 3 - Instance buffer count after rebuild: {}", instance_count3);
        
        log::info!("\n=== Test Summary ===");
        log::info!("Frame 1: {} instances created", instance_count1);
        log::info!("Frame 2: {} instances (should persist)", instance_count2);
        log::info!("Frame 3: {} instances (after explicit clear)", instance_count3);
        
        if instance_count1 == 3 && instance_count2 == 3 && instance_count3 == 3 {
            log::info!("✓ All tests passed!");
        } else {
            log::error!("✗ Test failed - unexpected instance counts");
        }
    });
}