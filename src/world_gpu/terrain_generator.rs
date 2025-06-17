use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use crate::world::ChunkPos;
use super::world_buffer::WorldBuffer;

/// Generic block distribution rule
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BlockDistribution {
    /// Block ID to place
    pub block_id: u32,
    /// Minimum Y coordinate (inclusive)
    pub min_height: i32,
    /// Maximum Y coordinate (inclusive)
    pub max_height: i32,
    /// Base probability (0.0-1.0)
    pub probability: f32,
    /// Noise threshold for placement (0.0-1.0)
    /// Used for clustering - higher values create more sparse placement
    pub noise_threshold: f32,
    /// Reserved for future use (ensures proper GPU alignment)
    /// Note: vec3<f32> in WGSL is padded to 16 bytes, so we need 4 floats
    pub _reserved: [f32; 4],
}

impl Default for BlockDistribution {
    fn default() -> Self {
        Self {
            block_id: 0,
            min_height: i32::MIN,
            max_height: i32::MAX,
            probability: 0.0,
            noise_threshold: 0.5,
            _reserved: [0.0; 4],
        }
    }
}

/// Maximum number of custom block distributions
/// This is a GPU limitation - we need fixed-size arrays in shaders
pub const MAX_BLOCK_DISTRIBUTIONS: usize = 16;

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
    /// Number of active block distributions (0 to MAX_BLOCK_DISTRIBUTIONS)
    pub num_distributions: u32,
    /// Padding for alignment
    pub _padding: [u32; 2],
    /// Custom block distributions
    /// Games can specify up to MAX_BLOCK_DISTRIBUTIONS custom blocks
    pub distributions: [BlockDistribution; MAX_BLOCK_DISTRIBUTIONS],
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            seed: 12345,
            sea_level: 64.0,
            terrain_scale: 0.01,
            mountain_threshold: 0.6,
            cave_threshold: 0.3,
            num_distributions: 0,
            _padding: [0; 2],
            distributions: [BlockDistribution::default(); MAX_BLOCK_DISTRIBUTIONS],
        }
    }
}

impl TerrainParams {
    /// Add a block distribution rule
    /// Returns true if added, false if at capacity
    pub fn add_distribution(&mut self, distribution: BlockDistribution) -> bool {
        if self.num_distributions as usize >= MAX_BLOCK_DISTRIBUTIONS {
            log::warn!("[TerrainParams] Cannot add distribution - at maximum capacity ({} distributions)", MAX_BLOCK_DISTRIBUTIONS);
            return false;
        }
        
        let index = self.num_distributions as usize;
        self.distributions[index] = distribution;
        self.num_distributions += 1;
        
        log::debug!("[TerrainParams] Added distribution for block {} at index {} (total: {})", 
                   distribution.block_id, index, self.num_distributions);
        true
    }
    
