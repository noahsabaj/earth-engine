use wgpu::util::DeviceExt;
use cgmath::{Vector3, Matrix4, SquareMatrix};
use crate::RaycastHit;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LineVertex {
    position: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
    progress: f32,
    _padding: [f32; 3], // Padding to ensure 16-byte alignment
}

impl ModelUniform {
    fn new() -> Self {
        Self {
            model: Matrix4::identity().into(),
            progress: 0.0,
            _padding: [0.0; 3],
        }
    }
}

pub struct SelectionRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
}

impl SelectionRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // Create vertices for a unit cube wireframe
        let vertices = [
            // Bottom face corners
            LineVertex { position: [0.0, 0.0, 0.0] },
            LineVertex { position: [1.0, 0.0, 0.0] },
            LineVertex { position: [1.0, 0.0, 1.0] },
            LineVertex { position: [0.0, 0.0, 1.0] },
            // Top face corners
            LineVertex { position: [0.0, 1.0, 0.0] },
            LineVertex { position: [1.0, 1.0, 0.0] },
            LineVertex { position: [1.0, 1.0, 1.0] },
            LineVertex { position: [0.0, 1.0, 1.0] },
        ];
        
        // Indices for line list (each pair forms a line)
        let indices: Vec<u16> = vec![
            // Bottom face
            0, 1, 1, 2, 2, 3, 3, 0,
            // Top face
            4, 5, 5, 6, 6, 7, 7, 4,
            // Vertical edges
            0, 4, 1, 5, 2, 6, 3, 7,
        ];
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Selection Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Selection Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Selection Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/selection.wgsl").into()),
        });
        
        // Create model uniform buffer and bind group
        let model_uniform = ModelUniform::new();
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Selection Model Buffer"),
            contents: bytemuck::cast_slice(&[model_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let model_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("selection_model_bind_group_layout"),
        });
        
        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &model_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
            label: Some("selection_model_bind_group"),
        });
        
        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Selection Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout, &model_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Selection Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
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
            vertex_buffer,
            index_buffer,
            render_pipeline,
            model_buffer,
            model_bind_group,
        }
    }
    
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
        selected_block: Option<&RaycastHit>,
        queue: &wgpu::Queue,
        breaking_progress: f32,
    ) {
        if let Some(hit) = selected_block {
            // Calculate model matrix for the selected block
            let pos = hit.position;
            let model_matrix = Matrix4::from_translation(Vector3::new(
                pos.x as f32 - 0.005,
                pos.y as f32 - 0.005,
                pos.z as f32 - 0.005,
            )) * Matrix4::from_scale(1.01); // Slightly larger to avoid z-fighting
            
            // Update model uniform with breaking progress
            let model_uniform = ModelUniform {
                model: model_matrix.into(),
                progress: breaking_progress,
                _padding: [0.0; 3],
            };
            queue.write_buffer(
                &self.model_buffer,
                0,
                bytemuck::cast_slice(&[model_uniform]),
            );
            
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.model_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..24, 0, 0..1); // 24 indices for 12 lines
        }
    }
}