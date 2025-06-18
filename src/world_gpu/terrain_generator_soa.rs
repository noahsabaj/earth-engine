//! SOA-optimized terrain generator
//! 
//! This module provides a Structure of Arrays version of the terrain generator
//! for maximum GPU performance and memory bandwidth efficiency.

use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use crate::world::ChunkPos;
use crate::gpu::{
    GpuBufferManager, GpuError,
    types::TypedGpuBuffer,
    soa::{
        SoaBufferBuilder, TerrainParamsSOA, BlockDistributionSOA,
        UnifiedGpuBuffer, BufferLayoutPreference, CpuGpuBridge,
    }
};
use crate::gpu::types::terrain::TerrainParams;
use super::world_buffer::WorldBuffer;

/// SOA-optimized GPU terrain generator
pub struct TerrainGeneratorSOA {
    device: Arc<wgpu::Device>,
    
    /// GPU buffer manager
    buffer_manager: Arc<GpuBufferManager>,
    
    /// Compute pipeline for SOA terrain generation
    generate_pipeline: wgpu::ComputePipeline,
    
    /// SOA parameters buffer
    params_buffer: TypedGpuBuffer<TerrainParamsSOA>,
    
    /// Bind group layout for SOA terrain generation
    bind_group_layout: wgpu::BindGroupLayout,
    
    /// Whether to use vectorized shader variant
    use_vectorized: bool,
}

