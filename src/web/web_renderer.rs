use wgpu::{RenderPipeline, BindGroup, BindGroupLayout};
use crate::web::{WebGpuContext, WebWorldBuffer, WebError};
use wasm_bindgen::prelude::*;
use bytemuck::{Pod, Zeroable};

/// Vertex data for web rendering
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct WebVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    ao: f32,
    light_level: f32,
}

impl WebVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WebVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 36,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

/// Web-optimized renderer using GPU-first architecture
pub struct WebRenderer {
    /// Render pipeline for voxel rendering
    voxel_pipeline: RenderPipeline,
    
    /// Compute pipeline for mesh generation
    mesh_gen_pipeline: wgpu::ComputePipeline,
    
    /// Bind groups
    world_bind_group: BindGroup,
    mesh_bind_group: BindGroup,
    
    /// Render state
    depth_texture: wgpu::TextureView,
    
    /// Performance tracking
    last_frame_time: f64,
    frame_count: u32,
    
    /// Browser-specific optimizations
    supports_timestamp_queries: bool,
}

impl WebRenderer {
    /// Create a new web renderer
    pub fn new(
        context: &WebGpuContext,
        world_buffer: &WebWorldBuffer,
    ) -> Result<Self, WebError> {
        log::info!("Creating WebRenderer");
        
        // Create depth texture
        let depth_texture = create_depth_texture(context);
        
        // Create bind group layouts
        let world_bind_group_layout = create_world_bind_group_layout(&context.device);
        let mesh_bind_group_layout = create_mesh_bind_group_layout(&context.device);
        
        // Create world bind group
        let world_bind_group = create_world_bind_group(
            &context.device,
            &world_bind_group_layout,
            world_buffer,
        );
        
        // Create mesh buffers and bind group
        let (mesh_bind_group, _vertex_buffer, _index_buffer) = create_mesh_resources(
            &context.device,
            &mesh_bind_group_layout,
        );
        
        // Create mesh generation compute pipeline
        let mesh_gen_pipeline = create_mesh_generation_pipeline(
            &context.device,
            &world_bind_group_layout,
            &mesh_bind_group_layout,
        )?;
        
        // Create render pipeline
        let voxel_pipeline = create_voxel_render_pipeline(
            &context.device,
            context.surface_config.format,
        )?;
        
        // Check for timestamp query support
        let supports_timestamp_queries = context.device.features()
            .contains(wgpu::Features::TIMESTAMP_QUERY);
        
        if supports_timestamp_queries {
            log::info!("Timestamp queries supported - enabling GPU timing");
        }
        
        Ok(Self {
            voxel_pipeline,
            mesh_gen_pipeline,
            world_bind_group,
            mesh_bind_group,
            depth_texture,
            last_frame_time: 0.0,
            frame_count: 0,
            supports_timestamp_queries,
        })
    }
    
    /// Update render state (called when window resizes)
    pub fn resize(&mut self, context: &WebGpuContext) {
        self.depth_texture = create_depth_texture(context);
    }
    
    /// Render a frame
    pub fn render(
        &mut self,
        context: &WebGpuContext,
        world_buffer: &WebWorldBuffer,
    ) -> Result<(), WebError> {
        // Get current texture
        let output = context.get_current_texture()
            .map_err(|e| WebError::JsError(format!("Failed to get texture: {:?}", e)))?;
        
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Web Render Encoder"),
            }
        );
        
        // Generate meshes with compute shader
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Mesh Generation Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.mesh_gen_pipeline);
            compute_pass.set_bind_group(0, &self.world_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.mesh_bind_group, &[]);
            
            // Dispatch based on world size
            let chunks_x = world_buffer.world_size / 32;
            let chunks_z = world_buffer.world_size / 32;
            let chunks_y = world_buffer.world_height / 32;
            
            compute_pass.dispatch_workgroups(chunks_x, chunks_y, chunks_z);
        }
        
        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Web Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.8,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            
            render_pass.set_pipeline(&self.voxel_pipeline);
            
            // Draw calls will be added here once mesh generation is complete
        }
        
        context.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        // Update performance tracking
        self.frame_count += 1;
        if self.frame_count % 60 == 0 {
            let current_time = web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0);
            
            if self.last_frame_time > 0.0 {
                let delta = current_time - self.last_frame_time;
                let fps = 60000.0 / delta;
                log::info!("Web FPS: {:.1}", fps);
            }
            
            self.last_frame_time = current_time;
        }
        
        Ok(())
    }
    
    /// Get performance stats
    pub fn get_stats(&self) -> WebRenderStats {
        WebRenderStats {
            frame_count: self.frame_count,
            supports_timestamp_queries: self.supports_timestamp_queries,
        }
    }
}

