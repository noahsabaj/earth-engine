#![allow(unused_variables, dead_code)]
use super::{
    culling_pipeline::{CullingData, CullingPipeline},
    indirect_commands::{DrawMetadata, IndirectCommandManager},
    instance_buffer::{InstanceBuffer, InstanceData, InstanceManager},
    lod_system::LodSystem,
};
use crate::camera::data_camera::CameraData;
use cgmath::Vector3;
use std::sync::Arc;
use std::time::Instant;

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

    /// GPU meshing state reference for accessing GPU-generated meshes
    pub gpu_meshing: Option<Arc<crate::renderer::gpu_meshing::GpuMeshingState>>,

    /// Render pipeline (might be None if creation failed)
    render_pipeline: Option<wgpu::RenderPipeline>,

    /// Statistics
    stats: RenderStats,

    /// Map from mesh ID to index count (for CPU-generated meshes)
    mesh_index_counts: std::collections::HashMap<u32, u32>,
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

        // Create render pipeline with error handling
        let shader_source = include_str!("../../shaders/rendering/gpu_driven.wgsl");
        log::debug!(
            "[GpuDrivenRenderer] Loading GPU driven shader ({} bytes)",
            shader_source.len()
        );

        let render_pipeline =
            match crate::gpu::automation::create_gpu_shader(&device, "gpu_driven", shader_source) {
                Ok(validated_shader) => {
                    log::info!("[GpuDrivenRenderer] GPU driven shader validated successfully");

                    let render_pipeline_layout =
                        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some("GPU Driven Pipeline Layout"),
                            bind_group_layouts: &[camera_bind_group_layout],
                            push_constant_ranges: &[],
                        });

                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                            label: Some("GPU Driven Pipeline"),
                            layout: Some(&render_pipeline_layout),
                            vertex: wgpu::VertexState {
                                module: &validated_shader.module,
                                entry_point: "vs_main",
                                buffers: &[
                                    crate::renderer::vertex::vertex_buffer_layout(),
                                    InstanceBuffer::desc(),
                                ],
                            },
                            fragment: Some(wgpu::FragmentState {
                                module: &validated_shader.module,
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
                            log::error!(
                                "[GpuDrivenRenderer] Failed to create render pipeline: {:?}",
                                e
                            );
                            log::error!(
                                "[GpuDrivenRenderer] GPU-driven rendering will be disabled"
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "[GpuDrivenRenderer] Failed to create GPU driven shader: {}",
                        e
                    );
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
            gpu_meshing: None,
            render_pipeline,
            stats: RenderStats::default(),
            mesh_index_counts: std::collections::HashMap::new(),
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

        log::debug!(
            "[GPU_RENDER] Instance persistence: keeping {} existing instances across frames",
            self.instance_manager.chunk_instances().count()
        );

        // Update camera for culling
        self.culling_pipeline.update_camera(&self.queue, camera);

        let duration = start.elapsed();
        log::debug!(
            "[GPU_RENDER] Frame begin completed in {:.1}μs",
            duration.as_micros()
        );
    }

    /// Clear all instances - should only be called when rebuilding the entire scene
    pub fn clear_instances(&mut self) {
        self.instance_manager.clear_all();
        self.mesh_index_counts.clear();
        log::debug!("[GpuDrivenRenderer::clear_instances] Cleared all instance buffers and index counts for scene rebuild");
    }

    /// Upload instance data to GPU after submission
    pub fn upload_instances(&mut self, queue: &wgpu::Queue) {
        let start = Instant::now();
        let instance_count_before = self.instance_manager.chunk_instances().count();

        log::debug!(
            "[GPU_RENDER] Uploading {} instances to GPU",
            instance_count_before
        );

        self.instance_manager.upload_all(queue);

        let duration = start.elapsed();
        let instance_data_size = instance_count_before as usize
            * std::mem::size_of::<super::instance_buffer::InstanceData>();
        log::info!(
            "[GPU_RENDER] Instance upload completed: {} instances ({} bytes) in {:.2}ms",
            instance_count_before,
            instance_data_size,
            duration.as_secs_f64() * 1000.0
        );

        // Calculate upload bandwidth
        if duration.as_secs_f64() > 0.0 {
            let bandwidth_mbps =
                (instance_data_size as f64 / duration.as_secs_f64()) / (1024.0 * 1024.0);
            log::debug!("[GPU_RENDER] Upload bandwidth: {:.1} MB/s", bandwidth_mbps);
        }
    }

    /// Submit objects for rendering
    pub fn submit_objects(&mut self, objects: &[RenderObject]) {
        let camera_pos = Vector3::new(0.0, 0.0, 0.0); // Get from camera
        let initial_count = self.stats.objects_submitted;

        // Clear previous frame's culling data
        self.culling_data.clear();

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

            if let Some(instance_id) = self
                .instance_manager
                .chunk_instances_mut()
                .add_instance(instance_data)
            {
                self.stats.instances_added += 1;

                // Store index count if provided (for CPU-generated meshes)
                if let Some(index_count) = object.index_count {
                    self.mesh_index_counts.insert(object.mesh_id, index_count);
                    log::trace!(
                        "[GpuDrivenRenderer::submit_objects] Stored index count {} for mesh_id {}",
                        index_count,
                        object.mesh_id
                    );
                }

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

                log::trace!(
                    "[GpuDrivenRenderer::submit_objects] Added metadata for object at {:?}: mesh_id={}, instance_id={}",
                    object.position,
                    object.mesh_id,
                    instance_id
                );

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

        log::debug!(
            "[GPU_RENDER] Building GPU commands for {} objects",
            self.stats.objects_submitted
        );

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
            log::warn!(
                "[GPU_RENDER] GPU culling not available, rendering all {} objects without culling",
                self.culling_data.metadata.len()
            );
            // In production, we might want to fall back to CPU culling here
            return;
        }

        log::info!(
            "[GPU_RENDER] Executing GPU culling for {} objects",
            self.culling_data.metadata.len()
        );

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

        log::debug!(
            "[GPU_RENDER] Starting render draw phase with {} submitted objects",
            self.stats.objects_submitted
        );

        // Set up render state
        let setup_start = Instant::now();
        render_pass.set_pipeline(render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);

        // Bind instance buffer
        let instance_count = self.instance_manager.chunk_instances().count();
        render_pass.set_vertex_buffer(
            1,
            self.instance_manager.chunk_instances().buffer().slice(..),
        );
        log::debug!(
            "[GPU_RENDER] Bound instance buffer with {} instances",
            instance_count
        );

        let setup_duration = setup_start.elapsed();

        // Execute draws for each mesh
        // Following DOP principles - iterate through data, not objects

        // Group instances by mesh_id for efficient rendering
        // In a production implementation, this would be done during culling
        let mut instances_per_mesh: std::collections::HashMap<u32, Vec<u32>> =
            std::collections::HashMap::new();

        // Log metadata state for debugging
        log::debug!(
            "[GPU_RENDER] Culling metadata count: {}",
            self.culling_data.metadata.len()
        );

        // Collect instance IDs for each mesh
        for (instance_idx, metadata) in self.culling_data.metadata.iter().enumerate() {
            log::trace!(
                "[GPU_RENDER] Metadata[{}]: mesh_id={}, flags={}, visible={}",
                instance_idx,
                metadata.mesh_id,
                metadata.flags,
                metadata.flags & 1 != 0
            );

            if metadata.flags & 1 != 0 {
                // Check visibility flag
                instances_per_mesh
                    .entry(metadata.mesh_id)
                    .or_insert_with(Vec::new)
                    .push(instance_idx as u32);
            }
        }

        log::debug!(
            "[GPU_RENDER] Instances per mesh: {} meshes with instances",
            instances_per_mesh.len()
        );

        // Draw each mesh with its instances
        let draw_start = Instant::now();
        let mut total_draw_calls = 0;
        let mut total_triangles = 0;

        // Check if GPU meshing is available
        if let Some(gpu_meshing) = &self.gpu_meshing {
            for (mesh_id, instance_indices) in instances_per_mesh.iter() {
                // Get mesh buffer from GPU meshing system
                if let Some(mesh_buffer) =
                    crate::renderer::gpu_meshing::get_mesh_buffer(gpu_meshing, *mesh_id)
                {
                    log::debug!(
                        "[GPU_RENDER] Drawing GPU mesh {} with {} instances",
                        mesh_id,
                        instance_indices.len()
                    );

                    // Set GPU mesh buffers
                    render_pass.set_vertex_buffer(0, mesh_buffer.vertices.slice(..));
                    render_pass
                        .set_index_buffer(mesh_buffer.indices.slice(..), wgpu::IndexFormat::Uint32);

                    // Read actual mesh size from metadata buffer
                    // The GPU mesh generation writes vertex/index counts to the metadata
                    // For CPU meshes, we pass the actual index count in the render object
                    // Fixed: Now using indirect drawing for GPU-generated meshes

                    // Check if this is a GPU-generated mesh (has metadata)
                    let has_metadata = mesh_buffer.metadata.size() > 0;

                    let index_count = if has_metadata {
                        // Use indirect drawing for GPU-generated meshes
                        // The GPU mesh generation writes the actual vertex/index counts to metadata
                        log::trace!(
                            "[GPU_RENDER] Using indirect draw for GPU mesh {} with {} instances",
                            mesh_id,
                            instance_indices.len()
                        );

                        // Create indirect command from metadata
                        // Note: In a production system, this would be done on GPU
                        let indirect_cmd = wgpu::util::DrawIndexedIndirectArgs {
                            index_count: 0, // Will be filled from metadata
                            instance_count: instance_indices.len() as u32,
                            first_index: 0,
                            base_vertex: 0,
                            first_instance: 0,
                        };

                        // For now, use the stored index count until we implement GPU-side indirect
                        let index_count = self
                            .mesh_index_counts
                            .get(mesh_id)
                            .copied()
                            .unwrap_or(36u32); // Default cube

                        render_pass.draw_indexed(
                            0..index_count,
                            0,
                            0..instance_indices.len() as u32,
                        );
                        index_count
                    } else {
                        // Use direct drawing for CPU-generated meshes with known index count
                        let index_count = self
                            .mesh_index_counts
                            .get(mesh_id)
                            .copied()
                            .unwrap_or_else(|| {
                                log::warn!("[GPU_RENDER] No index count for CPU mesh {}", mesh_id);
                                36u32 // Default to cube size
                            });

                        log::trace!(
                            "[GPU_RENDER] Drawing CPU mesh {} with {} indices for {} instances",
                            mesh_id,
                            index_count,
                            instance_indices.len()
                        );

                        render_pass.draw_indexed(
                            0..index_count,
                            0,
                            0..instance_indices.len() as u32,
                        );
                        index_count
                    };
                    total_draw_calls += 1;
                    total_triangles += (index_count / 3) * instance_indices.len() as u32;
                } else {
                    log::warn!(
                        "[GPU_RENDER] GPU mesh not found for mesh_id {} (buffer index)",
                        mesh_id
                    );
                }
            }
        } else {
            log::warn!("[GPU_RENDER] No GPU meshing state available for rendering");
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
            log::debug!(
                "[GPU_RENDER] Performance: {:.0} triangles/sec, {:.1} draw calls/ms",
                triangles_per_second,
                total_draw_calls as f64 / (total_duration.as_secs_f64() * 1000.0)
            );
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
        use std::sync::atomic::{AtomicU32, Ordering};
        static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);

        let frame_count = FRAME_COUNT.fetch_add(1, Ordering::Relaxed) + 1;

        // Log stats every 60 frames (1 second at 60 FPS)
        if frame_count % 60 == 0 {
            log::info!(
                "[GPU_RENDER] Frame {} performance - Submitted: {}, Drawn: {}, Instances: {}, Rejected: {}, Frame time: {:.2}ms",
                frame_count,
                    self.stats.objects_submitted,
                    self.stats.objects_drawn,
                    self.stats.instances_added,
                    self.stats.objects_rejected,
                    self.stats.frame_time_ms
                );

            // Calculate FPS and warn if performance is poor
            let fps = if self.stats.frame_time_ms > 0.0 {
                1000.0 / self.stats.frame_time_ms
            } else {
                0.0
            };
            log::info!("[GPU_RENDER] Current FPS: {:.1}, Target: 60.0", fps);

            if fps < 30.0 {
                log::warn!(
                    "[GPU_RENDER] Performance warning: FPS below 30 ({:.1})",
                    fps
                );
            }
        }

        // More detailed logging every 300 frames (5 seconds at 60 FPS)
        if frame_count % 300 == 0 {
            let instance_count = self.instance_manager.chunk_instances().count();
            let rejection_rate = if self.stats.objects_submitted > 0 {
                (self.stats.objects_rejected as f64 / self.stats.objects_submitted as f64) * 100.0
            } else {
                0.0
            };

            log::debug!(
                    "[GPU_RENDER] Detailed frame {} stats - Instance buffer usage: {}, Rejection rate: {:.1}%, Draw call efficiency: {:.1}",
                    frame_count,
                    instance_count,
                    rejection_rate,
                    if self.stats.draw_calls > 0 { self.stats.objects_drawn as f64 / self.stats.draw_calls as f64 } else { 0.0 }
                );
        }
    }

    /// Get rendering statistics
    pub fn stats(&self) -> &RenderStats {
        &self.stats
    }

    /// Set the GPU meshing state reference
    pub fn set_gpu_meshing(
        &mut self,
        gpu_meshing: Arc<crate::renderer::gpu_meshing::GpuMeshingState>,
    ) {
        self.gpu_meshing = Some(gpu_meshing);
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
    pub index_count: Option<u32>, // Optional index count for CPU-generated meshes
}
