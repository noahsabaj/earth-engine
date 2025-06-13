use std::sync::Arc;
use wgpu::util::DeviceExt;
use cgmath::{Vector3};
use crate::Camera;
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
        let shader_source = include_str!("../shaders/gpu_driven.wgsl");
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
    pub fn begin_frame(&mut self, camera: &Camera) {
        // Clear previous frame data
        self.culling_data.clear();
        self.stats = RenderStats::default();
        
        // NOTE: We do NOT clear instance buffers here anymore.
        // Instance buffers should persist across frames and only be updated
        // when objects change (chunks are added/removed/modified).
        // This fixes the issue where instances were being cleared every frame.
        
        // Update camera for culling
        self.culling_pipeline.update_camera(&self.queue, camera);
    }
    
    /// Clear all instances - should only be called when rebuilding the entire scene
    pub fn clear_instances(&mut self) {
        self.instance_manager.clear_all();
        log::debug!("[GpuDrivenRenderer::clear_instances] Cleared all instance buffers for scene rebuild");
    }
    
    /// Upload instance data to GPU after submission
    pub fn upload_instances(&mut self, queue: &wgpu::Queue) {
        self.instance_manager.upload_all(queue);
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
        // Upload instance data
        self.instance_manager.upload_all(&self.queue);
        
        // Upload culling data
        self.culling_data.upload(&self.queue);
        
        // In a real implementation, this would build commands on multiple threads
        // For now, we'll prepare the buffers for GPU culling
    }
    
    /// Execute GPU culling phase (must be called before render_draw)
    pub fn execute_culling(&mut self, encoder: &mut wgpu::CommandEncoder) {
        // Skip if no objects to cull
        if self.culling_data.metadata.is_empty() {
            log::debug!("[GpuDrivenRenderer] No objects to cull, skipping GPU culling");
            return;
        }
        
        // Check if GPU culling is available
        if !self.culling_pipeline.is_available() {
            log::warn!("[GpuDrivenRenderer] GPU culling not available, rendering all objects");
            // In production, we might want to fall back to CPU culling here
            return;
        }
        
        // Create culling bind group
        let culling_bind_group = self.culling_pipeline.create_bind_group(
            &self.culling_data.metadata_buffer,
            self.command_manager.opaque_commands().buffer(),
        );
        
        // Execute GPU culling
        self.culling_pipeline.execute_culling(
            encoder,
            &culling_bind_group,
            self.culling_data.metadata.len() as u32,
        );
        
        // Copy commands from staging to GPU
        self.command_manager.copy_all_to_gpu(encoder);
    }
    
    /// Update cached buffer references before rendering
    pub fn update_buffer_cache(&mut self) {
        if let Some((vb, ib)) = self.mesh_buffers.get_chunk_buffers() {
            self.cached_vertex_buffer = Some(vb);
            self.cached_index_buffer = Some(ib);
        }
    }
    
    /// Execute rendering phase (must be called after execute_culling)
    pub fn render_draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        // Check if render pipeline is available
        let Some(render_pipeline) = &self.render_pipeline else {
            log::debug!("[GpuDrivenRenderer] Skipping render - pipeline not available");
            return;
        };
        
        // Set up render state
        render_pass.set_pipeline(render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        
        // Bind mesh buffers from cache
        if let (Some(vertex_buffer), Some(index_buffer)) = (&self.cached_vertex_buffer, &self.cached_index_buffer) {
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        }
        
        // Bind instance buffer
        render_pass.set_vertex_buffer(1, self.instance_manager.chunk_instances().buffer().slice(..));
        
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
        for (mesh_id, instance_indices) in instances_per_mesh.iter() {
            if let Some(mesh_info) = self.mesh_buffers.get_mesh_info(*mesh_id) {
                if mesh_info.index_count > 0 && !instance_indices.is_empty() {
                    // Calculate index range for this mesh
                    let start_index = mesh_info.index_start;
                    let end_index = start_index + mesh_info.index_count;
                    
                    // Draw all instances of this mesh
                    // In a proper GPU-driven renderer, we'd use indirect drawing
                    // For now, draw each instance separately
                    for &instance_idx in instance_indices {
                        render_pass.draw_indexed(
                            start_index..end_index,
                            mesh_info.vertex_start as i32,
                            instance_idx..instance_idx + 1,
                        );
                    }
                }
            }
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
            if FRAME_COUNT % 300 == 0 {
                log::debug!(
                    "[GpuDrivenRenderer] Frame {} stats - Submitted: {}, Drawn: {}, Instances: {}, Rejected: {}, Frame time: {:.2}ms",
                    FRAME_COUNT,
                    self.stats.objects_submitted,
                    self.stats.objects_drawn,
                    self.stats.instances_added,
                    self.stats.objects_rejected,
                    self.stats.frame_time_ms
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
        if vertices.is_empty() || indices.is_empty() {
            return None;
        }
        
        // Check if we have space for another mesh
        if self.next_mesh_id >= self.max_meshes {
            log::warn!("[MeshBufferManager] Mesh buffer full, cannot upload new mesh");
            return None;
        }
        
        // Calculate chunk position hash for lookup
        let chunk_hash = chunk_pos_to_hash(chunk_pos);
        
        // Check if this chunk already has a mesh
        if let Some(&existing_id) = self.chunk_mesh_ids.get(&chunk_hash) {
            // For now, we'll overwrite the existing mesh
            // In a real implementation, we'd handle this more gracefully
            log::debug!("[MeshBufferManager] Overwriting existing mesh for chunk {:?}", chunk_pos);
        }
        
        let mesh_id = self.next_mesh_id;
        self.next_mesh_id += 1;
        
        // Calculate offsets for this mesh
        let vertex_byte_offset = self.vertex_offset * std::mem::size_of::<crate::renderer::vertex::Vertex>();
        let index_byte_offset = self.index_offset * std::mem::size_of::<u32>();
        
        // Upload vertex data
        if let Some(vertex_buffer) = &self.chunk_vertex_buffer {
            queue.write_buffer(
                vertex_buffer,
                vertex_byte_offset as u64,
                bytemuck::cast_slice(vertices),
            );
        }
        
        // Upload index data
        if let Some(index_buffer) = &self.chunk_index_buffer {
            queue.write_buffer(
                index_buffer,
                index_byte_offset as u64,
                bytemuck::cast_slice(indices),
            );
        }
        
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