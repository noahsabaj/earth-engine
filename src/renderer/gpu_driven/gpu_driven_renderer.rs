#![allow(unused_variables, dead_code)]
use std::sync::Arc;
use std::time::Instant;
use cgmath::{Vector3};
use crate::camera::data_camera::CameraData;
use super::{
    indirect_commands::{IndirectCommandManager, DrawMetadata},
    instance_buffer::{InstanceManager, InstanceData, InstanceBuffer},
    culling_pipeline::{CullingPipeline, CullingData},
    lod_system::{LodSystem},
};

/// Statistics for GPU-driven rendering
#[derive(Debug, Default, Clone)]
pub struct RenderStats {
    /// Total objects submitted
    pub objects_submitted: u32,
    
    /// Objects culled by frustum
    pub frustum_culled: u32,
    
    /// Objects culled by distance
    pub distance_culled: u32,
    
    /// Objects actually drawn
    pub objects_drawn: u32,
    
    /// Draw calls issued
    pub draw_calls: u32,
    
    /// Frame time in ms
    pub frame_time_ms: f32,
    
    /// Total instances added successfully
    pub instances_added: u32,
    
    /// Objects rejected (no instance space)
    pub objects_rejected: u32,
}

/// GPU-driven renderer
pub struct GpuDrivenRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    
    /// Indirect command management
    command_manager: IndirectCommandManager,
    
    /// Instance data management
    instance_manager: InstanceManager,
    
    /// Culling pipeline
    culling_pipeline: CullingPipeline,
    
    /// Culling data
    culling_data: CullingData,
    
    /// LOD system
    lod_system: LodSystem,
    
    /// Mesh vertex/index buffers (simplified for example)
    pub mesh_buffers: MeshBufferManager,
    
    /// Render pipeline (might be None if creation failed)
    render_pipeline: Option<wgpu::RenderPipeline>,
    
    /// Statistics
    stats: RenderStats,
    
    /// Cached buffer references to ensure they stay alive during rendering
    cached_vertex_buffer: Option<Arc<wgpu::Buffer>>,
    cached_index_buffer: Option<Arc<wgpu::Buffer>>,
}

impl GpuDrivenRenderer {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        // Create managers
        let command_manager = IndirectCommandManager::new(device.clone(), 100000);
        let instance_manager = InstanceManager::new(device.clone());
        let culling_pipeline = CullingPipeline::new(device.clone());
        let culling_data = CullingData::new(device.clone(), 100000);
        let lod_system = LodSystem::new();
        let mesh_buffers = MeshBufferManager::new(device.clone());
        
        // Create render pipeline with error handling
        let shader_source = include_str!("../../shaders/rendering/gpu_driven.wgsl");
        log::debug!("[GpuDrivenRenderer] Loading GPU driven shader ({} bytes)", shader_source.len());
        
