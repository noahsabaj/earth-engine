use wgpu::util::DeviceExt;
/// GPU Indirect Rendering System
///
/// Manages indirect draw commands generated entirely on GPU.
/// Enables single draw call for entire world rendering.
/// Part of Sprint 28: GPU-Driven Rendering Optimization
use wgpu::{Buffer, Device, RenderPipeline};

pub struct IndirectRenderer {
    render_pipeline: RenderPipeline,
    instance_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,

    max_instances: usize,
}

impl IndirectRenderer {
    pub fn new(device: &Device, max_instances: usize) -> Self {
        // Check if device supports VERTEX_WRITABLE_STORAGE
        let supports_vertex_storage = device
            .features()
            .contains(wgpu::Features::VERTEX_WRITABLE_STORAGE);
        // Create vertex buffer for a cube (shared by all chunks)
        let vertices = create_cube_vertices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices = create_cube_indices();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create instance buffer (will be filled with visible instances)
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Visible Instance Buffer"),
            size: (std::mem::size_of::<u32>() * max_instances) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // Create shader - use fallback if VERTEX_WRITABLE_STORAGE is not supported
        let shader_source = if supports_vertex_storage {
            include_str!("../../shaders/rendering/indirect_chunk.wgsl")
        } else {
            log::warn!(
                "Using fallback indirect chunk shader - VERTEX_WRITABLE_STORAGE not supported"
            );
            include_str!("../../shaders/rendering/indirect_chunk_fallback.wgsl")
        };

        let shader_name = if supports_vertex_storage {
            "indirect_chunk"
        } else {
            "indirect_chunk_fallback"
        };

        let validated_shader =
            match crate::gpu::automation::create_gpu_shader(device, shader_name, shader_source) {
                Ok(shader) => shader,
                Err(e) => {
                    log::error!(
                        "[IndirectRenderer] Failed to create indirect chunk shader: {}",
                        e
                    );
                    panic!("Cannot proceed without indirect chunk shader");
                }
            };

        // Create render pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Indirect Render Pipeline Layout"),
            bind_group_layouts: &[], // Will be added based on your renderer
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Indirect Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &validated_shader.module,
                entry_point: "vs_main",
                buffers: &[
                    // Vertex buffer layout
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        }],
                    },
                    // Instance buffer layout (just indices)
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Uint32,
                        }],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &validated_shader.module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb, // Adjust to your format
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

        Self {
            render_pipeline,
            instance_buffer,
            vertex_buffer,
            index_buffer,
            max_instances,
        }
    }

    /// Generate indirect draw commands from visible instances
    pub fn generate_commands<'a>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        visible_instances: &'a Buffer,
    ) -> &'a Buffer {
        // The draw commands are already generated by the frustum culling shader
        // This method exists for potential post-processing
        visible_instances
    }

    /// Render all visible chunks with a single multi-draw indirect call
    pub fn render_indirect<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        draw_commands: &'a Buffer,
        draw_count: u32,
        visible_instances: &'a Buffer,
    ) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, visible_instances.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        // Multi-draw indirect - single call renders all visible chunks!
        render_pass.multi_draw_indexed_indirect(draw_commands, 0, draw_count);
    }

    /// Get the instance buffer for updating chunk data
    pub fn get_instance_buffer(&self) -> &Buffer {
        &self.instance_buffer
    }
}

/// Create cube vertices (shared by all chunks)
fn create_cube_vertices() -> Vec<[f32; 3]> {
    vec![
        // Front face
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
        // Back face
        [-0.5, -0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, -0.5, -0.5],
        // Top face
        [-0.5, 0.5, -0.5],
        [-0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, -0.5],
        // Bottom face
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
        // Right face
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, 0.5, 0.5],
        [0.5, -0.5, 0.5],
        // Left face
        [-0.5, -0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, 0.5],
        [-0.5, 0.5, -0.5],
    ]
}

/// Create cube indices
fn create_cube_indices() -> Vec<u16> {
    let mut indices = Vec::with_capacity(36);
    for face in 0..6 {
        let base = face * 4;
        // Two triangles per face
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    indices
}

/// Performance metrics for indirect rendering
#[derive(Debug, Default)]
pub struct IndirectRenderMetrics {
    pub draw_calls: u32,
    pub triangles_submitted: u64,
    pub instances_rendered: u32,
    pub gpu_time_ms: f32,
}
