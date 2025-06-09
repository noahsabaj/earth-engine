use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() {
    println!("Earth Engine - GPU Detection Test");
    println!("=================================");
    
    // Create event loop and window
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = WindowBuilder::new()
        .with_title("Earth Engine - GPU Test")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .expect("Failed to create window");

    println!("✓ Window created successfully!");
    
    // Test GPU detection
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: Default::default(),
        ..Default::default()
    });
    
    println!("\nDetecting GPUs...");
    
    // Enumerate adapters
    let adapters = instance.enumerate_adapters(wgpu::Backends::all());
    let mut adapter_count = 0;
    
    for (i, adapter) in adapters.into_iter().enumerate() {
        adapter_count += 1;
        let info = adapter.get_info();
        println!("\nGPU #{}: {}", i + 1, info.name);
        println!("  Backend: {:?}", info.backend);
        println!("  Device Type: {:?}", info.device_type);
        println!("  Driver: {}", info.driver);
        println!("  Driver Info: {}", info.driver_info);
        
        // Check features
        let features = adapter.features();
        println!("  Supports Compute: {}", features.contains(wgpu::Features::SHADER_F16));
    }
    
    if adapter_count == 0 {
        println!("\n❌ No GPU adapters found!");
    } else {
        println!("\n✓ Found {} GPU adapter(s)", adapter_count);
    }
    
    println!("\nPress ESC or close window to exit...");
    
    // Simple event loop
    event_loop.run(move |event, window_target| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    println!("Exiting...");
                    window_target.exit();
                }
                WindowEvent::KeyboardInput {
                    event: winit::event::KeyEvent {
                        state: winit::event::ElementState::Pressed,
                        physical_key: winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
                        ..
                    },
                    ..
                } => {
                    println!("ESC pressed, exiting...");
                    window_target.exit();
                }
                _ => {}
            },
            _ => {}
        }
    }).expect("Failed to run event loop");
}