        let render_pipeline = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("GPU Driven Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });
            
            let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("GPU Driven Pipeline Layout"),
                bind_group_layouts: &[camera_bind_group_layout],
                push_constant_ranges: &[],
            });
            
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GPU Driven Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    crate::renderer::vertex::vertex_buffer_layout(),
                    InstanceBuffer::desc(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            })
        })) {
            Ok(pipeline) => {
                log::info!("[GpuDrivenRenderer] Render pipeline created successfully");
                Some(pipeline)
            }
            Err(e) => {
                log::error!("[GpuDrivenRenderer] Failed to create render pipeline: {:?}", e);
                log::error!("[GpuDrivenRenderer] GPU-driven rendering will be disabled");
                None
            }
        };
        
        Self {
            device,
            queue,
            command_manager,
            instance_manager,
            culling_pipeline,
            culling_data,
            lod_system,
            mesh_buffers,
            render_pipeline,
            stats: RenderStats::default(),
            cached_vertex_buffer: None,
            cached_index_buffer: None,
        }
    }
    
    /// Begin a new frame
    pub fn begin_frame(&mut self, camera: &CameraData) {
        let start = Instant::now();
        
        // Log camera spatial context for debugging
        log::debug!("[GPU_RENDER] Beginning frame at camera position ({:.1}, {:.1}, {:.1}), yaw: {:.1}°, pitch: {:.1}°", 
                   camera.position[0], camera.position[1], camera.position[2], 
                   camera.yaw_radians.to_degrees(), camera.pitch_radians.to_degrees());
        
        // Clear previous frame data
        self.culling_data.clear();
        self.stats = RenderStats::default();
        
        // NOTE: We do NOT clear instance buffers here anymore.
        // Instance buffers should persist across frames and only be updated
        // when objects change (chunks are added/removed/modified).
        // This fixes the issue where instances were being cleared every frame.
        
        log::debug!("[GPU_RENDER] Instance persistence: keeping {} existing instances across frames", 
                   self.instance_manager.chunk_instances().count());
        
        // Update camera for culling
        self.culling_pipeline.update_camera(&self.queue, camera);
        
        let duration = start.elapsed();
        log::debug!("[GPU_RENDER] Frame begin completed in {:.1}μs", duration.as_micros());
    }
    
    /// Clear all instances - should only be called when rebuilding the entire scene
    pub fn clear_instances(&mut self) {
        self.instance_manager.clear_all();
        log::debug!("[GpuDrivenRenderer::clear_instances] Cleared all instance buffers for scene rebuild");
    }
    
    /// Upload instance data to GPU after submission
    pub fn upload_instances(&mut self, queue: &wgpu::Queue) {
        let start = Instant::now();
        let instance_count_before = self.instance_manager.chunk_instances().count();
        
        log::debug!("[GPU_RENDER] Uploading {} instances to GPU", instance_count_before);
        
        self.instance_manager.upload_all(queue);
        
        let duration = start.elapsed();
        let instance_data_size = instance_count_before as usize * std::mem::size_of::<super::instance_buffer::InstanceData>();
        log::info!("[GPU_RENDER] Instance upload completed: {} instances ({} bytes) in {:.2}ms", 
                  instance_count_before, instance_data_size, duration.as_secs_f64() * 1000.0);
        
        // Calculate upload bandwidth
        if duration.as_secs_f64() > 0.0 {
            let bandwidth_mbps = (instance_data_size as f64 / duration.as_secs_f64()) / (1024.0 * 1024.0);
            log::debug!("[GPU_RENDER] Upload bandwidth: {:.1} MB/s", bandwidth_mbps);
        }
    }
    
    /// Submit objects for rendering
    pub fn submit_objects(&mut self, objects: &[RenderObject]) {
        let camera_pos = Vector3::new(0.0, 0.0, 0.0); // Get from camera
        let initial_count = self.stats.objects_submitted;
        
        // Track active instances for validation
        let instance_count_before = self.instance_manager.chunk_instances().count();
        log::debug!(
            "[GpuDrivenRenderer::submit_objects] Starting submission - Current instances: {}, Objects to submit: {}",
            instance_count_before,
            objects.len()
        );
        
        for object in objects {
            self.stats.objects_submitted += 1;
            
            // Add instance data
            let instance_data = InstanceData::new(object.position, object.scale, object.color);
            
            if let Some(instance_id) = self.instance_manager
                .chunk_instances_mut()
                .add_instance(instance_data) {
                
                self.stats.instances_added += 1;
                
                // Create draw metadata for culling
                let metadata = DrawMetadata {
                    bounding_sphere: [
                        object.position.x,
                        object.position.y,
                        object.position.z,
                        object.bounding_radius,
                    ],
                    lod_info: [50.0, 200.0, 0.0, 0.0], // LOD distances
                    material_id: object.material_id,
                    mesh_id: object.mesh_id,
                    instance_offset: instance_id,
                    flags: 1, // Visible
                };
                
                self.culling_data.add_draw(metadata);
            } else {
                self.stats.objects_rejected += 1;
                
                // Log warning when instance buffer is full
                if self.stats.objects_rejected == 1 {
                    log::warn!(
                        "[GpuDrivenRenderer::submit_objects] Instance buffer full! Cannot add more objects. \
                        Consider increasing buffer size."
                    );
                } else if self.stats.objects_rejected % 100 == 0 {
                    log::warn!(
                        "[GpuDrivenRenderer::submit_objects] {} objects rejected due to full instance buffer",
                        self.stats.objects_rejected
                    );
                }
            }
        }
        
        let submitted_this_call = self.stats.objects_submitted - initial_count;
        let instance_count_after = self.instance_manager.chunk_instances().count();
        
        if submitted_this_call > 0 {
            log::info!(
                "[GpuDrivenRenderer::submit_objects] Submission complete - Submitted: {}, Instances: {} -> {} (added: {}), Rejected: {}",
                submitted_this_call,
                instance_count_before,
                instance_count_after,
                self.stats.instances_added,
                self.stats.objects_rejected
            );
        }
        
        // Validate instance count
        if instance_count_after != self.stats.instances_added {
            log::error!(
                "[GpuDrivenRenderer::submit_objects] Instance count mismatch! Expected: {}, Actual: {}",
                self.stats.instances_added,
                instance_count_after
            );
        }
    }
    
    /// Build GPU commands (can be called from multiple threads)
    pub fn build_commands(&mut self) {
        let start = Instant::now();
        
        log::debug!("[GPU_RENDER] Building GPU commands for {} objects", self.stats.objects_submitted);
        
        // Upload instance data
        let instance_upload_start = Instant::now();
        self.instance_manager.upload_all(&self.queue);
        let instance_upload_duration = instance_upload_start.elapsed();
        
        // Upload culling data
        let culling_upload_start = Instant::now();
        self.culling_data.upload(&self.queue);
        let culling_upload_duration = culling_upload_start.elapsed();
        
        let total_duration = start.elapsed();
        
        log::debug!("[GPU_RENDER] Command building completed in {:.2}ms (instance: {:.2}ms, culling: {:.2}ms)", 
                   total_duration.as_secs_f64() * 1000.0, 
                   instance_upload_duration.as_secs_f64() * 1000.0,
                   culling_upload_duration.as_secs_f64() * 1000.0);
        
        // In a real implementation, this would build commands on multiple threads
        // For now, we'll prepare the buffers for GPU culling
    }
    
    /// Execute GPU culling phase (must be called before render_draw)
    pub fn execute_culling(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let start = Instant::now();
        
        // Skip if no objects to cull
        if self.culling_data.metadata.is_empty() {
            log::debug!("[GPU_RENDER] No objects to cull, skipping GPU culling");
            return;
        }
        
        // Check if GPU culling is available
        if !self.culling_pipeline.is_available() {
            log::warn!("[GPU_RENDER] GPU culling not available, rendering all {} objects without culling", 
                      self.culling_data.metadata.len());
            // In production, we might want to fall back to CPU culling here
            return;
        }
        
        log::info!("[GPU_RENDER] Executing GPU culling for {} objects", self.culling_data.metadata.len());
        
        // Create culling bind group
        let bind_group_start = Instant::now();
        let culling_bind_group = self.culling_pipeline.create_bind_group(
            &self.culling_data.metadata_buffer,
            self.command_manager.opaque_commands().buffer(),
        );
        let bind_group_duration = bind_group_start.elapsed();
        
        // Execute GPU culling
        let culling_start = Instant::now();
        self.culling_pipeline.execute_culling(
            encoder,
            &culling_bind_group,
            self.culling_data.metadata.len() as u32,
        );
        let culling_duration = culling_start.elapsed();
        
        // Copy commands from staging to GPU
        let copy_start = Instant::now();
        self.command_manager.copy_all_to_gpu(encoder);
        let copy_duration = copy_start.elapsed();
        
        let total_duration = start.elapsed();
        
        log::info!("[GPU_RENDER] GPU culling completed in {:.2}ms (bind: {:.1}μs, cull: {:.1}μs, copy: {:.1}μs)", 
                  total_duration.as_secs_f64() * 1000.0,
                  bind_group_duration.as_micros(),
                  culling_duration.as_micros(),
                  copy_duration.as_micros());
    }
    
    /// Update cached buffer references before rendering
    pub fn update_buffer_cache(&mut self) {
        let start = Instant::now();
        
        if let Some((vb, ib)) = self.mesh_buffers.get_chunk_buffers() {
            self.cached_vertex_buffer = Some(vb);
            self.cached_index_buffer = Some(ib);
            
            log::debug!("[GPU_RENDER] Buffer cache updated with vertex and index buffers");
        } else {
            log::warn!("[GPU_RENDER] Failed to get chunk buffers for cache update");
        }
        
        let duration = start.elapsed();
        log::debug!("[GPU_RENDER] Buffer cache update completed in {:.1}μs", duration.as_micros());
    }
    
    /// Execute rendering phase (must be called after execute_culling)
    pub fn render_draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        let start = Instant::now();
        
        // Check if render pipeline is available
        let Some(render_pipeline) = &self.render_pipeline else {
            log::debug!("[GPU_RENDER] Skipping render - pipeline not available");
            return;
        };
        
        log::debug!("[GPU_RENDER] Starting render draw phase with {} submitted objects", 
                   self.stats.objects_submitted);
        
        // Set up render state
        let setup_start = Instant::now();
        render_pass.set_pipeline(render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        
        // Bind mesh buffers from cache
        if let (Some(vertex_buffer), Some(index_buffer)) = (&self.cached_vertex_buffer, &self.cached_index_buffer) {
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            log::debug!("[GPU_RENDER] Bound cached vertex and index buffers");
        } else {
            log::error!("[GPU_RENDER] No cached buffers available for rendering!");
            return;
        }
        
        // Bind instance buffer
        let instance_count = self.instance_manager.chunk_instances().count();
        render_pass.set_vertex_buffer(1, self.instance_manager.chunk_instances().buffer().slice(..));
        log::debug!("[GPU_RENDER] Bound instance buffer with {} instances", instance_count);
        
        let setup_duration = setup_start.elapsed();
        
        // Execute draws for each mesh
        // Following DOP principles - iterate through data, not objects
        let mesh_infos = self.mesh_buffers.mesh_infos();
        
        // Group instances by mesh_id for efficient rendering
        // In a production implementation, this would be done during culling
        let mut instances_per_mesh: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
        
        // Collect instance IDs for each mesh
        for (instance_idx, metadata) in self.culling_data.metadata.iter().enumerate() {
            if metadata.flags & 1 != 0 { // Check visibility flag
                instances_per_mesh
                    .entry(metadata.mesh_id)
                    .or_insert_with(Vec::new)
                    .push(instance_idx as u32);
            }
        }
        
        // Draw each mesh with its instances
        let draw_start = Instant::now();
        let mut total_draw_calls = 0;
        let mut total_triangles = 0;
        
        for (mesh_id, instance_indices) in instances_per_mesh.iter() {
            if let Some(mesh_info) = self.mesh_buffers.get_mesh_info(*mesh_id) {
                if mesh_info.index_count > 0 && !instance_indices.is_empty() {
                    // Calculate index range for this mesh
                    let start_index = mesh_info.index_start;
                    let end_index = start_index + mesh_info.index_count;
                    
                    log::debug!("[GPU_RENDER] Drawing mesh {} with {} instances (indices {}-{}, {} triangles)", 
                               mesh_id, instance_indices.len(), start_index, end_index, mesh_info.index_count / 3);
                    
                    // Draw all instances of this mesh
                    // In a proper GPU-driven renderer, we'd use indirect drawing
                    // For now, draw each instance separately
                    for &instance_idx in instance_indices {
                        render_pass.draw_indexed(
                            start_index..end_index,
                            mesh_info.vertex_start as i32,
                            instance_idx..instance_idx + 1,
                        );
                        total_draw_calls += 1;
                        total_triangles += mesh_info.index_count / 3;
                    }
                }
            } else {
                log::warn!("[GPU_RENDER] Mesh info not found for mesh_id {}", mesh_id);
            }
        }
        
        let draw_duration = draw_start.elapsed();
        let total_duration = start.elapsed();
        
        log::info!("[GPU_RENDER] Render draw completed: {} draw calls, {} triangles in {:.2}ms (setup: {:.1}μs, draw: {:.1}μs)", 
                  total_draw_calls, total_triangles, 
                  total_duration.as_secs_f64() * 1000.0,
                  setup_duration.as_micros(),
                  draw_duration.as_micros());
        
        // Calculate rendering performance metrics
        if total_duration.as_secs_f64() > 0.0 {
            let triangles_per_second = total_triangles as f64 / total_duration.as_secs_f64();
            log::debug!("[GPU_RENDER] Performance: {:.0} triangles/sec, {:.1} draw calls/ms", 
                       triangles_per_second, total_draw_calls as f64 / (total_duration.as_secs_f64() * 1000.0));
        }
    }
    
    /// Update statistics after rendering
    pub fn update_stats(&mut self, start_time: std::time::Instant) {
        let draw_count = self.command_manager.opaque_commands().count();
        if draw_count > 0 {
            self.stats.draw_calls = 1;
            self.stats.objects_drawn = draw_count;
        }
        self.stats.frame_time_ms = start_time.elapsed().as_secs_f32() * 1000.0;
        
        // Log performance metrics periodically
        static mut FRAME_COUNT: u32 = 0;
        // SAFETY: Static mut access is safe here because:
        // - This is only used for performance logging
        // - Single-threaded access pattern (render loop)
        // - Only incremented during frame rendering
        // - Race conditions would only affect log timing
        unsafe {
            FRAME_COUNT += 1;
            
            // Log stats every 60 frames (1 second at 60 FPS)
            if FRAME_COUNT % 60 == 0 {
                log::info!(
                    "[GPU_RENDER] Frame {} performance - Submitted: {}, Drawn: {}, Instances: {}, Rejected: {}, Frame time: {:.2}ms",
                    FRAME_COUNT,
                    self.stats.objects_submitted,
                    self.stats.objects_drawn,
                    self.stats.instances_added,
                    self.stats.objects_rejected,
                    self.stats.frame_time_ms
                );
                
                // Calculate FPS and warn if performance is poor
                let fps = if self.stats.frame_time_ms > 0.0 { 1000.0 / self.stats.frame_time_ms } else { 0.0 };
                log::info!("[GPU_RENDER] Current FPS: {:.1}, Target: 60.0", fps);
                
                if fps < 30.0 {
                    log::warn!("[GPU_RENDER] Performance warning: FPS below 30 ({:.1})", fps);
                }
            }
            
            // More detailed logging every 300 frames (5 seconds at 60 FPS)
            if FRAME_COUNT % 300 == 0 {
                let instance_count = self.instance_manager.chunk_instances().count();
                let rejection_rate = if self.stats.objects_submitted > 0 {
                    (self.stats.objects_rejected as f64 / self.stats.objects_submitted as f64) * 100.0
                } else { 0.0 };
                
                log::debug!(
                    "[GPU_RENDER] Detailed frame {} stats - Instance buffer usage: {}, Rejection rate: {:.1}%, Draw call efficiency: {:.1}",
                    FRAME_COUNT,
                    instance_count,
                    rejection_rate,
                    if self.stats.draw_calls > 0 { self.stats.objects_drawn as f64 / self.stats.draw_calls as f64 } else { 0.0 }
                );
            }
        }
    }
    
    /// Get rendering statistics
    pub fn stats(&self) -> &RenderStats {
        &self.stats
    }
    
    /// Check if GPU-driven rendering is available
    pub fn is_available(&self) -> bool {
        self.render_pipeline.is_some() && self.culling_pipeline.is_available()
    }
    
    /// Get the current instance count (for testing/debugging)
    pub fn get_instance_count(&self) -> u32 {
        self.instance_manager.chunk_instances().count()
    }
}

