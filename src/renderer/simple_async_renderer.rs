use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use wgpu::util::DeviceExt;
use cgmath::{Point3, Vector4};
use crate::{
    ChunkPos, Chunk, BlockRegistry, Camera,
    renderer::AsyncMeshBuilder,
    world::{ParallelWorld, WorldInterface},
};
use crate::renderer::allocation_optimizations::{ChunkPositionBuffer, MeshRequestBuffer, STRING_POOL};

/// GPU-ready chunk mesh data
struct GpuChunkMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

/// Simple async chunk renderer that works with the existing World system
pub struct SimpleAsyncRenderer {
    /// GPU meshes indexed by chunk position
    gpu_meshes: HashMap<ChunkPos, GpuChunkMesh>,
    /// Async mesh builder
    mesh_builder: Arc<AsyncMeshBuilder>,
    /// Chunk size for calculations
    chunk_size: u32,
    /// Maximum mesh uploads per frame
    max_uploads_per_frame: usize,
    /// Pre-allocated buffer for dirty chunks
    dirty_chunks: ChunkPositionBuffer,
    /// Pre-allocated buffer for unloaded chunks
    unloaded_chunks: ChunkPositionBuffer,
}

impl SimpleAsyncRenderer {
    pub fn new(
        registry: Arc<BlockRegistry>,
        chunk_size: u32,
        mesh_threads: Option<usize>,
    ) -> Self {
        let mesh_builder = Arc::new(AsyncMeshBuilder::new(
            registry,
            chunk_size,
            mesh_threads,
        ));
        
        Self {
            gpu_meshes: HashMap::new(),
            mesh_builder,
            chunk_size,
            max_uploads_per_frame: 8,
            dirty_chunks: ChunkPositionBuffer::with_capacity(256),
            unloaded_chunks: ChunkPositionBuffer::with_capacity(128),
        }
    }
    
    /// Queue dirty chunks for mesh rebuilding
    pub fn queue_dirty_chunks(
        &mut self,
        world: &ParallelWorld,
        camera: &Camera,
    ) {
        let mut queued_count = 0;
        let camera_chunk = ChunkPos::new(
            (camera.position.x / self.chunk_size as f32).floor() as i32,
            (camera.position.y / self.chunk_size as f32).floor() as i32,
            (camera.position.z / self.chunk_size as f32).floor() as i32,
        );
        
        // Get all chunks from the parallel world's chunk manager
        let chunk_manager = world.chunk_manager();
        
        // Log chunk count on first few calls
        static mut QUEUE_COUNT: usize = 0;
        unsafe {
            if QUEUE_COUNT < 10 {
                let chunk_count = chunk_manager.loaded_chunk_count();
                log::info!("[SimpleAsyncRenderer::queue_dirty_chunks] Loaded chunks: {}", chunk_count);
                QUEUE_COUNT += 1;
            }
        }
        
        // Process chunks that need mesh updates
        for (chunk_pos, chunk_lock) in chunk_manager.chunks_iter() {
            // Check if chunk needs mesh rebuild
            let needs_rebuild = {
                let chunk = chunk_lock.read();
                chunk.is_dirty()
            };
            
            if needs_rebuild {
                // Calculate priority (closer = higher priority = lower value)
                let distance_sq = chunk_pos.distance_squared_to(camera_chunk);
                let priority = (distance_sq as f32).sqrt() as i32;
                
                // Get neighbor chunks for face culling
                let neighbors = [
                    chunk_manager.get_chunk(ChunkPos::new(chunk_pos.x + 1, chunk_pos.y, chunk_pos.z)),
                    chunk_manager.get_chunk(ChunkPos::new(chunk_pos.x - 1, chunk_pos.y, chunk_pos.z)),
                    chunk_manager.get_chunk(ChunkPos::new(chunk_pos.x, chunk_pos.y + 1, chunk_pos.z)),
                    chunk_manager.get_chunk(ChunkPos::new(chunk_pos.x, chunk_pos.y - 1, chunk_pos.z)),
                    chunk_manager.get_chunk(ChunkPos::new(chunk_pos.x, chunk_pos.y, chunk_pos.z + 1)),
                    chunk_manager.get_chunk(ChunkPos::new(chunk_pos.x, chunk_pos.y, chunk_pos.z - 1)),
                ];
                
                // Queue the mesh build using the existing chunk lock
                self.mesh_builder.queue_chunk(
                    chunk_pos,
                    Arc::clone(&chunk_lock),
                    priority,
                    neighbors,
                );
                
                // Mark chunk as clean after queuing
                chunk_lock.write().clear_dirty();
                queued_count += 1;
            }
        }
        
        if queued_count > 0 {
            log::info!("[SimpleAsyncRenderer::queue_dirty_chunks] Queued {} chunks for mesh building", queued_count);
        }
    }
    
