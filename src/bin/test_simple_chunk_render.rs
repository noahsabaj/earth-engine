#![allow(unused_variables, dead_code, unused_imports)]
use earth_engine::{
    ChunkPos, Chunk, BlockId, BlockRegistry,
    world::{AirBlock, StoneBlock},
    renderer::SimpleAsyncRenderer,
    camera::{CameraData, init_camera_with_spawn, build_view_matrix, build_projection_matrix},
};
use cgmath::{Point3, Vector3, Matrix4, SquareMatrix};
use std::sync::Arc;
use wgpu::util::DeviceExt;

// Camera uniform structure matching what the shader expects
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    view_proj: [[f32; 4]; 4],
    position: [f32; 3],
    _padding: f32,
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view: Matrix4::identity().into(),
            projection: Matrix4::identity().into(),
            view_proj: Matrix4::identity().into(),
            position: [0.0, 0.0, 0.0],
            _padding: 0.0,
        }
    }

    fn update_view_proj(&mut self, camera: &CameraData) {
        let view = build_view_matrix(camera);
        let proj = build_projection_matrix(camera);
        self.view = view.into();
        self.projection = proj.into();
        self.view_proj = (proj * view).into();
        self.position = camera.position;
    }
}

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
            },
            None,
        )
        .await
        .expect("Failed to create device");
    
    log::info!("WGPU initialized");
    
    // Create block registry
    let mut registry = BlockRegistry::new();
    registry.register("earth:air", AirBlock);
    registry.register("earth:stone", StoneBlock);
    let registry = Arc::new(registry);
    
    // Create renderer
    let chunk_size = 32;
    let mut renderer = SimpleAsyncRenderer::new(registry.clone(), chunk_size, None);
    
    // Create a test chunk at origin with some blocks
    let mut chunk = Chunk::new(ChunkPos::new(0, 0, 0), chunk_size);
    
    // Fill bottom layer with stone
    for x in 0..chunk_size {
        for z in 0..chunk_size {
            chunk.set_block(x, 0, z, BlockId::STONE);
        }
    }
    
    // Add a few blocks in a pattern
    for i in 0..10 {
        chunk.set_block(i, 1, i, BlockId::STONE);
        chunk.set_block(i, 2, i, BlockId::STONE);
    }
    
    let non_air = chunk.blocks().iter().filter(|&&b| b != BlockId::AIR).count();
    log::info!("Created test chunk with {} non-air blocks", non_air);
    
    // Mark chunk as dirty
    chunk.mark_dirty();
    
    log::info!("Test chunk created and marked dirty");
    
    // Create a simple camera
    let camera = init_camera_with_spawn(800, 600, 16.0, 20.0, 40.0);
    
    // Create camera uniform buffer
    let mut camera_uniform = CameraUniform::new();
    camera_uniform.update_view_proj(&camera);
    
    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Create bind group layout
    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
        label: Some("camera_bind_group"),
    });
    
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
        
        // SimpleAsyncRenderer is currently just a placeholder
        // In a real implementation, it would render chunks here
        log::info!("Would render chunks here - renderer is placeholder");
    }
    
    queue.submit(std::iter::once(encoder.finish()));
    
    log::info!("Test complete!");
}