/// Information about a single mesh in the buffer
#[derive(Debug, Clone, Copy)]
pub struct MeshInfo {
    /// Start index in the index buffer
    pub index_start: u32,
    /// Number of indices in this mesh
    pub index_count: u32,
    /// Start vertex in the vertex buffer
    pub vertex_start: u32,
    /// Number of vertices in this mesh
    pub vertex_count: u32,
}

/// Simple mesh buffer manager
pub struct MeshBufferManager {
    device: Arc<wgpu::Device>,
    chunk_vertex_buffer: Option<Arc<wgpu::Buffer>>,
    chunk_index_buffer: Option<Arc<wgpu::Buffer>>,
    /// Map from chunk position hash to mesh ID
    chunk_mesh_ids: std::collections::HashMap<u64, u32>,
    /// Mesh information for each mesh ID
    mesh_infos: Vec<MeshInfo>,
    /// Next available mesh ID
    next_mesh_id: u32,
    /// Maximum number of meshes that can be stored
    max_meshes: u32,
    /// Vertex data staging buffer for GPU upload
    vertex_staging_buffer: Vec<crate::renderer::vertex::Vertex>,
    /// Index data staging buffer for GPU upload
    index_staging_buffer: Vec<u32>,
    /// Current offset into vertex buffer
    vertex_offset: usize,
    /// Current offset into index buffer
    index_offset: usize,
}