    /// Clear all distributions
    pub fn clear_distributions(&mut self) {
        self.num_distributions = 0;
        // Zero out for safety
        self.distributions = [BlockDistribution::default(); MAX_BLOCK_DISTRIBUTIONS];
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
        // Log device capabilities
        log::info!("[TerrainGenerator] Initializing GPU terrain generator");
        log::debug!("[TerrainGenerator] Device features: {:?}", device.features());
        
        // Debug: Print actual sizes
        log::info!("[TerrainGenerator] Size of BlockDistribution: {} bytes", std::mem::size_of::<BlockDistribution>());
        log::info!("[TerrainGenerator] Size of TerrainParams: {} bytes", std::mem::size_of::<TerrainParams>());
        log::info!("[TerrainGenerator] Expected distributions array size: {} bytes", std::mem::size_of::<[BlockDistribution; MAX_BLOCK_DISTRIBUTIONS]>());
        
        let limits = device.limits();
        log::info!("[TerrainGenerator] Device compute limits: workgroup_size=({}, {}, {}), workgroups=({}, {}, {})",
                  limits.max_compute_workgroup_size_x, limits.max_compute_workgroup_size_y, limits.max_compute_workgroup_size_z,
                  limits.max_compute_workgroups_per_dimension, limits.max_compute_workgroups_per_dimension, limits.max_compute_workgroups_per_dimension);
        
        // Validate our workgroup size (8, 4, 4) against device limits
        if limits.max_compute_workgroup_size_x < 8 || limits.max_compute_workgroup_size_y < 4 || limits.max_compute_workgroup_size_z < 4 {
            panic!("[TerrainGenerator] Device doesn't support required workgroup size (8, 4, 4)!");
        }
        
        // Use the proper terrain generation shader with Perlin noise
        let shader_source = include_str!("shaders/terrain_generation.wgsl");
        
        log::info!("[TerrainGenerator] Loading terrain generation shader ({} characters)", shader_source.len());
        
        // Validate shader source
        if shader_source.is_empty() {
            panic!("[TerrainGenerator] Shader source is empty!");
        }
        
        // Log first few lines for debugging
        let shader_lines: Vec<&str> = shader_source.lines().take(5).collect();
        log::debug!("[TerrainGenerator] Shader preview: {:?}", shader_lines);
        
        // Create shader module with validation
        log::info!("[TerrainGenerator] Creating shader module...");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain Generation Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        // Note: create_shader_module doesn't return Result in wgpu 0.17+
        // Errors will be caught when creating the pipeline
        log::info!("[TerrainGenerator] Shader module created");
        
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
                        min_binding_size: None, // Let WGPU infer size from shader
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
        
        // Create compute pipeline for terrain generation with validation
        log::info!("[TerrainGenerator] Creating terrain generation compute pipeline...");
        
        // Validate entry point exists
        let entry_point = "generate_chunk";
        if !shader_source.contains(&format!("fn {}", entry_point)) {
            panic!("[TerrainGenerator] Entry point '{}' not found in shader!", entry_point);
        }
        
        // Push error scope to catch pipeline creation errors
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        
        let generate_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Terrain Generation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point,
        });
        
        // Check for errors
        if let Some(error) = futures::executor::block_on(device.pop_error_scope()) {
            panic!("[TerrainGenerator] Failed to create compute pipeline: {:?}", error);
        }
        
        log::info!("[TerrainGenerator] Terrain generation compute pipeline created successfully!");
        log::info!("[TerrainGenerator] GPU terrain generation is ready");
        
        // Create parameters buffer
        let default_params = TerrainParams::default();
        let params_bytes = bytemuck::cast_slice(&[default_params]);
        log::info!("[TerrainGenerator] Creating parameters buffer: {} bytes (shader will validate actual requirements)", params_bytes.len());
        
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Terrain Parameters Buffer"),
            contents: params_bytes,
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
        let start = Instant::now();
        log::debug!("[GPU_TERRAIN] Updating terrain parameters: seed={}, sea_level={}, scale={}", 
                   params.seed, params.sea_level, params.terrain_scale);
        
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[*params]));
        
        let duration = start.elapsed();
        log::debug!("[GPU_TERRAIN] Parameter update completed in {:.2}μs", duration.as_micros());
    }
    
    /// Generate multiple chunks on the GPU
    pub fn generate_chunks(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &mut WorldBuffer,
        chunk_positions: &[ChunkPos],
    ) {
        let overall_start = Instant::now();
        
        if chunk_positions.is_empty() {
            log::debug!("[GPU_TERRAIN] No chunks to generate, returning early");
            return;
        }
        
        log::info!("[GPU_TERRAIN] Starting GPU terrain generation for {} chunks", chunk_positions.len());
        
        // Log spatial context of chunks being generated
        if chunk_positions.len() <= 5 {
            for pos in chunk_positions {
                log::debug!("[GPU_TERRAIN] Generating chunk at world position ({}, {}, {})", 
                           pos.x * 32, pos.y * 32, pos.z * 32);
            }
        } else {
            let min_x = chunk_positions.iter().map(|p| p.x).min().unwrap_or(0);
            let max_x = chunk_positions.iter().map(|p| p.x).max().unwrap_or(0);
            let min_y = chunk_positions.iter().map(|p| p.y).min().unwrap_or(0);
            let max_y = chunk_positions.iter().map(|p| p.y).max().unwrap_or(0);
            let min_z = chunk_positions.iter().map(|p| p.z).min().unwrap_or(0);
            let max_z = chunk_positions.iter().map(|p| p.z).max().unwrap_or(0);
            log::debug!("[GPU_TERRAIN] Chunk batch bounds: X({} to {}), Y({} to {}), Z({} to {})", 
                       min_x, max_x, min_y, max_y, min_z, max_z);
        }
        
        // Create buffer for chunk positions WITH SLOT INDICES
        // CRITICAL FIX: Include actual slot indices to resolve buffer index mismatch
        let buffer_prep_start = Instant::now();
        let positions_data: Vec<[i32; 4]> = chunk_positions
            .iter()
            .map(|pos| {
                let slot = world_buffer.get_chunk_slot(*pos);
                log::debug!("[GPU_TERRAIN] Chunk {:?} allocated to slot {}", pos, slot);
                [pos.x, pos.y, pos.z, slot as i32]  // Pack slot in 4th component
            })
            .collect();
        
        let buffer_size = positions_data.len() * std::mem::size_of::<[i32; 4]>();
        log::debug!("[GPU_TERRAIN] Creating chunk positions buffer: {} bytes for {} chunks", 
                   buffer_size, chunk_positions.len());
        
        let positions_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Positions Buffer"),
            contents: bytemuck::cast_slice(&positions_data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        let buffer_prep_duration = buffer_prep_start.elapsed();
        log::debug!("[GPU_TERRAIN] Chunk positions buffer created in {:.2}μs", buffer_prep_duration.as_micros());
        
        // Create bind group for terrain generation
        let bind_group_start = Instant::now();
        log::debug!("[GPU_TERRAIN] Creating bind group for terrain generation...");
        
        // Log buffer information for diagnostics
        log::debug!("[GPU_TERRAIN] Bind group buffer info:");
        log::debug!("[GPU_TERRAIN]   - World buffer size: {} bytes (max {} chunks)", 
                   world_buffer.buffer_size(), world_buffer.max_chunks());
        log::debug!("[GPU_TERRAIN]   - Positions buffer size: {} bytes", buffer_size);
        
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
        
        let bind_group_duration = bind_group_start.elapsed();
        log::debug!("[GPU_TERRAIN] Terrain generation bind group created in {:.2}μs", bind_group_duration.as_micros());
        
        // Dispatch compute shader
        let compute_start = Instant::now();
        log::info!("[GPU_TERRAIN] Starting compute pass for {} chunks", chunk_positions.len());
        
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Terrain Generation Pass"),
            timestamp_writes: None,
        });
        
        log::debug!("[GPU_TERRAIN] Setting compute pipeline...");
        compute_pass.set_pipeline(&self.generate_pipeline);
        
        log::debug!("[GPU_TERRAIN] Setting bind group...");
        compute_pass.set_bind_group(0, &bind_group, &[]);
        
        // Process chunks in parallel on GPU
        // Each workgroup handles one chunk
        let workgroup_count = chunk_positions.len() as u32;
        log::info!("[GPU_TERRAIN] Dispatching {} workgroups for parallel terrain generation", workgroup_count);
        log::debug!("[GPU_TERRAIN] GPU workload: {} chunks × 32³ voxels = {} total voxels to generate", 
                   workgroup_count, workgroup_count * 32 * 32 * 32);
        
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        
        // Note: Compute pass completion time will be logged after encoder submission
        let compute_setup_duration = compute_start.elapsed();
        log::debug!("[GPU_TERRAIN] Compute pass setup completed in {:.2}μs", compute_setup_duration.as_micros());
        
        // Drop compute pass to end it
        drop(compute_pass);
        
        let overall_duration = overall_start.elapsed();
        log::info!("[GPU_TERRAIN] GPU terrain generation dispatch completed in {:.2}ms for {} chunks", 
                  overall_duration.as_secs_f64() * 1000.0, chunk_positions.len());
        
        // Calculate theoretical performance metrics
        let chunks_per_second = chunk_positions.len() as f64 / overall_duration.as_secs_f64();
        let voxels_per_second = chunks_per_second * (32.0 * 32.0 * 32.0);
        log::info!("[GPU_TERRAIN] Performance metrics: {:.1} chunks/sec, {:.0} voxels/sec", 
                  chunks_per_second, voxels_per_second);
    }
    
    /// Generate a single chunk (convenience method)
    pub fn generate_chunk(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &mut WorldBuffer,
        chunk_pos: ChunkPos,
    ) {
        self.generate_chunks(encoder, world_buffer, &[chunk_pos]);
    }
}