use std::sync::Arc;
use wgpu::util::DeviceExt;
use crate::world::ChunkPos;
use super::world_buffer::WorldBuffer;

/// GPU-based ambient occlusion and lighting system
pub struct GpuLighting {
    device: Arc<wgpu::Device>,
    
    /// Pipeline for calculating ambient occlusion
    ao_pipeline: wgpu::ComputePipeline,
    
    /// Pipeline for smoothing AO values
    smooth_pipeline: wgpu::ComputePipeline,
    
    /// Bind group layout for lighting operations
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuLighting {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ambient Occlusion Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ambient_occlusion.wgsl").into()),
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GPU Lighting Bind Group Layout"),
            entries: &[
                // World buffer
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
                // Chunk positions
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
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GPU Lighting Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create pipelines
        let ao_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("AO Calculation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "calculate_ao",
        });
        
        let smooth_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("AO Smoothing Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "smooth_ao",
        });
        
        Self {
            device,
            ao_pipeline,
            smooth_pipeline,
            bind_group_layout,
        }
    }
    
    /// Calculate ambient occlusion for chunks
    pub fn calculate_ambient_occlusion(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &WorldBuffer,
        chunk_positions: &[ChunkPos],
        smooth_passes: u32,
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
            label: Some("AO Chunk Positions Buffer"),
            contents: bytemuck::cast_slice(&positions_data),
            usage: wgpu::BufferUsages::STORAGE,
        });
        
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AO Calculation Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: world_buffer.voxel_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: positions_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Calculate initial AO values
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AO Calculation Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.ao_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            // Process chunks in parallel
            compute_pass.dispatch_workgroups(
                chunk_positions.len() as u32,
                1,
                1,
            );
        }
        
        // Apply smoothing passes
        for pass in 0..smooth_passes {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(&format!("AO Smoothing Pass {}", pass)),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&self.smooth_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            
            compute_pass.dispatch_workgroups(
                chunk_positions.len() as u32,
                1,
                1,
            );
        }
    }
    
    /// Calculate AO for newly generated chunks
    pub fn update_chunk_lighting(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &WorldBuffer,
        chunk_pos: ChunkPos,
    ) {
        // When updating a single chunk, we need to also update neighbors
        // to ensure smooth AO transitions at chunk boundaries
        let mut chunks_to_update = vec![chunk_pos];
        
        // Add neighboring chunks
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    chunks_to_update.push(ChunkPos {
                        x: chunk_pos.x + dx,
                        y: chunk_pos.y + dy,
                        z: chunk_pos.z + dz,
                    });
                }
            }
        }
        
        // Calculate AO with smoothing
        self.calculate_ambient_occlusion(
            encoder,
            world_buffer,
            &chunks_to_update,
            2, // 2 smoothing passes for quality
        );
    }
    
    /// Batch update lighting for multiple chunks
    pub fn batch_update_lighting(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &WorldBuffer,
        chunk_positions: &[ChunkPos],
    ) {
        // Collect all chunks including neighbors
        let mut all_chunks = Vec::new();
        let mut chunk_set = std::collections::HashSet::new();
        
        for chunk_pos in chunk_positions {
            // Add the chunk itself
            if chunk_set.insert(*chunk_pos) {
                all_chunks.push(*chunk_pos);
            }
            
            // Add immediate neighbors for smooth transitions
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if dx == 0 && dy == 0 && dz == 0 {
                            continue;
                        }
                        let neighbor = ChunkPos {
                            x: chunk_pos.x + dx,
                            y: chunk_pos.y + dy,
                            z: chunk_pos.z + dz,
                        };
                        if chunk_set.insert(neighbor) {
                            all_chunks.push(neighbor);
                        }
                    }
                }
            }
        }
        
        // Process in batches to avoid GPU memory limits
        const BATCH_SIZE: usize = 1000;
        for batch in all_chunks.chunks(BATCH_SIZE) {
            self.calculate_ambient_occlusion(
                encoder,
                world_buffer,
                batch,
                1, // Single smoothing pass for performance in batch mode
            );
        }
    }
}

/// Extract AO value from voxel metadata
pub fn extract_ao_from_metadata(metadata: u8) -> u8 {
    metadata & 0xF
}

/// Get AO factor for rendering (0.0 = fully occluded, 1.0 = no occlusion)
pub fn ao_to_factor(ao_value: u8) -> f32 {
    1.0 - (ao_value as f32 / 15.0) * 0.5 // Max 50% darkening
}