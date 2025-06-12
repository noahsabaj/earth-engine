use wgpu::util::DeviceExt;
use crate::world::gpu_chunk::GpuChunkManager;
use std::sync::Arc;

/// Manages compute pipelines for GPU-based chunk operations
pub struct ComputePipelineManager {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    
    // Pipelines
    mesh_count_pipeline: wgpu::ComputePipeline,
    mesh_generate_pipeline: wgpu::ComputePipeline,
    
    // Bind group layouts
    chunk_bind_group_layout: wgpu::BindGroupLayout,
    output_bind_group_layout: wgpu::BindGroupLayout,
}

/// Output buffers for mesh generation
pub struct MeshGenerationOutput {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub info_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    
    // Staging buffers for readback
    pub vertex_staging: Option<wgpu::Buffer>,
    pub index_staging: Option<wgpu::Buffer>,
    pub info_staging: Option<wgpu::Buffer>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MeshInfo {
    vertex_count: u32,
    index_count: u32,
    _padding: [u32; 2],
}

impl ComputePipelineManager {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Load compute shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Chunk Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/chunk_compute.wgsl").into()
            ),
        });
        
        // Create bind group layout for chunk data
        let chunk_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Chunk Compute Bind Group Layout"),
            entries: &[
                // Metadata
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Block data
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
                // Light data
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create bind group layout for output buffers
        let output_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mesh Output Bind Group Layout"),
            entries: &[
                // Mesh info (vertex/index counts)
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
                // Vertex buffer
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
                // Index buffer
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
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Chunk Compute Pipeline Layout"),
            bind_group_layouts: &[&chunk_bind_group_layout, &output_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create count pipeline (first pass)
        let mesh_count_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Mesh Count Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "count_faces",
        });
        
        // Create generate pipeline (second pass)
        let mesh_generate_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Mesh Generate Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "generate_mesh",
        });
        
        Self {
            device,
            queue,
            mesh_count_pipeline,
            mesh_generate_pipeline,
            chunk_bind_group_layout,
            output_bind_group_layout,
        }
    }
    
    /// Create output buffers for mesh generation
    pub fn create_mesh_output(&self, max_vertices: u32, max_indices: u32) -> MeshGenerationOutput {
        // Create info buffer
        let info = MeshInfo {
            vertex_count: 0,
            index_count: 0,
            _padding: [0; 2],
        };
        let info_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Info Buffer"),
            contents: bytemuck::bytes_of(&info),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        
        // Create vertex buffer (11 floats per vertex: position, color, normal, light, ao)
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Vertex Buffer"),
            size: (max_vertices * 11 * 4) as u64, // 11 floats * 4 bytes
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create index buffer
        let index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Index Buffer"),
            size: (max_indices * 4) as u64, // u32 indices
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mesh Output Bind Group"),
            layout: &self.output_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: info_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: index_buffer.as_entire_binding(),
                },
            ],
        });
        
        MeshGenerationOutput {
            vertex_buffer,
            index_buffer,
            info_buffer,
            bind_group,
            vertex_staging: None,
            index_staging: None,
            info_staging: None,
        }
    }
    
    /// Generate mesh for a GPU chunk
    pub fn generate_chunk_mesh(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        chunk_bind_group: &wgpu::BindGroup,
        output: &MeshGenerationOutput,
        chunk_size: u32,
    ) {
        // First pass: count faces
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Count Faces Pass"),
                timestamp_writes: None,
            });
            
            pass.set_pipeline(&self.mesh_count_pipeline);
            pass.set_bind_group(0, chunk_bind_group, &[]);
            pass.set_bind_group(1, &output.bind_group, &[]);
            
            let workgroups = (chunk_size + 7) / 8; // 8x8x8 workgroup size
            pass.dispatch_workgroups(workgroups, workgroups, workgroups);
        }
        
        // Memory barrier
        encoder.insert_debug_marker("Memory barrier between passes");
        
        // Second pass: generate mesh
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Generate Mesh Pass"),
                timestamp_writes: None,
            });
            
            pass.set_pipeline(&self.mesh_generate_pipeline);
            pass.set_bind_group(0, chunk_bind_group, &[]);
            pass.set_bind_group(1, &output.bind_group, &[]);
            
            let workgroups = (chunk_size + 7) / 8;
            pass.dispatch_workgroups(workgroups, workgroups, workgroups);
        }
    }
    
    /// Create staging buffers for CPU readback
    pub fn create_staging_buffers(&self, output: &mut MeshGenerationOutput, max_vertices: u32, max_indices: u32) {
        output.info_staging = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Info Staging Buffer"),
            size: std::mem::size_of::<MeshInfo>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        
        output.vertex_staging = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Vertex Staging Buffer"),
            size: (max_vertices * 11 * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        
        output.index_staging = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Index Staging Buffer"),
            size: (max_indices * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
    
    /// Copy mesh data to staging buffers for readback
    pub fn copy_to_staging(&self, encoder: &mut wgpu::CommandEncoder, output: &MeshGenerationOutput) {
        if let Some(info_staging) = &output.info_staging {
            encoder.copy_buffer_to_buffer(
                &output.info_buffer,
                0,
                info_staging,
                0,
                std::mem::size_of::<MeshInfo>() as u64,
            );
        }
        
        // Note: We'd need to know the actual sizes to copy vertex/index data efficiently
        // For now, we'll copy the entire buffers
        if let Some(vertex_staging) = &output.vertex_staging {
            let size = vertex_staging.size();
            encoder.copy_buffer_to_buffer(
                &output.vertex_buffer,
                0,
                vertex_staging,
                0,
                size,
            );
        }
        
        if let Some(index_staging) = &output.index_staging {
            let size = index_staging.size();
            encoder.copy_buffer_to_buffer(
                &output.index_buffer,
                0,
                index_staging,
                0,
                size,
            );
        }
    }
}

/// Integration with the renderer
pub struct GpuMeshGenerator {
    compute_manager: ComputePipelineManager,
    chunk_manager: Arc<GpuChunkManager>,
}

impl GpuMeshGenerator {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        chunk_manager: Arc<GpuChunkManager>,
    ) -> Self {
        let compute_manager = ComputePipelineManager::new(device, queue);
        
        Self {
            compute_manager,
            chunk_manager,
        }
    }
    
    /// Generate mesh for a chunk position
    pub fn generate_mesh(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        chunk_pos: &crate::world::ChunkPos,
    ) -> Option<MeshGenerationOutput> {
        // Get GPU chunk
        let gpu_chunk = self.chunk_manager.get_chunk(chunk_pos)?;
        let chunk_bind_group = gpu_chunk.bind_group()?;
        
        // Create output buffers
        // Conservative estimate: max 6 faces per block * 4 vertices per face
        let blocks_per_chunk = 32 * 32 * 32;
        let max_vertices = blocks_per_chunk * 6 * 4;
        let max_indices = blocks_per_chunk * 6 * 6;
        
        let output = self.compute_manager.create_mesh_output(max_vertices, max_indices);
        
        // Generate mesh
        self.compute_manager.generate_chunk_mesh(
            encoder,
            chunk_bind_group,
            &output,
            32, // chunk size
        );
        
        Some(output)
    }
}