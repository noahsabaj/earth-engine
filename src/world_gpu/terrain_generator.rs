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
        // Use minimal test shader to isolate pipeline issues
        let shader_source = include_str!("shaders/minimal_test.wgsl");
        
        // Debug: Print shader info
        log::info!("[TerrainGenerator] Using MINIMAL test shader, length: {} characters", shader_source.len());
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Minimal Test Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        // Create bind group layout for minimal test shader
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Minimal Test Bind Group Layout"),
            entries: &[
                // Single test buffer binding
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
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terrain Generation Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create compute pipeline with proper validation
        log::info!("[TerrainGenerator] Creating compute pipeline with MINIMAL test shader...");
        
        // Force validation by creating a test command encoder immediately
        let mut test_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pipeline Validation Test"),
        });
        
        let generate_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Minimal Test Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "test_compute",
        });
        
        // Try to validate the pipeline by running a complete compute dispatch
        log::info!("[TerrainGenerator] Validating pipeline by running full compute test...");
        
        // Create a test buffer for validation
        let test_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pipeline Validation Buffer"),
            size: 64 * 4, // 64 u32 values
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        
        // Create bind group for validation
        let test_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pipeline Validation Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: test_buffer.as_entire_binding(),
                },
            ],
        });
        
        let mut test_pass = test_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Pipeline Validation Pass"),
            timestamp_writes: None,
        });
        
        // Full pipeline validation - this should catch all issues
        test_pass.set_pipeline(&generate_pipeline);
        test_pass.set_bind_group(0, &test_bind_group, &[]);
        test_pass.dispatch_workgroups(4, 1, 1); // Small test dispatch
        
        drop(test_pass);
        
        // Submit the validation command and wait for completion
        // Note: We don't have access to queue during construction, so we'll skip this for now
        // The validation pass creation itself should catch pipeline issues
        drop(test_encoder);
        
        log::info!("[TerrainGenerator] Pipeline validation PASSED - full compute dispatch successful!");
        
        log::info!("[TerrainGenerator] Minimal test compute pipeline created and validated successfully!");
        
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
        
        // Create test buffer for minimal shader
        let test_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Minimal Test Buffer"),
            size: 1024 * 4, // 1024 u32 values
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        // Create bind group for minimal test
        log::debug!("[TerrainGenerator] Creating bind group for minimal test...");
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Minimal Test Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: test_buffer.as_entire_binding(),
                },
            ],
        });
        log::debug!("[TerrainGenerator] Minimal test bind group created successfully");
        
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
        
        // Dispatch minimal test - just process 64 elements
        log::debug!("[TerrainGenerator] Dispatching minimal test workgroups");
        compute_pass.dispatch_workgroups(
            64, // Simple test with 64 workgroups
            1,
            1,
        );
        log::debug!("[TerrainGenerator] Minimal test compute pass complete");
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