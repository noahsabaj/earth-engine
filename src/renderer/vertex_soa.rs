use wgpu::util::DeviceExt;

/// Struct-of-Arrays vertex buffer for better cache efficiency
pub struct VertexBufferSoA {
    // Separate buffers for each attribute
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    lights: Vec<f32>,
    aos: Vec<f32>,

    // GPU buffers (created on upload)
    position_buffer: Option<wgpu::Buffer>,
    color_buffer: Option<wgpu::Buffer>,
    normal_buffer: Option<wgpu::Buffer>,
    light_buffer: Option<wgpu::Buffer>,
    ao_buffer: Option<wgpu::Buffer>,
}

impl VertexBufferSoA {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            colors: Vec::new(),
            normals: Vec::new(),
            lights: Vec::new(),
            aos: Vec::new(),
            position_buffer: None,
            color_buffer: None,
            normal_buffer: None,
            light_buffer: None,
            ao_buffer: None,
        }
    }

    /// Add a vertex to the buffer
    pub fn push(
        &mut self,
        position: [f32; 3],
        color: [f32; 3],
        normal: [f32; 3],
        light: f32,
        ao: f32,
    ) {
        self.positions.push(position);
        self.colors.push(color);
        self.normals.push(normal);
        self.lights.push(light);
        self.aos.push(ao);
    }

    /// Clear all vertex data
    pub fn clear(&mut self) {
        self.positions.clear();
        self.colors.clear();
        self.normals.clear();
        self.lights.clear();
        self.aos.clear();
    }

    /// Get the number of vertices
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Upload to GPU - creates separate buffers for each attribute
    pub fn upload(&mut self, device: &wgpu::Device) {
        if self.is_empty() {
            return;
        }

        // Create position buffer
        self.position_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Position Buffer"),
                contents: bytemuck::cast_slice(&self.positions),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );

        // Create color buffer
        self.color_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Color Buffer"),
                contents: bytemuck::cast_slice(&self.colors),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );

        // Create normal buffer
        self.normal_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Normal Buffer"),
                contents: bytemuck::cast_slice(&self.normals),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );

        // Create light buffer
        self.light_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Light Buffer"),
                contents: bytemuck::cast_slice(&self.lights),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );

        // Create AO buffer
        self.ao_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex AO Buffer"),
                contents: bytemuck::cast_slice(&self.aos),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
    }

    /// Bind buffers for rendering
    pub fn bind<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if let Some(buffer) = &self.position_buffer {
            render_pass.set_vertex_buffer(0, buffer.slice(..));
        }
        if let Some(buffer) = &self.color_buffer {
            render_pass.set_vertex_buffer(1, buffer.slice(..));
        }
        if let Some(buffer) = &self.normal_buffer {
            render_pass.set_vertex_buffer(2, buffer.slice(..));
        }
        if let Some(buffer) = &self.light_buffer {
            render_pass.set_vertex_buffer(3, buffer.slice(..));
        }
        if let Some(buffer) = &self.ao_buffer {
            render_pass.set_vertex_buffer(4, buffer.slice(..));
        }
    }

    /// Get vertex buffer layouts for SoA
    pub fn desc<'a>() -> Vec<wgpu::VertexBufferLayout<'a>> {
        vec![
            // Position buffer
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                }],
            },
            // Color buffer
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                }],
            },
            // Normal buffer
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                }],
            },
            // Light buffer
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<f32>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                }],
            },
            // AO buffer
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<f32>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                }],
            },
        ]
    }

    /// Convert from Array-of-Structs for migration
    pub fn from_aos(vertices: &[super::vertex::Vertex]) -> Self {
        let mut soa = Self::new();
        for vertex in vertices {
            soa.push(
                vertex.position,
                vertex.color,
                vertex.normal,
                vertex.light,
                vertex.ao,
            );
        }
        soa
    }

    /// Get memory statistics
    pub fn memory_stats(&self) -> VertexBufferStats {
        let positions_size = self.positions.len() * std::mem::size_of::<[f32; 3]>();
        let colors_size = self.colors.len() * std::mem::size_of::<[f32; 3]>();
        let normals_size = self.normals.len() * std::mem::size_of::<[f32; 3]>();
        let lights_size = self.lights.len() * std::mem::size_of::<f32>();
        let aos_size = self.aos.len() * std::mem::size_of::<f32>();

        VertexBufferStats {
            vertex_count: self.len(),
            total_size: positions_size + colors_size + normals_size + lights_size + aos_size,
            positions_size,
            colors_size,
            normals_size,
            lights_size,
            aos_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VertexBufferStats {
    pub vertex_count: usize,
    pub total_size: usize,
    pub positions_size: usize,
    pub colors_size: usize,
    pub normals_size: usize,
    pub lights_size: usize,
    pub aos_size: usize,
}

impl std::fmt::Display for VertexBufferStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VertexBuffer: {} vertices, {} bytes total (pos: {}, col: {}, norm: {}, light: {}, ao: {})",
            self.vertex_count,
            self.total_size,
            self.positions_size,
            self.colors_size,
            self.normals_size,
            self.lights_size,
            self.aos_size
        )
    }
}
