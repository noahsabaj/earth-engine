use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() {
    println!("Pipeline Debug Test");
    println!("==================");
    
    // Create event loop and window
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = WindowBuilder::new()
        .with_title("Pipeline Debug")
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
        
        // Test the exact shader from voxel.wgsl
        let shader_source = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32, // Alignment padding for vec3
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) light: f32,
    @location(4) ao: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) light: f32,
    @location(4) ao: f32,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.normal = model.normal;
    out.world_pos = model.position;
    out.light = model.light;
    out.ao = model.ao;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Combine block/sky light with simple directional shading
    let light_dir = normalize(vec3<f32>(0.5, -1.0, 0.3));
    let directional = max(dot(in.normal, -light_dir), 0.0) * 0.3;
    
    // Use the per-vertex light level
    let block_light = in.light;
    
    // Apply ambient occlusion
    let ao_factor = in.ao;
    
    // Combine all lighting
    let final_light = (block_light + directional) * ao_factor;
    
    // Apply fog for distance from camera
    let fog_distance = length(in.world_pos - camera.camera_pos);
    let fog_factor = exp(-fog_distance * 0.002);
    
    let fog_color = vec3<f32>(0.7, 0.8, 0.9);
    let final_color = mix(fog_color, in.color * final_light, fog_factor);
    
    return vec4<f32>(final_color, 1.0);
}
"#;
        
        println!("Creating shader module...");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        println!("Shader module created successfully!");
        
        // Create bind group layout with VERTEX | FRAGMENT visibility
        println!("Creating bind group layout...");
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("camera_bind_group_layout"),
        });
        
        println!("Bind group layout created!");
        
        // Create pipeline layout
        println!("Creating pipeline layout...");
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        println!("Pipeline layout created!");
        
        // Define vertex buffer layout
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 11]>() as wgpu::BufferAddress, // pos(3) + color(3) + normal(3) + light(1) + ao(1)
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // position
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3, // color
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3, // normal
                },
                wgpu::VertexAttribute {
                    offset: 36,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32, // light
                },
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32, // ao
                },
            ],
        }];
        
        // Create render pipeline
        println!("Creating render pipeline...");
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Test Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &vertex_buffers,
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
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        
        println!("✓ Render pipeline created successfully!");
        drop(render_pipeline);
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