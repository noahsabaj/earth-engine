//! GPU-driven culling pipeline
//! 
//! This file uses deprecated Camera but will be migrated to data-oriented design
//! in a future sprint.

#![allow(deprecated)]

use std::sync::Arc;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4};
use super::indirect_commands::DrawMetadata;
use crate::camera::Camera;

/// Camera data for culling shader
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraData {
    /// View-projection matrix
    pub view_proj: [[f32; 4]; 4],
    
    /// Camera world position
    pub position: [f32; 3],
    
    /// Padding for alignment
    pub _padding0: f32,
    
    /// Frustum planes for GPU culling (6 planes)
    pub frustum_planes: [[f32; 4]; 6],
    
    /// Additional padding to reach 208 bytes total
    pub _padding1: [f32; 8],
}

impl CameraData {
    pub fn from_camera(camera: &Camera) -> Self {
        let view_proj = camera.build_projection_matrix() * camera.build_view_matrix();
        let position = [camera.position.x, camera.position.y, camera.position.z];
        
        // Extract frustum planes from view-projection matrix
        let frustum_planes = extract_frustum_planes(&view_proj);
        
        Self {
            view_proj: view_proj.into(),
            position,
            _padding0: 0.0,
            frustum_planes,
            _padding1: [0.0; 8],
        }
    }
}

/// Draw count for atomic counter
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct DrawCount {
    count: u32,
}

/// Culling statistics for debugging
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CullingStats {
    pub total_tested: u32,
    pub frustum_culled: u32,
    pub distance_culled: u32,
    pub drawn: u32,
}

/// GPU culling pipeline
pub struct CullingPipeline {
    /// Compute pipeline for culling (might be None if creation failed)
    cull_pipeline: Option<wgpu::ComputePipeline>,
    
    /// Compute pipeline for resetting counters (might be None if creation failed)
    reset_pipeline: Option<wgpu::ComputePipeline>,
    
    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,
    
    /// Camera uniform buffer
    camera_buffer: wgpu::Buffer,
    
    /// Draw count buffer
    draw_count_buffer: wgpu::Buffer,
    
    /// Culling stats buffer
    stats_buffer: wgpu::Buffer,
    
    /// Staging buffer for stats readback
    stats_staging: wgpu::Buffer,
    
    /// Flag indicating if pipelines are available
    pipelines_available: bool,
    
    /// Reference to device that created these resources
    device: Arc<wgpu::Device>,
}

impl CullingPipeline {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        // Load shader with error reporting
        let shader_source = include_str!("../shaders/gpu_culling.wgsl");
        log::debug!("[CullingPipeline] Loading GPU culling shader ({} bytes)", shader_source.len());
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GPU Culling Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Culling Bind Group Layout"),
            entries: &[
                // Camera data
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
                // Draw metadata
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
                // Indirect commands
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
                // Draw count
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Culling stats
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
            label: Some("Culling Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Try to create culling pipeline with error handling
        let cull_pipeline = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Cull Instances Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "cull_instances",
            })
        })) {
            Ok(pipeline) => {
                log::info!("[CullingPipeline] Culling compute pipeline created successfully");
                Some(pipeline)
            }
            Err(e) => {
                log::error!("[CullingPipeline] Failed to create culling pipeline: {:?}", e);
                log::error!("[CullingPipeline] GPU culling will be disabled for this session");
                None
            }
        };
        
        // Try to create reset pipeline with error handling
        let reset_pipeline = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Reset Counters Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "reset_counters",
            })
        })) {
            Ok(pipeline) => {
                log::info!("[CullingPipeline] Reset compute pipeline created successfully");
                Some(pipeline)
            }
            Err(e) => {
                log::error!("[CullingPipeline] Failed to create reset pipeline: {:?}", e);
                None
            }
        };
        
        let pipelines_available = cull_pipeline.is_some() && reset_pipeline.is_some();
        
        // Create buffers
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: std::mem::size_of::<CameraData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let draw_count_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Draw Count Buffer"),
            contents: bytemuck::bytes_of(&DrawCount { count: 0 }),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        let stats_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Culling Stats Buffer"),
            contents: bytemuck::bytes_of(&CullingStats {
                total_tested: 0,
                frustum_culled: 0,
                distance_culled: 0,
                drawn: 0,
            }),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        
        let stats_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Stats Staging Buffer"),
            size: std::mem::size_of::<CullingStats>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            cull_pipeline,
            reset_pipeline,
            bind_group_layout,
            camera_buffer,
            draw_count_buffer,
            stats_buffer,
            stats_staging,
            pipelines_available,
            device,
        }
    }
    
    /// Update camera data
    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        let camera_data = CameraData::from_camera(camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_data));
    }
    
    /// Create bind group for culling
    pub fn create_bind_group(
        &self,
        metadata_buffer: &wgpu::Buffer,
        commands_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Culling Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: metadata_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: commands_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.draw_count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.stats_buffer.as_entire_binding(),
                },
            ],
        })
    }
    
    /// Execute culling pass
    pub fn execute_culling(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        instance_count: u32,
    ) {
        // Check if pipelines are available
        if !self.pipelines_available {
            log::debug!("[CullingPipeline] Skipping GPU culling - pipelines not available");
            return;
        }
        
        // Reset counters
        if let Some(reset_pipeline) = &self.reset_pipeline {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Reset Counters Pass"),
                timestamp_writes: None,
            });
            
            pass.set_pipeline(reset_pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        
        // Execute culling
        if let Some(cull_pipeline) = &self.cull_pipeline {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Culling Pass"),
                timestamp_writes: None,
            });
            
            pass.set_pipeline(cull_pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            
            // Dispatch with 64 threads per workgroup
            let workgroups = (instance_count + 63) / 64;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
    }
    
    /// Check if culling is available
    pub fn is_available(&self) -> bool {
        self.pipelines_available
    }
    
    /// Copy stats for CPU readback
    pub fn copy_stats(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_buffer(
            &self.stats_buffer,
            0,
            &self.stats_staging,
            0,
            std::mem::size_of::<CullingStats>() as u64,
        );
    }
    
    /// Read stats from GPU (async)
    pub async fn read_stats(&self) -> Option<CullingStats> {
        let buffer_slice = self.stats_staging.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });
        
        receiver.await.ok()?.ok()?;
        
        let data = buffer_slice.get_mapped_range();
        let stats = bytemuck::from_bytes::<CullingStats>(&data).clone();
        
        drop(data);
        self.stats_staging.unmap();
        
        Some(stats)
    }
}

