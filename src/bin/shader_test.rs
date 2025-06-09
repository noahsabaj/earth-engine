use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() {
    println!("Testing shader compilation directly...");
    
    // Create event loop and window
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = WindowBuilder::new()
        .with_title("Shader Test")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .expect("Failed to create window");
    
    // Create wgpu instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    pollster::block_on(async {
        // Create surface
        let surface = instance.create_surface(&window).expect("Failed to create surface");
        
        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find adapter");
        
        println!("Adapter: {:?}", adapter.get_info());
        
        // Create device and queue
        let (device, _queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .expect("Failed to create device");
        
        // Test shader compilation
        let shader_source = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return camera.view_proj * vec4<f32>(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Test accessing camera uniform in fragment shader
    let dist = length(camera.camera_pos);
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;
        
        println!("Creating shader module...");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        println!("Creating bind group layout...");
        // Test with VERTEX only
        let bind_group_layout_vertex_only = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("vertex_only_bind_group_layout"),
        });
        
        // Test with VERTEX | FRAGMENT
        let bind_group_layout_both = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("both_bind_group_layout"),
        });
        
        println!("Testing pipeline with VERTEX only visibility...");
        let pipeline_layout_vertex_only = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Vertex Only Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout_vertex_only],
            push_constant_ranges: &[],
        });
        
        println!("Creating pipeline with VERTEX only visibility (will panic if fragment shader needs the uniform)...");
        
        println!("\nTesting pipeline with VERTEX | FRAGMENT visibility...");
        let pipeline_layout_both = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Both Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout_both],
            push_constant_ranges: &[],
        });
        
        println!("Creating pipeline with VERTEX | FRAGMENT visibility (should succeed)...");
        let _pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Both Pipeline"),
            layout: Some(&pipeline_layout_both),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        println!("✓ Pipeline with VERTEX | FRAGMENT visibility succeeded!");
    });
    
    println!("\nTest completed. Press ESC or close window to exit...");
    
    event_loop.run(move |event, window_target| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => window_target.exit(),
                WindowEvent::KeyboardInput {
                    event: winit::event::KeyEvent {
                        state: winit::event::ElementState::Pressed,
                        physical_key: winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
                        ..
                    },
                    ..
                } => window_target.exit(),
                _ => {}
            },
            _ => {}
        }
    }).expect("Failed to run event loop");
}