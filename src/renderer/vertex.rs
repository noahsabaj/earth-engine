use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
    pub light: f32, // Combined light level (0.0 - 1.0)
    pub ao: f32,    // Ambient occlusion (0.0 - 1.0)
}

impl Vertex {
    pub fn new(position: [f32; 3], color: [f32; 3], normal: [f32; 3]) -> Self {
        Self {
            position,
            color,
            normal,
            light: 1.0, // Default full brightness
            ao: 1.0,    // Default no occlusion
        }
    }
    
    pub fn with_lighting(position: [f32; 3], color: [f32; 3], normal: [f32; 3], light: f32, ao: f32) -> Self {
        Self {
            position,
            color,
            normal,
            light,
            ao,
        }
    }
    
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Light
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 9]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
                // Ambient Occlusion
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 10]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}