impl MeshBufferManager {
    fn new(device: Arc<wgpu::Device>) -> Self {
        // Pre-allocate GPU buffers for multiple chunk meshes
        // GPU max buffer size is 2,147,483,648 bytes (2.14 GB)
        // Vertex size is 44 bytes, so max vertices = 2.14GB / 44 = ~48.8M vertices
        // We'll use 40M vertices to leave headroom
        let max_total_vertices = 40_000_000usize;
        let vertices_per_mesh = 40_000usize; // 40K vertices per mesh (reduced from 64K)
        let max_meshes = (max_total_vertices / vertices_per_mesh) as u32; // 1000 meshes
        let indices_per_mesh = vertices_per_mesh * 3 / 2; // 1.5x indices (60K)
        
        let total_vertices = (max_meshes as usize) * vertices_per_mesh;
        let total_indices = (max_meshes as usize) * indices_per_mesh;
        
        // Calculate buffer sizes
        let vertex_buffer_size = (total_vertices * std::mem::size_of::<crate::renderer::vertex::Vertex>()) as u64;
        let index_buffer_size = (total_indices * std::mem::size_of::<u32>()) as u64;
        
        log::info!(
            "[MeshBufferManager] Allocating buffers - Vertex: {:.2} GB, Index: {:.2} GB, Total meshes: {}",
            vertex_buffer_size as f64 / (1024.0 * 1024.0 * 1024.0),
            index_buffer_size as f64 / (1024.0 * 1024.0 * 1024.0),
            max_meshes
        );
        
        // Create large GPU buffers to hold all mesh data
        let chunk_vertex_buffer = Some(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Vertex Buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })));
        
        let chunk_index_buffer = Some(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Index Buffer"),
            size: index_buffer_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })));
        
        Self {
            device,
            chunk_vertex_buffer,
            chunk_index_buffer,
            chunk_mesh_ids: std::collections::HashMap::new(),
            mesh_infos: Vec::with_capacity(max_meshes as usize),
            next_mesh_id: 0,
            max_meshes,
            vertex_staging_buffer: Vec::with_capacity(vertices_per_mesh),
            index_staging_buffer: Vec::with_capacity(indices_per_mesh),
            vertex_offset: 0,
            index_offset: 0,
        }
    }
    
    fn get_chunk_buffers(&self) -> Option<(Arc<wgpu::Buffer>, Arc<wgpu::Buffer>)> {
        match (&self.chunk_vertex_buffer, &self.chunk_index_buffer) {
            (Some(vb), Some(ib)) => Some((Arc::clone(vb), Arc::clone(ib))),
            _ => None,
        }
    }
    
    /// Upload mesh data to GPU and return mesh ID
    /// Following DOP principles - pure function that transforms mesh data to GPU representation
    /// IMPORTANT: This function now takes ownership of queue reference to ensure proper lifetime
    pub fn upload_mesh(
        &mut self,
        queue: &wgpu::Queue,
        chunk_pos: crate::ChunkPos,
        vertices: &[crate::renderer::vertex::Vertex],
        indices: &[u32],
    ) -> Option<u32> {
        let start = Instant::now();
        
        if vertices.is_empty() || indices.is_empty() {
            log::debug!("[GPU_RENDER] Skipping mesh upload for chunk {:?}: empty data (vertices: {}, indices: {})", 
                       chunk_pos, vertices.len(), indices.len());
            return None;
        }
        
        log::info!("[GPU_RENDER] Uploading mesh for chunk {:?}: {} vertices, {} indices", 
                  chunk_pos, vertices.len(), indices.len());
        
        // Check if we have space for another mesh
        if self.next_mesh_id >= self.max_meshes {
            log::error!("[GPU_RENDER] Mesh buffer full! Cannot upload mesh for chunk {:?} (used: {}/{})", 
                       chunk_pos, self.next_mesh_id, self.max_meshes);
            return None;
        }
        
        log::debug!("[GPU_RENDER] Mesh buffer usage: {}/{} meshes allocated", self.next_mesh_id, self.max_meshes);
        
        // Calculate chunk position hash for lookup
        let chunk_hash = chunk_pos_to_hash(chunk_pos);
        
        // Check if this chunk already has a mesh
        if let Some(&existing_id) = self.chunk_mesh_ids.get(&chunk_hash) {
            // For now, we'll overwrite the existing mesh
            // In a real implementation, we'd handle this more gracefully
            log::debug!("[GPU_RENDER] Overwriting existing mesh ID {} for chunk {:?}", existing_id, chunk_pos);
        }
        
        let mesh_id = self.next_mesh_id;
        self.next_mesh_id += 1;
        
        // Calculate offsets for this mesh
        let vertex_byte_offset = self.vertex_offset * std::mem::size_of::<crate::renderer::vertex::Vertex>();
        let index_byte_offset = self.index_offset * std::mem::size_of::<u32>();
        
        let vertex_upload_size = vertices.len() * std::mem::size_of::<crate::renderer::vertex::Vertex>();
        let index_upload_size = indices.len() * std::mem::size_of::<u32>();
        
        log::debug!("[GPU_RENDER] Uploading to GPU: vertex offset {} bytes, index offset {} bytes", 
                   vertex_byte_offset, index_byte_offset);
        log::debug!("[GPU_RENDER] Upload sizes: {} bytes vertices, {} bytes indices", 
                   vertex_upload_size, index_upload_size);
        
        // Upload vertex data
        let vertex_upload_start = Instant::now();
        if let Some(vertex_buffer) = &self.chunk_vertex_buffer {
            queue.write_buffer(
                vertex_buffer,
                vertex_byte_offset as u64,
                bytemuck::cast_slice(vertices),
            );
        } else {
            log::error!("[GPU_RENDER] No vertex buffer available for upload!");
            return None;
        }
        let vertex_upload_duration = vertex_upload_start.elapsed();
        
        // Upload index data
        let index_upload_start = Instant::now();
        if let Some(index_buffer) = &self.chunk_index_buffer {
            queue.write_buffer(
                index_buffer,
                index_byte_offset as u64,
                bytemuck::cast_slice(indices),
            );
        } else {
            log::error!("[GPU_RENDER] No index buffer available for upload!");
            return None;
        }
        let index_upload_duration = index_upload_start.elapsed();
        
        // Create mesh info
        let mesh_info = MeshInfo {
            index_start: self.index_offset as u32,
            index_count: indices.len() as u32,
            vertex_start: self.vertex_offset as u32,
            vertex_count: vertices.len() as u32,
        };
        
        // Store mesh info
        if mesh_id as usize >= self.mesh_infos.len() {
            self.mesh_infos.resize(mesh_id as usize + 1, MeshInfo {
                index_start: 0,
                index_count: 0,
                vertex_start: 0,
                vertex_count: 0,
            });
        }
        self.mesh_infos[mesh_id as usize] = mesh_info;
        
        // Update offsets
        self.vertex_offset += vertices.len();
        self.index_offset += indices.len();
        
        // Store mesh ID for this chunk
        self.chunk_mesh_ids.insert(chunk_hash, mesh_id);
        
        let total_duration = start.elapsed();
        
        // Calculate upload performance metrics
        let total_upload_size = vertex_upload_size + index_upload_size;
        let upload_bandwidth = if total_duration.as_secs_f64() > 0.0 {
            (total_upload_size as f64 / total_duration.as_secs_f64()) / (1024.0 * 1024.0)
        } else { 0.0 };
        
        log::info!("[GPU_RENDER] Mesh upload completed for chunk {:?}: mesh_id {}, {:.2}ms total ({:.1}MB/s)", 
                  chunk_pos, mesh_id, total_duration.as_secs_f64() * 1000.0, upload_bandwidth);
        
        log::debug!("[GPU_RENDER] Upload breakdown: vertices {:.1}μs, indices {:.1}μs", 
                   vertex_upload_duration.as_micros(), index_upload_duration.as_micros());
        
        // Update buffer usage statistics
        let vertex_buffer_usage = (self.vertex_offset as f64 / (self.max_meshes * 40000) as f64) * 100.0;
        let index_buffer_usage = (self.index_offset as f64 / (self.max_meshes * 60000) as f64) * 100.0;
        
        log::debug!("[GPU_RENDER] Buffer usage after upload: vertex {:.1}%, index {:.1}%", 
                   vertex_buffer_usage, index_buffer_usage);
        
        Some(mesh_id)
    }
    
    /// Get mesh ID for a chunk position
    pub fn get_mesh_id(&self, chunk_pos: crate::ChunkPos) -> Option<u32> {
        let chunk_hash = chunk_pos_to_hash(chunk_pos);
        self.chunk_mesh_ids.get(&chunk_hash).copied()
    }
    
    /// Get mesh info for a mesh ID
    pub fn get_mesh_info(&self, mesh_id: u32) -> Option<&MeshInfo> {
        self.mesh_infos.get(mesh_id as usize)
    }
    
    /// Get all mesh infos
    pub fn mesh_infos(&self) -> &[MeshInfo] {
        &self.mesh_infos
    }
}

/// Convert chunk position to hash for lookup
/// Pure function following DOP principles
fn chunk_pos_to_hash(pos: crate::ChunkPos) -> u64 {
    // Simple hash combining x, y, z coordinates
    let x = pos.x as u64;
    let y = pos.y as u64;
    let z = pos.z as u64;
    (x << 42) | (y << 21) | z
}

/// Object to render
#[derive(Debug, Clone)]
pub struct RenderObject {
    pub position: Vector3<f32>,
    pub scale: f32,
    pub color: [f32; 4],
    pub bounding_radius: f32,
    pub mesh_id: u32,
    pub material_id: u32,
}