/// Data for culling multiple object types
pub struct CullingData {
    /// Draw metadata for all objects
    pub metadata: Vec<DrawMetadata>,
    
    /// GPU buffer for metadata
    pub metadata_buffer: wgpu::Buffer,
    
    /// Reference to device that created this buffer
    device: Arc<wgpu::Device>,
}

impl CullingData {
    pub fn new(device: Arc<wgpu::Device>, capacity: u32) -> Self {
        let metadata = Vec::with_capacity(capacity as usize);
        
        let metadata_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Draw Metadata Buffer"),
            size: (std::mem::size_of::<DrawMetadata>() * capacity as usize) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            metadata,
            metadata_buffer,
            device,
        }
    }
    
    /// Add draw metadata
    pub fn add_draw(&mut self, metadata: DrawMetadata) {
        self.metadata.push(metadata);
    }
    
    /// Add multiple draw metadata in batch (DOP compliant)
    pub fn add_draws_batch(&mut self, metadata: &[DrawMetadata]) {
        self.metadata.extend_from_slice(metadata);
    }
    
    /// Clear all draws
    pub fn clear(&mut self) {
        self.metadata.clear();
    }
    
    /// Upload to GPU
    pub fn upload(&self, queue: &wgpu::Queue) {
        if !self.metadata.is_empty() {
            queue.write_buffer(
                &self.metadata_buffer,
                0,
                bytemuck::cast_slice(&self.metadata),
            );
        }
    }
}

/// Extract frustum planes from view-projection matrix
fn extract_frustum_planes(view_proj: &Matrix4<f32>) -> [[f32; 4]; 6] {
    let mut planes = [[0.0; 4]; 6];
    
    // Left plane
    planes[0] = [
        view_proj.w.x + view_proj.x.x,
        view_proj.w.y + view_proj.x.y,
        view_proj.w.z + view_proj.x.z,
        view_proj.w.w + view_proj.x.w,
    ];
    
    // Right plane
    planes[1] = [
        view_proj.w.x - view_proj.x.x,
        view_proj.w.y - view_proj.x.y,
        view_proj.w.z - view_proj.x.z,
        view_proj.w.w - view_proj.x.w,
    ];
    
    // Bottom plane
    planes[2] = [
        view_proj.w.x + view_proj.y.x,
        view_proj.w.y + view_proj.y.y,
        view_proj.w.z + view_proj.y.z,
        view_proj.w.w + view_proj.y.w,
    ];
    
    // Top plane
    planes[3] = [
        view_proj.w.x - view_proj.y.x,
        view_proj.w.y - view_proj.y.y,
        view_proj.w.z - view_proj.y.z,
        view_proj.w.w - view_proj.y.w,
    ];
    
    // Near plane
    planes[4] = [
        view_proj.w.x + view_proj.z.x,
        view_proj.w.y + view_proj.z.y,
        view_proj.w.z + view_proj.z.z,
        view_proj.w.w + view_proj.z.w,
    ];
    
    // Far plane
    planes[5] = [
        view_proj.w.x - view_proj.z.x,
        view_proj.w.y - view_proj.z.y,
        view_proj.w.z - view_proj.z.z,
        view_proj.w.w - view_proj.z.w,
    ];
    
    // Normalize planes
    for plane in &mut planes {
        let length = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
        if length > 0.0 {
            plane[0] /= length;
            plane[1] /= length;
            plane[2] /= length;
            plane[3] /= length;
        }
    }
    
    planes
}