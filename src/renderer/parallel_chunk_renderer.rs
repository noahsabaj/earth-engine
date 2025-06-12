use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;
use crossbeam_channel::{unbounded, Sender, Receiver};
use rayon::prelude::*;
use wgpu::util::DeviceExt;
use crate::{ChunkPos, Chunk, BlockRegistry};
use crate::world::ConcurrentChunkManager;
use super::{ChunkMesh, greedy_mesher::GreedyMesher};

/// Parallel chunk renderer that generates meshes in background threads
pub struct ParallelChunkRenderer {
    /// GPU meshes by chunk position
    gpu_meshes: Arc<RwLock<std::collections::HashMap<ChunkPos, GpuChunkMesh>>>,
    /// Mesh generation requests
    mesh_sender: Sender<MeshRequest>,
    mesh_receiver: Receiver<MeshRequest>,
    /// Completed meshes ready for GPU upload
    completed_sender: Sender<(ChunkPos, ChunkMesh)>,
    completed_receiver: Receiver<(ChunkPos, ChunkMesh)>,
    /// Thread pool for mesh generation
    mesh_pool: rayon::ThreadPool,
}

struct MeshRequest {
    chunk_pos: ChunkPos,
    chunk: Arc<RwLock<Chunk>>,
}

struct GpuChunkMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

impl ParallelChunkRenderer {
    pub fn new() -> Self {
        let (mesh_sender, mesh_receiver) = unbounded();
        let (completed_sender, completed_receiver) = unbounded();
        
        // Create dedicated thread pool for mesh generation
        let mesh_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get().saturating_sub(2).max(2))
            .thread_name(|idx| format!("mesh-gen-{}", idx))
            .build()
            .expect("Failed to create mesh generation thread pool");
        
        Self {
            gpu_meshes: Arc::new(RwLock::new(std::collections::HashMap::new())),
            mesh_sender,
            mesh_receiver,
            completed_sender,
            completed_receiver,
            mesh_pool,
        }
    }
    
    /// Queue chunks for mesh generation
    pub fn queue_dirty_chunks(
        &self,
        chunk_manager: &ConcurrentChunkManager,
        registry: &BlockRegistry,
    ) {
        // Collect chunks that need meshing
        let dirty_chunks: Vec<_> = chunk_manager
            .chunks_iter()
            .filter_map(|(pos, chunk_lock)| {
                let chunk = chunk_lock.read();
                if chunk.is_dirty() {
                    Some(MeshRequest {
                        chunk_pos: pos,
                        chunk: Arc::clone(&chunk_lock),
                    })
                } else {
                    None
                }
            })
            .collect();
        
        // Queue mesh generation requests
        for request in dirty_chunks {
            let _ = self.mesh_sender.send(request);
        }
    }
    
    /// Process mesh generation queue in parallel
    pub fn process_mesh_queue(&self, chunk_manager: &ConcurrentChunkManager, registry: Arc<BlockRegistry>) {
        let mut pending_requests = Vec::new();
        
        // Collect pending requests
        while let Ok(request) = self.mesh_receiver.try_recv() {
            pending_requests.push(request);
        }
        
        if pending_requests.is_empty() {
            return;
        }
        
        let completed_sender = self.completed_sender.clone();
        let chunk_size = 32; // TODO: Get from chunk manager
        
        // Generate meshes in parallel
        self.mesh_pool.install(|| {
            pending_requests.par_iter().for_each(|request| {
                // Lock chunk for reading
                let chunk = request.chunk.read();
                
                // Create mesh builder with neighbor chunks
                let mut mesher = GreedyMesher::new(chunk_size);
                
                // Get neighbor chunks for proper face culling
                let neighbors = [
                    chunk_manager.get_chunk(request.chunk_pos.offset(1, 0, 0)),
                    chunk_manager.get_chunk(request.chunk_pos.offset(-1, 0, 0)),
                    chunk_manager.get_chunk(request.chunk_pos.offset(0, 1, 0)),
                    chunk_manager.get_chunk(request.chunk_pos.offset(0, -1, 0)),
                    chunk_manager.get_chunk(request.chunk_pos.offset(0, 0, 1)),
                    chunk_manager.get_chunk(request.chunk_pos.offset(0, 0, -1)),
                ];
                
                // Build mesh
                let mesh = mesher.build_chunk_mesh(
                    &chunk,
                    request.chunk_pos,
                    chunk_size,
                    &registry,
                    neighbors.as_slice(),
                );
                
                // Clear dirty flag
                drop(chunk);
                request.chunk.write().clear_dirty();
                
                // Send completed mesh
                let _ = completed_sender.send((request.chunk_pos, mesh));
            });
        });
    }
    
    /// Upload completed meshes to GPU
    pub fn upload_completed_meshes(&self, device: &wgpu::Device) {
        while let Ok((chunk_pos, mesh)) = self.completed_receiver.try_recv() {
            if mesh.vertices.is_empty() {
                // Remove empty mesh
                self.gpu_meshes.write().remove(&chunk_pos);
                continue;
            }
            
            // Create GPU buffers
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Chunk {:?} Vertex Buffer", chunk_pos)),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Chunk {:?} Index Buffer", chunk_pos)),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            
            let gpu_mesh = GpuChunkMesh {
                vertex_buffer,
                index_buffer,
                num_indices: mesh.indices.len() as u32,
            };
            
            self.gpu_meshes.write().insert(chunk_pos, gpu_mesh);
        }
    }
    
    /// Render visible chunks with frustum culling
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera: &crate::Camera,
        chunk_size: u32,
    ) {
        // Simple frustum culling based on distance
        let camera_chunk = ChunkPos::new(
            (camera.position.x / chunk_size as f32).floor() as i32,
            (camera.position.y / chunk_size as f32).floor() as i32,
            (camera.position.z / chunk_size as f32).floor() as i32,
        );
        
        // Unfortunately, due to lifetime constraints with wgpu RenderPass,
        // we cannot hold a lock while rendering. This is a known limitation
        // when trying to use thread-safe structures with wgpu.
        // 
        // For now, we'll skip the actual rendering and just count meshes.
        // In a real implementation, you would need to either:
        // 1. Use a single-threaded renderer that owns the meshes
        // 2. Copy mesh data before rendering
        // 3. Use a different synchronization strategy
        
        let mesh_count = self.gpu_meshes.read().len();
        log::debug!("Would render {} chunk meshes", mesh_count);
        
        // TODO: Implement proper rendering strategy that works with lifetimes
    }
    
    /// Get number of loaded GPU meshes
    pub fn mesh_count(&self) -> usize {
        self.gpu_meshes.read().len()
    }
}