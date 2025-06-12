//! Quick GPU check utility to diagnose initialization issues

use std::sync::Arc;
use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    dpi::LogicalSize,
};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format_timestamp_millis()
        .init();
    
    println!("=== GPU Check Utility ===");
    log::info!("Starting GPU check...");
    
    // Check environment
    if let Ok(backend) = std::env::var("WGPU_BACKEND") {
        log::info!("WGPU_BACKEND set to: {}", backend);
    } else {
        log::info!("WGPU_BACKEND not set, will use auto-detection");
    }
    
    // Try to create a window and GPU context
    let result = pollster::block_on(check_gpu());
    
    match result {
        Ok(_) => {
            println!("\n✅ GPU initialization successful!");
            log::info!("GPU check completed successfully");
        }
        Err(e) => {
            println!("\n❌ GPU initialization failed!");
            log::error!("GPU check failed: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn check_gpu() -> anyhow::Result<()> {
    log::info!("Creating event loop...");
    let event_loop = EventLoop::new()?;
    
    log::info!("Creating window...");
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("GPU Check")
            .with_inner_size(LogicalSize::new(800, 600))
            .build(&event_loop)?
    );
    
    log::info!("Creating WGPU instance...");
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    // List available backends
    log::info!("Available backends: {:?}", wgpu::Backends::all());
    
    log::info!("Creating surface...");
    let surface = instance.create_surface(window.clone())?;
    
    log::info!("Enumerating adapters...");
    let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all()).collect();
    
    if adapters.is_empty() {
        log::error!("No GPU adapters found!");
        anyhow::bail!("No GPU adapters found. This might be due to missing drivers or WSL GPU support.");
    }
    
    println!("\nFound {} adapter(s):", adapters.len());
    for (i, adapter) in adapters.iter().enumerate() {
        let info = adapter.get_info();
        println!("  [{}] {}", i, info.name);
        println!("      Backend: {:?}", info.backend);
        println!("      Device Type: {:?}", info.device_type);
        println!("      Driver: {}", info.driver);
        println!("      Driver Info: {}", info.driver_info);
        
        // Check if compatible with surface
        if adapter.is_surface_supported(&surface) {
            println!("      ✓ Surface compatible");
        } else {
            println!("      ✗ NOT surface compatible");
        }
    }
    
    log::info!("Requesting adapter...");
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to find suitable adapter"))?;
    
    let adapter_info = adapter.get_info();
    println!("\nSelected adapter: {}", adapter_info.name);
    
    log::info!("Requesting device...");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: Some("Test Device"),
            },
            None,
        )
        .await?;
    
    println!("Device created successfully!");
    
    // Test creating a simple buffer
    log::info!("Testing buffer creation...");
    let _buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Test Buffer"),
        size: 1024,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });
    
    println!("Buffer created successfully!");
    
    // Test surface configuration
    log::info!("Testing surface configuration...");
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats[0];
    
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: 800,
        height: 600,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    
    surface.configure(&device, &config);
    println!("Surface configured successfully!");
    
    // Test getting a surface texture
    log::info!("Testing surface texture acquisition...");
    match surface.get_current_texture() {
        Ok(_texture) => {
            println!("Surface texture acquired successfully!");
        }
        Err(e) => {
            log::warn!("Failed to get surface texture: {:?}", e);
            println!("Warning: Failed to get surface texture (this might be normal without a render loop)");
        }
    }
    
    // Force a device poll to ensure everything is working
    device.poll(wgpu::Maintain::Wait);
    queue.submit([]);
    
    Ok(())
}