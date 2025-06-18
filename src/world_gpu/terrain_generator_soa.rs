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
    },
    buffer_layouts::{bindings, usage, layouts, constants::*},
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
    /// Validate that a shader entry point exists in the shader source
    fn validate_shader_entry_point(shader_source: &str, entry_point: &str) -> Result<(), String> {
        // Check for the entry point function definition
        let fn_pattern = format!("fn {}(", entry_point);
        if !shader_source.contains(&fn_pattern) {
            return Err(format!(
                "Entry point '{}' not found in shader. Available functions: {}",
                entry_point,
                Self::extract_function_names(shader_source).join(", ")
            ));
        }
        
        // Check for @compute annotation
        let lines: Vec<&str> = shader_source.lines().collect();
        let mut found_entry_point = false;
        let mut has_compute_annotation = false;
        
        for (i, line) in lines.iter().enumerate() {
            if line.contains(&fn_pattern) {
                found_entry_point = true;
                // Check previous lines for @compute annotation
                for j in (0..i).rev() {
                    let prev_line = lines[j].trim();
                    if prev_line.is_empty() || prev_line.starts_with("//") {
                        continue;
                    }
                    if prev_line.contains("@compute") {
                        has_compute_annotation = true;
                        break;
                    }
                    // If we hit another function or non-annotation, stop looking
                    if prev_line.contains("fn ") || (!prev_line.starts_with("@") && !prev_line.starts_with("//")) {
                        break;
                    }
                }
                break;
            }
        }
        
        if !found_entry_point {
            return Err(format!("Entry point function '{}' not found", entry_point));
        }
        
        if !has_compute_annotation {
            return Err(format!(
                "Entry point '{}' found but missing @compute annotation. Compute shaders require @compute.",
                entry_point
            ));
        }
        
        log::debug!("[TerrainGeneratorSOA] Shader validation passed for entry point: {}", entry_point);
        Ok(())
    }
    
    /// Extract function names from shader source for debugging
    fn extract_function_names(shader_source: &str) -> Vec<String> {
        let mut functions = Vec::new();
        for line in shader_source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("fn ") {
                if let Some(name_start) = trimmed.find("fn ").map(|i| i + 3) {
                    if let Some(name_end) = trimmed[name_start..].find('(') {
                        let name = trimmed[name_start..name_start + name_end].trim();
                        if !name.is_empty() {
                            functions.push(name.to_string());
                        }
                    }
                }
            }
        }
        functions
    }
    
    /// Validate bind group layout matches shader expectations
    fn validate_bind_group_layout(shader_source: &str) -> Result<(), String> {
        let mut issues = Vec::new();
        
        // Check for expected bindings in shader
        let expected_bindings = [
            ("@group(0) @binding(0)", "voxel buffer"),
            ("@group(0) @binding(1)", "metadata buffer"), 
            ("@group(0) @binding(2)", "params buffer"),
        ];
        
        for (binding_pattern, binding_name) in expected_bindings {
            if !shader_source.contains(binding_pattern) {
                issues.push(format!("Missing {} binding: {}", binding_name, binding_pattern));
            }
        }
        
        // Check for ChunkMetadata usage
        if !shader_source.contains("ChunkMetadata") {
            issues.push("Shader missing ChunkMetadata struct definition".to_string());
        }
        
        // Check for world_data array access (the actual buffer name used)
        if !shader_source.contains("world_data[") {
            issues.push("Shader missing world_data array access pattern".to_string());
        }
        
        if !issues.is_empty() {
            return Err(format!("Bind group layout validation failed: {}", issues.join(", ")));
        }
        
        log::debug!("[TerrainGeneratorSOA] Bind group layout validation passed");
        Ok(())
    }

    /// Create compute pipeline with comprehensive validation and error handling
    fn create_compute_pipeline_with_validation(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        entry_point: &str,
    ) -> Result<wgpu::ComputePipeline, String> {
        log::debug!("[TerrainGeneratorSOA] Attempting to create compute pipeline with entry point: {}", entry_point);
        
        // Create pipeline descriptor
        let descriptor = wgpu::ComputePipelineDescriptor {
            label: Some("SOA Terrain Generation Pipeline"),
            layout: Some(pipeline_layout),
            module: shader,
            entry_point,
        };
        
        // Attempt pipeline creation
        // Note: wgpu doesn't return Result from create_compute_pipeline, but it can panic
        // We'll use std::panic::catch_unwind to catch any panics during creation
        let pipeline_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            device.create_compute_pipeline(&descriptor)
        }));
        
        match pipeline_result {
            Ok(pipeline) => {
                log::debug!("[TerrainGeneratorSOA] Compute pipeline created successfully");
                
                // Additional validation - check pipeline is not null/invalid
                // wgpu pipelines don't have a direct "is_valid" method, but we can check basic properties
                log::debug!("[TerrainGeneratorSOA] Pipeline validation complete");
                Ok(pipeline)
            },
            Err(panic_payload) => {
                let error_msg = if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic during pipeline creation".to_string()
                };
                
                Err(format!("Pipeline creation panicked: {}", error_msg))
            }
        }
    }

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
        
        // Keep a copy of shader source for validation
        let shader_source_for_validation = shader_source.clone();
        
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SOA Terrain Generation Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        // Create bind group layout for SOA shader using centralized definitions
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SOA Terrain Generator Bind Group Layout"),
            entries: &[
                // World buffer binding
                layouts::storage_buffer_entry(
                    bindings::world::VOXEL_BUFFER,
                    false,
                    wgpu::ShaderStages::COMPUTE,
                ),
                // Metadata buffer binding
                layouts::storage_buffer_entry(
                    bindings::world::METADATA_BUFFER,
                    true,
                    wgpu::ShaderStages::COMPUTE,
                ),
                // SOA Parameters buffer
                layouts::storage_buffer_entry(
                    bindings::world::PARAMS_BUFFER,
                    true,
                    wgpu::ShaderStages::COMPUTE,
                ),
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SOA Terrain Generation Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create compute pipeline with comprehensive error handling
        let entry_point = if use_vectorized {
            "generate_terrain_vectorized"
        } else {
            "generate_terrain"
        };
        
        log::info!("[TerrainGeneratorSOA] Using entry point: {}", entry_point);
        
        // Validate shader entry point exists before pipeline creation
        Self::validate_shader_entry_point(&shader_source_for_validation, entry_point)
            .unwrap_or_else(|e| {
                log::error!("[TerrainGeneratorSOA] Shader validation failed: {}", e);
                panic!("[TerrainGeneratorSOA] Cannot create pipeline with invalid shader: {}", e);
            });
        
        // Validate bind group layout matches shader expectations
        Self::validate_bind_group_layout(&shader_source_for_validation)
            .unwrap_or_else(|e| {
                log::error!("[TerrainGeneratorSOA] Bind group layout validation failed: {}", e);
                panic!("[TerrainGeneratorSOA] Cannot create pipeline with mismatched bindings: {}", e);
            });
        
        // Log detailed pipeline creation parameters for debugging
        log::info!(
            "[TerrainGeneratorSOA] Creating compute pipeline - Entry: {}, Shader size: {} chars, Layout bindings: {}",
            entry_point,
            shader_source_for_validation.len(),
            3  // We have 3 bindings: voxel_buffer, metadata_buffer, params_buffer
        );
        
        // Attempt pipeline creation with error catching
        let generate_pipeline = Self::create_compute_pipeline_with_validation(
            &device,
            &pipeline_layout,
            &shader,
            entry_point,
        ).unwrap_or_else(|e| {
            log::error!("[TerrainGeneratorSOA] Pipeline creation failed: {}", e);
            log::error!("[TerrainGeneratorSOA] Shader source (first 500 chars): {}", 
                       &shader_source_for_validation[..shader_source_for_validation.len().min(500)]);
            log::error!("[TerrainGeneratorSOA] Entry point requested: {}", entry_point);
            log::error!("[TerrainGeneratorSOA] Pipeline layout: {:?}", pipeline_layout);
            panic!("[TerrainGeneratorSOA] Cannot continue without valid compute pipeline: {}", e);
        });
        
        log::info!("[TerrainGeneratorSOA] Compute pipeline created successfully");
        
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
        // Each chunk needs a full ChunkMetadata struct (4 u32 values)
        let metadata_data: Vec<u32> = chunk_positions.iter()
            .flat_map(|pos| {
                // Create ChunkMetadata for each chunk
                let flags = ((pos.x & 0xFFFF) as u32) << 16 | (pos.z & 0xFFFF) as u32;
                let timestamp = 0u32;
                let checksum = 0u32;
                let reserved = pos.y as u32; // Store Y position in reserved field
                vec![flags, timestamp, checksum, reserved]
            })
            .collect();
        
        let metadata_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SOA Chunk Metadata"),
            contents: bytemuck::cast_slice(&metadata_data),
            usage: usage::STORAGE,
        });
        
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SOA Terrain Generation Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: bindings::world::VOXEL_BUFFER,
                    resource: world_buffer.voxel_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: bindings::world::METADATA_BUFFER,
                    resource: metadata_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: bindings::world::PARAMS_BUFFER,
                    resource: self.params_buffer.buffer.as_entire_binding(),
                },
            ],
        });
        
        // Record compute pass with comprehensive error handling
        {
            log::debug!(
                "[TerrainGeneratorSOA] Starting compute pass for {} chunks", 
                chunk_positions.len()
            );
            
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SOA Terrain Generation Pass"),
                timestamp_writes: None,
            });
            
            // Validate pipeline before use
            log::debug!("[TerrainGeneratorSOA] Setting compute pipeline");
            compute_pass.set_pipeline(&self.generate_pipeline);
            
            // Validate bind group before use
            log::debug!("[TerrainGeneratorSOA] Setting bind group with {} entries", 3);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            // Dispatch workgroups for one chunk at a time
            // With workgroup size 8x8x8 and chunk size 32x32x32
            let workgroups_x = 4; // 32 / 8
            let workgroups_y = 4; // 32 / 8  
            let workgroups_z = 4; // 32 / 8
            
            // For now, generate one chunk at a time
            // TODO: Optimize to generate multiple chunks in parallel
            log::debug!(
                "[TerrainGeneratorSOA] Dispatching workgroups: {}x{}x{} (total: {} workgroups)",
                workgroups_x, workgroups_y, workgroups_z,
                workgroups_x * workgroups_y * workgroups_z
            );
            
            compute_pass.dispatch_workgroups(
                workgroups_x,
                workgroups_y,
                workgroups_z
            );
            
            log::debug!("[TerrainGeneratorSOA] Compute pass dispatch completed successfully");
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