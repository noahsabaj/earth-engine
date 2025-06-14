use std::sync::Arc;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use crate::world::ChunkPos;
use super::world_buffer::WorldBuffer;

/// Parameters for terrain generation
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TerrainParams {
    /// World seed for deterministic generation
    pub seed: u32,
    /// Sea level height
    pub sea_level: f32,
    /// Base terrain scale
    pub terrain_scale: f32,
    /// Mountain threshold
    pub mountain_threshold: f32,
    /// Cave density threshold
    pub cave_threshold: f32,
    /// Ore generation chances (0.0-1.0)
    pub ore_chances: [f32; 4],
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            seed: 12345,
            sea_level: 64.0,
            terrain_scale: 0.01,
            mountain_threshold: 0.6,
            cave_threshold: 0.3,
            ore_chances: [0.1, 0.05, 0.02, 0.01], // Coal, Iron, Gold, Diamond
        }
    }
}

/// GPU-based terrain generator
pub struct TerrainGenerator {
    device: Arc<wgpu::Device>,
    
    /// Compute pipeline for terrain generation
    generate_pipeline: wgpu::ComputePipeline,
    
    /// Parameters buffer
    params_buffer: wgpu::Buffer,
    
    /// Bind group layout for terrain generation
    bind_group_layout: wgpu::BindGroupLayout,
}

impl TerrainGenerator {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        // TESTING: Use minimal shader first to verify pipeline works
        let shader_source = include_str!("shaders/minimal_terrain.wgsl");
        
        log::info!("[TerrainGenerator] TESTING with minimal terrain shader: {} characters", shader_source.len());
        
        // Create shader module with error handling
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Minimal Terrain Generation Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        log::info!("[TerrainGenerator] Minimal shader module created successfully");
        
        // Create bind group layout for full terrain generation shader
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Terrain Generator Bind Group Layout"),
            entries: &[
                // World buffer binding (from WorldBuffer)
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
                // Metadata buffer binding (from WorldBuffer)
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
                // Parameters buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(std::num::NonZeroU64::new(std::mem::size_of::<TerrainParams>() as u64).unwrap()),
                    },
                    count: None,
                },
                // Chunk position buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terrain Generation Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create compute pipeline for terrain generation
        log::info!("[TerrainGenerator] Creating terrain generation compute pipeline on NVIDIA GPU...");
        let generate_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Terrain Generation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "generate_chunk",
        });
        log::info!("[TerrainGenerator] Terrain generation compute pipeline created successfully!");
        
        // Create parameters buffer
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Terrain Parameters Buffer"),
            contents: bytemuck::cast_slice(&[TerrainParams::default()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        Self {
            device,
            generate_pipeline,
            params_buffer,
            bind_group_layout,
        }
    }
    
    /// Update terrain generation parameters
    pub fn update_params(&self, queue: &wgpu::Queue, params: &TerrainParams) {
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[*params]));
    }
    
    /// Generate multiple chunks on the GPU
    pub fn generate_chunks(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &WorldBuffer,
        chunk_positions: &[ChunkPos],
    ) {
        if chunk_positions.is_empty() {
            return;
        }
        
        // Create buffer for chunk positions
        let positions_data: Vec<[i32; 4]> = chunk_positions
            .iter()
            .map(|pos| [pos.x, pos.y, pos.z, 0])
            .collect();
        
        let positions_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Positions Buffer"),
            contents: bytemuck::cast_slice(&positions_data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        // Create bind group for terrain generation
        log::debug!("[TerrainGenerator] Creating bind group for terrain generation...");
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain Generation Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: world_buffer.voxel_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: world_buffer.metadata_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: positions_buffer.as_entire_binding(),
                },
            ],
        });
        log::debug!("[TerrainGenerator] Terrain generation bind group created successfully");
        
        // Dispatch compute shader
        log::debug!("[TerrainGenerator] Starting compute pass for {} chunks", chunk_positions.len());
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Terrain Generation Pass"),
            timestamp_writes: None,
        });
        
        log::debug!("[TerrainGenerator] Setting compute pipeline...");
        compute_pass.set_pipeline(&self.generate_pipeline);
        log::debug!("[TerrainGenerator] Setting bind group...");
        compute_pass.set_bind_group(0, &bind_group, &[]);
        
        // Process chunks in parallel on GPU
        // Each workgroup handles one chunk
        log::debug!("[TerrainGenerator] Dispatching {} workgroups for terrain generation", chunk_positions.len());
        compute_pass.dispatch_workgroups(
            chunk_positions.len() as u32,
            1,
            1,
        );
        log::debug!("[TerrainGenerator] Terrain generation compute pass complete");
    }
    
    /// Generate a single chunk (convenience method)
    pub fn generate_chunk(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &WorldBuffer,
        chunk_pos: ChunkPos,
    ) {
        self.generate_chunks(encoder, world_buffer, &[chunk_pos]);
    }
}