    /// Update the renderer, processing mesh builds and uploads
    pub fn update(&mut self, device: &wgpu::Device) {
        // Process mesh building queue
        self.mesh_builder.process_queue(16);
        
        // Upload completed meshes to GPU
        self.upload_completed_meshes(device);
    }
    
    /// Upload completed meshes to GPU
    fn upload_completed_meshes(&mut self, device: &wgpu::Device) {
        let completed_meshes = self.mesh_builder.get_completed_meshes();
        let mut uploaded = 0;
        
        // Log completed meshes
        if !completed_meshes.is_empty() {
            log::info!("[SimpleAsyncRenderer::upload_completed_meshes] Got {} completed meshes", completed_meshes.len());
        }
        
        for completed in completed_meshes {
            if uploaded >= self.max_uploads_per_frame {
                break;
            }
            
            if completed.mesh.vertices.is_empty() {
                // Empty mesh, remove from GPU
                log::warn!("[SimpleAsyncRenderer::upload_completed_meshes] Empty mesh for chunk {:?}", completed.chunk_pos);
                self.gpu_meshes.remove(&completed.chunk_pos);
                continue;
            }
            
            log::info!("[SimpleAsyncRenderer::upload_completed_meshes] Uploading mesh for chunk {:?} with {} vertices, {} indices", 
                      completed.chunk_pos, completed.mesh.vertices.len(), completed.mesh.indices.len());
            
            // Create GPU buffers - use static labels to avoid allocation
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Vertices"),
                contents: bytemuck::cast_slice(&completed.mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Indices"),
                contents: bytemuck::cast_slice(&completed.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            
            let gpu_mesh = GpuChunkMesh {
                vertex_buffer,
                index_buffer,
                num_indices: completed.mesh.indices.len() as u32,
            };
            
            log::info!("[SimpleAsyncRenderer::upload_completed_meshes] Uploaded mesh for chunk {:?} ({} vertices, {} indices)", 
                      completed.chunk_pos, completed.mesh.vertices.len(), gpu_mesh.num_indices);
            
            self.gpu_meshes.insert(completed.chunk_pos, gpu_mesh);
            uploaded += 1;
        }
    }
    
    /// Remove meshes for unloaded chunks
    pub fn cleanup_unloaded_chunks(&mut self, world: &ParallelWorld) {
        // Clear reusable buffer
        self.unloaded_chunks.clear();
        
        // Get currently loaded chunk positions
        let loaded_positions: std::collections::HashSet<ChunkPos> = world
            .get_loaded_chunk_positions()
            .into_iter()
            .collect();
        
        // Find GPU meshes that no longer have corresponding loaded chunks
        for pos in self.gpu_meshes.keys() {
            if !loaded_positions.contains(pos) {
                self.unloaded_chunks.push(*pos);
            }
        }
        
        // Remove GPU buffers for unloaded chunks
        for pos in self.unloaded_chunks.iter() {
            self.gpu_meshes.remove(pos);
        }
    }
    
    /// Render visible chunks with frustum culling
    /// Returns the number of chunks rendered
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera: &Camera,
    ) -> usize {
        let view_proj = camera.build_projection_matrix() * camera.build_view_matrix();
        let mut chunks_rendered = 0;
        let total_meshes = self.gpu_meshes.len();
        
        if total_meshes == 0 {
            // Only log occasionally to avoid spam
            static mut LAST_LOG: std::time::Instant = std::time::Instant::now();
            unsafe {
                if LAST_LOG.elapsed().as_secs() >= 1 {
                    log::debug!("[SimpleAsyncRenderer::render] No GPU meshes available to render (queued: {})", self.mesh_builder.active_builds());
                    LAST_LOG = std::time::Instant::now();
                }
            }
            return 0;
        }
        
        // Log mesh count on first few renders
        static mut RENDER_COUNT: usize = 0;
        unsafe {
            if RENDER_COUNT < 10 {
                log::info!("[SimpleAsyncRenderer::render] GPU meshes available: {}", self.gpu_meshes.len());
                RENDER_COUNT += 1;
            }
        }
        
        for (chunk_pos, gpu_mesh) in &self.gpu_meshes {
            // Calculate chunk bounds
            let min = Point3::new(
                (chunk_pos.x * self.chunk_size as i32) as f32,
                (chunk_pos.y * self.chunk_size as i32) as f32,
                (chunk_pos.z * self.chunk_size as i32) as f32,
            );
            let max = Point3::new(
                min.x + self.chunk_size as f32,
                min.y + self.chunk_size as f32,
                min.z + self.chunk_size as f32,
            );
            
            // Simple frustum culling - check if chunk center is in view
            let center = Point3::new(
                (min.x + max.x) * 0.5,
                (min.y + max.y) * 0.5,
                (min.z + max.z) * 0.5,
            );
            
            let clip_pos = view_proj * Vector4::new(center.x, center.y, center.z, 1.0);
            
            // Check if in frustum (rough check)
            // TEMPORARY: Bypass frustum culling to debug rendering
            let bypass_frustum = true;
            
            if bypass_frustum || clip_pos.w > 0.0 {
                let ndc_x = if clip_pos.w != 0.0 { clip_pos.x / clip_pos.w } else { 0.0 };
                let ndc_y = if clip_pos.w != 0.0 { clip_pos.y / clip_pos.w } else { 0.0 };
                let ndc_z = if clip_pos.w != 0.0 { clip_pos.z / clip_pos.w } else { 0.0 };
                
                // Expand bounds a bit to avoid culling edge chunks
                if bypass_frustum || (ndc_x >= -1.5 && ndc_x <= 1.5 &&
                   ndc_y >= -1.5 && ndc_y <= 1.5 &&
                   ndc_z >= 0.0 && ndc_z <= 1.0) {
                    render_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..gpu_mesh.num_indices, 0, 0..1);
                    chunks_rendered += 1;
                }
            }
        }
        
        // Log rendering stats occasionally
        static mut RENDER_COUNTER: u32 = 0;
        unsafe {
            RENDER_COUNTER += 1;
            if RENDER_COUNTER % 60 == 0 {  // Every ~1 second at 60fps
                if chunks_rendered > 0 {
                    log::info!("[SimpleAsyncRenderer::render] Rendered {}/{} chunks, {} queued for building", 
                              chunks_rendered, total_meshes, self.mesh_builder.active_builds());
                } else if total_meshes > 0 {
                    log::warn!("[SimpleAsyncRenderer::render] All {} chunks were frustum culled!", total_meshes);
                }
            }
        }
        
        chunks_rendered
    }
    
