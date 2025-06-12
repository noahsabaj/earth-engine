use earth_engine::{
    ChunkPos, Chunk, BlockId, BlockRegistry,
    renderer::{SimpleAsyncRenderer, Vertex},
    Camera,
};
use cgmath::{Point3, Vector3};
use std::sync::Arc;
use wgpu::util::DeviceExt;

fn main() {
    env_logger::init();
    
    // Create a simple test
    pollster::block_on(test_chunk_rendering());
}

async fn test_chunk_rendering() {
    // Initialize WGPU
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find adapter");
    
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");
    
    log::info!("WGPU initialized");
    
    // Create block registry
    let mut registry = BlockRegistry::new();
    registry.register("earth:air", BlockId::AIR, Default::default());
    registry.register("earth:stone", BlockId(3), Default::default());
    let registry = Arc::new(registry);
    
    // Create renderer
    let chunk_size = 32;
    let mut renderer = SimpleAsyncRenderer::new(Arc::clone(&registry), chunk_size, Some(1));
    
    // Create a test chunk at origin with some blocks
    let mut chunk = Chunk::new(ChunkPos::new(0, 0, 0), chunk_size);
    
    // Fill bottom layer with stone
    for x in 0..chunk_size {
        for z in 0..chunk_size {
            chunk.set_block(x, 0, z, BlockId(3)); // Stone
        }
    }
    
    // Add a few blocks in a pattern
    for i in 0..10 {
        chunk.set_block(i, 1, i, BlockId(3));
        chunk.set_block(i, 2, i, BlockId(3));
    }
    
    let non_air = chunk.blocks().iter().filter(|&&b| b != BlockId::AIR).count();
    log::info!("Created test chunk with {} non-air blocks", non_air);
    
    // Mark chunk as dirty
    chunk.mark_dirty();
    
    // Create a mock world to queue the chunk
    use parking_lot::RwLock;
    let chunk_lock = Arc::new(RwLock::new(chunk));
    
    // Queue for mesh building
    renderer.mesh_builder.queue_chunk(
        ChunkPos::new(0, 0, 0),
        Arc::clone(&chunk_lock),
        0, // High priority
        [None, None, None, None, None, None], // No neighbors
    );
    
    log::info!("Chunk queued for mesh building");
    
    // Process the mesh building
    renderer.mesh_builder.process_queue(1);
    std::thread::sleep(std::time::Duration::from_millis(100)); // Give time for processing
    
    // Update renderer to upload meshes
    renderer.update(&device);
    
    log::info!("Renderer updated, mesh count: {}", renderer.mesh_count());
    
    // Create a simple camera
    let camera = Camera::new(
        Point3::new(16.0, 20.0, 40.0),
        -90.0_f32.to_radians(),
        -20.0_f32.to_radians(),
        800,
        600,
    );
    
    // Test rendering
    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: Some("Test Render Target"),
        view_formats: &[],
    };
    
    let texture = device.create_texture(&texture_desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Test Encoder"),
    });
    
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Test Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        
        let chunks_rendered = renderer.render(&mut render_pass, &camera);
        log::info!("Chunks rendered: {}", chunks_rendered);
    }
    
    queue.submit(std::iter::once(encoder.finish()));
    
    log::info!("Test complete!");
}