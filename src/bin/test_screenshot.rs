use anyhow::Result;
use earth_engine::renderer::screenshot;
use wgpu::{Device, Queue, TextureFormat};

async fn test_screenshot_functions(device: &Device, queue: &Queue) -> Result<()> {
    // Test buffer creation
    let buffer = screenshot::create_staging_buffer(&device, 256, 256, 4);
    println!("✓ Created staging buffer: size={}", buffer.size());

    // Test image conversion with mock data
    let width = 2;
    let height = 2;
    let mock_data = vec![
        255, 0, 0, 255,   // Red pixel
        0, 255, 0, 255,   // Green pixel
        0, 0, 255, 255,   // Blue pixel
        255, 255, 0, 255, // Yellow pixel
    ];
    
    let image = screenshot::buffer_to_image(&mock_data, width, height, TextureFormat::Rgba8UnormSrgb)?;
    println!("✓ Converted buffer to image: {}x{}", image.width(), image.height());
    
    // Test BGRA format conversion
    let bgra_data = vec![
        0, 0, 255, 255,   // Red pixel (BGR -> RGB)
        0, 255, 0, 255,   // Green pixel
        255, 0, 0, 255,   // Blue pixel (BGR -> RGB)
        0, 255, 255, 255, // Yellow pixel (BGR -> RGB)
    ];
    
    let bgra_image = screenshot::buffer_to_image(&bgra_data, width, height, TextureFormat::Bgra8UnormSrgb)?;
    println!("✓ Converted BGRA buffer to image");
    
    // Save test image
    screenshot::save_screenshot(&image, "test_screenshot.png")?;
    println!("✓ Saved test screenshot to test_screenshot.png");
    
    // Clean up
    std::fs::remove_file("test_screenshot.png").ok();
    
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();
    
    println!("Testing screenshot module...");
    
    pollster::block_on(async {
        // Create minimal GPU context for testing
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find adapter");
            
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("Failed to create device");
            
        test_screenshot_functions(&device, &queue).await?;
        
        println!("\nAll screenshot tests passed!");
        Ok(())
    })
}