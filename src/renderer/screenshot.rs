use anyhow::{Result, anyhow};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::Path;
use wgpu::{Buffer, CommandEncoder, Device, Extent3d, ImageCopyBuffer, ImageCopyTexture, Origin3d, Texture, TextureAspect, TextureFormat};

/// Create a staging buffer for reading texture data from GPU
pub fn create_staging_buffer(
    device: &Device,
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
) -> Buffer {
    let buffer_size = (width * height * bytes_per_pixel) as u64;
    
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Screenshot Staging Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}

/// Copy texture content to staging buffer
pub fn copy_texture_to_buffer(
    encoder: &mut CommandEncoder,
    texture: &Texture,
    buffer: &Buffer,
    width: u32,
    height: u32,
    format: TextureFormat,
) {
    let bytes_per_pixel = match format {
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm => 4,
        TextureFormat::Bgra8UnormSrgb | TextureFormat::Bgra8Unorm => 4,
        _ => return Err(anyhow!("Unsupported texture format for screenshot: {:?}", format)),
    };
    
    encoder.copy_texture_to_buffer(
        ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        ImageCopyBuffer {
            buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * bytes_per_pixel),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

/// Convert buffer data to RGBA image
pub fn buffer_to_image(
    data: &[u8],
    width: u32,
    height: u32,
    format: TextureFormat,
) -> Result<RgbaImage> {
    let expected_len = (width * height * 4) as usize;
    if data.len() != expected_len {
        return Err(anyhow!(
            "Buffer size mismatch: expected {} bytes, got {}",
            expected_len,
            data.len()
        ));
    }
    
    let mut img_buffer = ImageBuffer::new(width, height);
    
    match format {
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm => {
            // Direct copy for RGBA formats
            for (i, pixel) in img_buffer.pixels_mut().enumerate() {
                let offset = i * 4;
                *pixel = Rgba([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
            }
        }
        TextureFormat::Bgra8UnormSrgb | TextureFormat::Bgra8Unorm => {
            // Swap B and R for BGRA formats
            for (i, pixel) in img_buffer.pixels_mut().enumerate() {
                let offset = i * 4;
                *pixel = Rgba([
                    data[offset + 2], // R
                    data[offset + 1], // G
                    data[offset],     // B
                    data[offset + 3], // A
                ]);
            }
        }
        _ => return Err(anyhow!("Unsupported texture format: {:?}", format)),
    }
    
    Ok(img_buffer)
}

/// Save image to PNG file
pub fn save_screenshot(
    image: &RgbaImage,
    path: impl AsRef<Path>,
) -> Result<()> {
    image.save(path)?;
    Ok(())
}

/// Complete screenshot capture pipeline
pub async fn capture_screenshot(
    device: &Device,
    queue: &wgpu::Queue,
    texture: &Texture,
    width: u32,
    height: u32,
    format: TextureFormat,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    // Create staging buffer
    let staging_buffer = create_staging_buffer(device, width, height, 4);
    
    // Copy texture to buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Screenshot Encoder"),
    });
    
    copy_texture_to_buffer(&mut encoder, texture, &staging_buffer, width, height, format);
    
    queue.submit(Some(encoder.finish()));
    
    // Map buffer for reading
    let buffer_slice = staging_buffer.slice(..);
    let (tx, rx) = futures::channel::oneshot::channel();
    
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    
    device.poll(wgpu::Maintain::Wait);
    rx.await.unwrap()?;
    
    // Read buffer data
    let data = buffer_slice.get_mapped_range();
    let image = buffer_to_image(&data, width, height, format)?;
    
    // Important: drop the mapped range before unmapping
    drop(data);
    staging_buffer.unmap();
    
    // Save to file
    save_screenshot(&image, output_path)?;
    
    Ok(())
}

/// Screenshot data for deferred processing
pub struct ScreenshotData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

/// Capture screenshot data without immediately saving
pub async fn capture_screenshot_data(
    device: &Device,
    queue: &wgpu::Queue,
    texture: &Texture,
    width: u32,
    height: u32,
    format: TextureFormat,
) -> Result<ScreenshotData> {
    // Create staging buffer
    let staging_buffer = create_staging_buffer(device, width, height, 4);
    
    // Copy texture to buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Screenshot Data Encoder"),
    });
    
    copy_texture_to_buffer(&mut encoder, texture, &staging_buffer, width, height, format);
    
    queue.submit(Some(encoder.finish()));
    
    // Map buffer for reading
    let buffer_slice = staging_buffer.slice(..);
    let (tx, rx) = futures::channel::oneshot::channel();
    
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    
    device.poll(wgpu::Maintain::Wait);
    rx.await.unwrap()?;
    
    // Read buffer data
    let data = buffer_slice.get_mapped_range();
    let screenshot_data = ScreenshotData {
        data: data.to_vec(),
        width,
        height,
        format,
    };
    
    // Important: drop the mapped range before unmapping
    drop(data);
    staging_buffer.unmap();
    
    Ok(screenshot_data)
}

/// Process screenshot data into image
pub fn process_screenshot_data(data: &ScreenshotData) -> Result<RgbaImage> {
    buffer_to_image(&data.data, data.width, data.height, data.format)
}