/// Performance statistics
#[wasm_bindgen]
pub struct WebRenderStats {
    pub frame_count: u32,
    pub supports_timestamp_queries: bool,
}

/// Create depth texture for rendering
fn create_depth_texture(context: &WebGpuContext) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: context.surface_config.width,
        height: context.surface_config.height,
        depth_or_array_layers: 1,
    };
    
    let desc = wgpu::TextureDescriptor {
        label: Some("Web Depth Texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    
    let texture = context.device.create_texture(&desc);
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Create world bind group layout
fn create_world_bind_group_layout(device: &wgpu::Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("World Bind Group Layout"),
        entries: &[
            // Voxel buffer
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Metadata buffer
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/// Create mesh bind group layout
fn create_mesh_bind_group_layout(device: &wgpu::Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Mesh Bind Group Layout"),
        entries: &[
            // Vertex buffer
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Index buffer
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Indirect buffer
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/// Create world bind group
fn create_world_bind_group(
    device: &wgpu::Device,
    layout: &BindGroupLayout,
    world_buffer: &WebWorldBuffer,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("World Bind Group"),
        layout,
        entries: &world_buffer.create_bind_group_entries(),
    })
}

/// Create mesh resources
fn create_mesh_resources(
    device: &wgpu::Device,
    layout: &BindGroupLayout,
) -> (BindGroup, wgpu::Buffer, wgpu::Buffer) {
    // Create vertex buffer (preallocate for max vertices)
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Web Vertex Buffer"),
        size: 100 * 1024 * 1024, // 100MB
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });
    
    // Create index buffer
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Web Index Buffer"),
        size: 50 * 1024 * 1024, // 50MB
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDEX,
        mapped_at_creation: false,
    });
    
    // Create indirect buffer for GPU-driven rendering
    let indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Web Indirect Buffer"),
        size: 1024 * 1024, // 1MB for indirect commands
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
        mapped_at_creation: false,
    });
    
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Mesh Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: vertex_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: index_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: indirect_buffer.as_entire_binding(),
            },
        ],
    });
    
    (bind_group, vertex_buffer, index_buffer)
}

/// Create mesh generation compute pipeline
fn create_mesh_generation_pipeline(
    device: &wgpu::Device,
    world_layout: &BindGroupLayout,
    mesh_layout: &BindGroupLayout,
) -> Result<wgpu::ComputePipeline, WebError> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Web Mesh Generation Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/web_mesh_gen.wgsl").into()),
    });
    
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh Generation Pipeline Layout"),
        bind_group_layouts: &[world_layout, mesh_layout],
        push_constant_ranges: &[],
    });
    
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Web Mesh Generation Pipeline"),
        layout: Some(&layout),
        module: &shader,
        entry_point: "main",
    });
    
    Ok(pipeline)
}

/// Create voxel render pipeline
fn create_voxel_render_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> Result<RenderPipeline, WebError> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Web Voxel Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/web_voxel.wgsl").into()),
    });
    
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Web Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });
    
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Web Voxel Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[WebVertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
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
    
    Ok(pipeline)
}