impl TerrainGeneratorSOA {
    /// Create a new SOA terrain generator with its own buffer manager
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let buffer_manager = Arc::new(GpuBufferManager::new(device.clone(), queue));
        Self::new_with_manager(device, buffer_manager, false)
    }
    
    /// Create a new SOA terrain generator
    pub fn new_with_manager(
        device: Arc<wgpu::Device>, 
        buffer_manager: Arc<GpuBufferManager>,
        use_vectorized: bool,
    ) -> Self {
        log::info!("[TerrainGeneratorSOA] Initializing SOA-optimized terrain generator");
        log::info!("[TerrainGeneratorSOA] Vectorized mode: {}", use_vectorized);
        
        // Log SOA sizes for debugging
        log::info!("[TerrainGeneratorSOA] BlockDistributionSOA size: {} bytes", 
                  std::mem::size_of::<BlockDistributionSOA>());
        log::info!("[TerrainGeneratorSOA] TerrainParamsSOA size: {} bytes", 
                  std::mem::size_of::<TerrainParamsSOA>());
        
        // Load SOA shader
        let shader_source_raw = include_str!("../gpu/shaders/soa/terrain_generation_soa.wgsl");
        
        // Preprocess the shader
        let shader_source = match crate::gpu::preprocess_shader_content(
            shader_source_raw, 
            std::path::Path::new("src/gpu/shaders/soa/terrain_generation_soa.wgsl")
        ) {
            Ok(processed) => processed,
            Err(e) => {
                log::error!("[TerrainGeneratorSOA] Failed to preprocess shader: {}", e);
                panic!("[TerrainGeneratorSOA] Shader preprocessing failed: {}", e);
            }
        };
        
        log::info!("[TerrainGeneratorSOA] Loading SOA shader ({} characters)", shader_source.len());
        
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SOA Terrain Generation Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        // Create bind group layout for SOA shader
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SOA Terrain Generator Bind Group Layout"),
            entries: &[
                // World buffer binding
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
                // Metadata buffer binding
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
                // SOA Parameters buffer (using storage buffer for SOA)
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
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SOA Terrain Generation Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create compute pipeline
        let entry_point = if use_vectorized {
            "generate_terrain_vectorized"
        } else {
            "generate_terrain"
        };
        
        log::info!("[TerrainGeneratorSOA] Using entry point: {}", entry_point);
        
        let generate_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("SOA Terrain Generation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point,
        });
        
        // Create SOA parameters buffer
        let default_params = TerrainParams::default();
        let soa_params = TerrainParamsSOA::from_aos(&default_params);
        
        log::info!("[TerrainGeneratorSOA] Creating SOA parameters buffer");
        
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SOA Terrain Parameters"),
            contents: bytemuck::bytes_of(&soa_params),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        let params_buffer = TypedGpuBuffer::new(
            params_buffer,
            std::mem::size_of::<TerrainParamsSOA>() as u64,
        );
        
        log::info!("[TerrainGeneratorSOA] SOA terrain generator ready!");
        
        Self {
            device,
            buffer_manager,
            generate_pipeline,
            params_buffer,
            bind_group_layout,
            use_vectorized,
        }
    }
    
    /// Update terrain parameters (converts from AOS to SOA)
    pub fn update_params(&self, params: &TerrainParams) -> Result<(), GpuError> {
        let queue = &self.buffer_manager.queue();
        
        // Convert AOS to SOA
        let soa_params = CpuGpuBridge::pack_terrain_params(params);
        
        // Update GPU buffer
        queue.write_buffer(&self.params_buffer.buffer, 0, bytemuck::bytes_of(&soa_params));
        
        log::debug!("[TerrainGeneratorSOA] Updated SOA parameters from AOS");
        Ok(())
    }
    
    /// Update terrain parameters directly with SOA data
    pub fn update_params_soa(&self, params: &TerrainParamsSOA) -> Result<(), GpuError> {
        let queue = &self.buffer_manager.queue();
        
        // Update GPU buffer directly with SOA data
        queue.write_buffer(&self.params_buffer.buffer, 0, bytemuck::bytes_of(params));
        
        log::debug!("[TerrainGeneratorSOA] Updated SOA parameters directly");
        Ok(())
    }
    
    /// Generate chunks using SOA layout
    pub fn generate_chunks(
        &self,
        world_buffer: &WorldBuffer,
        chunk_positions: &[ChunkPos],
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), GpuError> {
        if chunk_positions.is_empty() {
            return Ok(());
        }
        
        let start = Instant::now();
        let batch_size = chunk_positions.len();
        
        log::info!("[TerrainGeneratorSOA] Generating {} chunks with SOA layout", batch_size);
        
        // Create metadata buffer for chunk generation
        let metadata_data: Vec<u32> = chunk_positions.iter()
            .map(|pos| {
                // Pack chunk position into metadata
                let x = ((pos.x & 0xFFFF) as u32) << 16;
                let z = (pos.z & 0xFFFF) as u32;
                x | z
            })
            .collect();
        
        let metadata_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SOA Chunk Metadata"),
            contents: bytemuck::cast_slice(&metadata_data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SOA Terrain Generation Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: world_buffer.voxel_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: metadata_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.params_buffer.buffer.as_entire_binding(),
                },
            ],
        });
        
        // Record compute pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SOA Terrain Generation Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.generate_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            // Dispatch workgroups
            let workgroups_x = 4; // 32 / 8
            let workgroups_y = 8; // 32 / 4  
            let workgroups_z = 8; // 32 / 4
            
            for i in 0..batch_size {
                compute_pass.dispatch_workgroups(
                    workgroups_x,
                    workgroups_y,
                    workgroups_z
                );
            }
        }
        
        let elapsed = start.elapsed();
        log::info!(
            "[TerrainGeneratorSOA] Generated {} chunks in {:?} ({} mode)",
            batch_size,
            elapsed,
            if self.use_vectorized { "vectorized" } else { "scalar" }
        );
        
        Ok(())
    }
    
    /// Generate a single chunk (convenience method)
    pub fn generate_chunk(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &mut WorldBuffer,
        chunk_pos: ChunkPos,
    ) {
        self.generate_chunks(world_buffer, &[chunk_pos], encoder)
            .expect("Failed to generate chunk with SOA");
    }
}

/// Builder for creating SOA terrain generator with options
pub struct TerrainGeneratorSOABuilder {
    use_vectorized: bool,
}

impl TerrainGeneratorSOABuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            use_vectorized: false,
        }
    }
    
    /// Enable vectorized shader variant
    pub fn with_vectorization(mut self, enabled: bool) -> Self {
        self.use_vectorized = enabled;
        self
    }
    
    /// Build the SOA terrain generator
    pub fn build(
        self,
        device: Arc<wgpu::Device>,
        buffer_manager: Arc<GpuBufferManager>,
    ) -> TerrainGeneratorSOA {
        TerrainGeneratorSOA::new_with_manager(device, buffer_manager, self.use_vectorized)
    }
}

impl Default for TerrainGeneratorSOABuilder {
    fn default() -> Self {
        Self::new()
    }
}