    /// Get total number of GPU meshes
    pub fn mesh_count(&self) -> usize {
        self.gpu_meshes.len()
    }
    
    /// Get number of queued mesh builds
    pub fn queued_builds(&self) -> usize {
        self.mesh_builder.active_builds()
    }
    
    /// Clear all meshes and queues
    pub fn clear(&mut self) {
        self.gpu_meshes.clear();
        self.mesh_builder.clear_queue();
    }
}

/// Lightweight snapshot of chunk data for async processing
struct ChunkSnapshot {
    position: ChunkPos,
    size: u32,
    blocks: Vec<crate::BlockId>,
}

impl ChunkSnapshot {
    fn from_chunk(chunk: &Chunk) -> Self {
        Self {
            position: chunk.position(),
            size: chunk.size(),
            blocks: chunk.blocks().to_vec(),
        }
    }
    
    fn into_chunk(self) -> Chunk {
        let mut chunk = Chunk::new(self.position, self.size);
        // Copy blocks back
        for (i, &block) in self.blocks.iter().enumerate() {
            let x = (i % self.size as usize) as u32;
            let y = ((i / self.size as usize) % self.size as usize) as u32;
            let z = (i / (self.size * self.size) as usize) as u32;
            chunk.set_block(x, y, z, block);
        }
        chunk.mark_clean(); // It's just a snapshot, not actually dirty
        chunk
    }
}