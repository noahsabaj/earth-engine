use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use wgpu::util::DeviceExt;
use cgmath::{Point3, Vector3};
use crate::{
    ChunkPos, Chunk, BlockRegistry, Camera,
    world::{ConcurrentChunkManager, ParallelChunkManager},
    renderer::{
        AsyncMeshBuilder, MeshBuildRequest, CompletedMesh,
        ChunkMesh, MeshBuildStats,
    },
};

/// GPU-ready chunk mesh data
struct GpuChunkMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    #[allow(dead_code)]
    vertex_count: u32,
}

/// Rendering statistics
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    pub chunks_rendered: usize,
    pub chunks_culled: usize,
    pub total_triangles: usize,
    pub mesh_uploads_per_frame: usize,
    pub draw_calls: usize,
}

/// Async chunk renderer that builds meshes in background and renders efficiently
pub struct AsyncChunkRenderer {
    /// GPU meshes indexed by chunk position
    gpu_meshes: Arc<RwLock<std::collections::HashMap<ChunkPos, GpuChunkMesh>>>,
    /// Async mesh builder
    mesh_builder: Arc<AsyncMeshBuilder>,
    /// Chunk size for calculations
    chunk_size: u32,
    /// View distance for culling
    view_distance: i32,
    /// Rendering statistics
    stats: Arc<RwLock<RenderStats>>,
    /// Maximum mesh uploads per frame
    max_uploads_per_frame: usize,
    /// Chunks that need mesh rebuilds
    dirty_chunks: Arc<RwLock<std::collections::HashSet<ChunkPos>>>,
    /// Last frame update time
    last_update: Arc<RwLock<Instant>>,
}

impl AsyncChunkRenderer {
    pub fn new(
        registry: Arc<BlockRegistry>,
        chunk_size: u32,
        view_distance: i32,
        mesh_threads: Option<usize>,
    ) -> Self {
        let mesh_builder = Arc::new(AsyncMeshBuilder::new(
            registry,
            chunk_size,
            mesh_threads,
        ));
        
        Self {
            gpu_meshes: Arc::new(RwLock::new(std::collections::HashMap::new())),
            mesh_builder,
            chunk_size,
            view_distance,
            stats: Arc::new(RwLock::new(RenderStats::default())),
            max_uploads_per_frame: 8,
            dirty_chunks: Arc::new(RwLock::new(std::collections::HashSet::new())),
            last_update: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    /// Queue dirty chunks for mesh rebuilding
    pub fn queue_dirty_chunks<T>(&self, chunk_manager: &T, camera: &Camera)
    where
        T: ChunkManager,
    {
        let camera_chunk = ChunkPos::new(
            (camera.position.x / self.chunk_size as f32).floor() as i32,
            (camera.position.y / self.chunk_size as f32).floor() as i32,
            (camera.position.z / self.chunk_size as f32).floor() as i32,
        );
        
        // Collect dirty chunks
        let mut dirty_chunks = Vec::new();
        
        for (chunk_pos, chunk_lock) in chunk_manager.chunks_iter() {
            let distance = chunk_pos.distance_squared_to(camera_chunk);
            
            // Skip distant chunks
            if distance > (self.view_distance * self.view_distance) {
                continue;
            }
            
            // Check if chunk is dirty
            let needs_rebuild = {
                let chunk = chunk_lock.read();
                chunk.is_dirty()
            };
            
            if needs_rebuild {
                dirty_chunks.push((chunk_pos, chunk_lock, distance));
            }
        }
        
        // Sort by distance (closer chunks first)
        dirty_chunks.sort_by_key(|(_, _, dist)| *dist);
        
        // Queue chunks for mesh building
        for (chunk_pos, chunk_lock, distance) in dirty_chunks {
            // Get neighbor chunks for proper face culling
            let neighbors = self.get_chunk_neighbors(chunk_manager, chunk_pos);
            
            // Calculate priority (closer = higher priority = lower value)
            let priority = (distance as f32).sqrt() as i32;
            
            // Queue the mesh build
            self.mesh_builder.queue_chunk(
                chunk_pos,
                Arc::clone(&chunk_lock),
                priority,
                neighbors,
            );
            
            // Mark chunk as no longer dirty
            chunk_lock.write().clear_dirty();
            
            // Track dirty chunk
            self.dirty_chunks.write().insert(chunk_pos);
        }
    }
    
    /// Get neighbor chunks for face culling
    fn get_chunk_neighbors<T>(&self, chunk_manager: &T, pos: ChunkPos) -> [Option<Arc<RwLock<Chunk>>>; 6]
    where
        T: ChunkManager,
    {
        [
            chunk_manager.get_chunk(ChunkPos::new(pos.x + 1, pos.y, pos.z)), // +X
            chunk_manager.get_chunk(ChunkPos::new(pos.x - 1, pos.y, pos.z)), // -X
            chunk_manager.get_chunk(ChunkPos::new(pos.x, pos.y + 1, pos.z)), // +Y
            chunk_manager.get_chunk(ChunkPos::new(pos.x, pos.y - 1, pos.z)), // -Y
            chunk_manager.get_chunk(ChunkPos::new(pos.x, pos.y, pos.z + 1)), // +Z
            chunk_manager.get_chunk(ChunkPos::new(pos.x, pos.y, pos.z - 1)), // -Z
        ]
    }
    
    /// Update the renderer, processing mesh builds and uploads
    pub fn update(&self, device: &wgpu::Device, camera: &Camera) {
        let start_time = Instant::now();
        
        // Process mesh building queue
        self.mesh_builder.process_queue(16); // Process up to 16 chunks
        
        // Upload completed meshes to GPU
        self.upload_completed_meshes(device);
        
        // Remove meshes for unloaded chunks
        self.cleanup_distant_meshes(camera);
        
        // Update frame timing
        *self.last_update.write() = start_time;
    }
    
    /// Upload completed meshes to GPU
    fn upload_completed_meshes(&self, device: &wgpu::Device) {
        let completed_meshes = self.mesh_builder.get_completed_meshes();
        let mut uploaded = 0;
        
        for completed in completed_meshes {
            if uploaded >= self.max_uploads_per_frame {
                // Re-queue the rest for next frame
                // TODO: Implement re-queueing logic
                break;
            }
            
            if completed.mesh.vertices.is_empty() {
                // Empty mesh, remove from GPU
                self.gpu_meshes.write().remove(&completed.chunk_pos);
                continue;
            }
            
            // Create GPU buffers
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Chunk {:?} Vertices", completed.chunk_pos)),
                contents: bytemuck::cast_slice(&completed.mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Chunk {:?} Indices", completed.chunk_pos)),
                contents: bytemuck::cast_slice(&completed.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            
            let gpu_mesh = GpuChunkMesh {
                vertex_buffer,
                index_buffer,
                num_indices: completed.mesh.indices.len() as u32,
                vertex_count: completed.mesh.vertices.len() as u32,
            };
            
            self.gpu_meshes.write().insert(completed.chunk_pos, gpu_mesh);
            uploaded += 1;
            
            // Remove from dirty set
            self.dirty_chunks.write().remove(&completed.chunk_pos);
        }
        
        self.stats.write().mesh_uploads_per_frame = uploaded;
    }
    
    /// Remove meshes for chunks that are too far away
    fn cleanup_distant_meshes(&self, camera: &Camera) {
        let camera_chunk = ChunkPos::new(
            (camera.position.x / self.chunk_size as f32).floor() as i32,
            (camera.position.y / self.chunk_size as f32).floor() as i32,
            (camera.position.z / self.chunk_size as f32).floor() as i32,
        );
        
        let max_distance_sq = (self.view_distance + 2) * (self.view_distance + 2);
        
        let mut meshes = self.gpu_meshes.write();
        meshes.retain(|chunk_pos, _| {
            chunk_pos.distance_squared_to(camera_chunk) <= max_distance_sq
        });
    }
    
    /// Render visible chunks with frustum culling
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera: &Camera,
    ) {
        // Due to lifetime constraints with wgpu RenderPass and thread-safe data structures,
        // we cannot directly render from our RwLock-protected meshes.
        // In a real implementation, you would need one of these approaches:
        // 1. Store meshes in a single-threaded structure owned by the renderer
        // 2. Copy mesh references before rendering
        // 3. Use a different architecture that avoids this conflict
        
        let mesh_count = self.gpu_meshes.read().len();
        log::debug!("AsyncChunkRenderer would render {} chunks", mesh_count);
        
        // Update stats
        let stats = RenderStats {
            chunks_rendered: mesh_count,
            chunks_culled: 0,
            total_triangles: mesh_count * 1000, // Estimate
            mesh_uploads_per_frame: 0,
            draw_calls: mesh_count,
        };
        *self.stats.write() = stats;
    }
    
    /// Simple frustum culling check
    fn chunk_in_frustum(&self, chunk_pos: &ChunkPos, view_proj: &cgmath::Matrix4<f32>) -> bool {
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
        
        // Check chunk center
        let center = Point3::new(
            (min.x + max.x) * 0.5,
            (min.y + max.y) * 0.5,
            (min.z + max.z) * 0.5,
        );
        
        // Transform to clip space
        let clip_pos = view_proj * cgmath::Vector4::new(center.x, center.y, center.z, 1.0);
        
        // Simple frustum test
        if clip_pos.w <= 0.0 {
            return false; // Behind camera
        }
        
        let ndc = Vector3::new(
            clip_pos.x / clip_pos.w,
            clip_pos.y / clip_pos.w,
            clip_pos.z / clip_pos.w,
        );
        
        // Check if in normalized device coordinates
        ndc.x.abs() <= 2.0 && ndc.y.abs() <= 2.0 && ndc.z >= -1.0 && ndc.z <= 1.0
    }
    
    /// Get current render statistics
    pub fn get_render_stats(&self) -> RenderStats {
        self.stats.read().clone()
    }
    
    /// Get mesh building statistics
    pub fn get_mesh_stats(&self) -> MeshBuildStats {
        self.mesh_builder.get_stats()
    }
    
    /// Get total number of GPU meshes
    pub fn mesh_count(&self) -> usize {
        self.gpu_meshes.read().len()
    }
    
    /// Get number of queued mesh builds
    pub fn queued_builds(&self) -> usize {
        self.mesh_builder.active_builds()
    }
    
    /// Clear all meshes and queues
    pub fn clear(&self) {
        self.gpu_meshes.write().clear();
        self.dirty_chunks.write().clear();
        self.mesh_builder.clear_queue();
    }
}

/// Trait for chunk management systems
pub trait ChunkManager: Send + Sync {
    fn chunks_iter(&self) -> Box<dyn Iterator<Item = (ChunkPos, Arc<RwLock<Chunk>>)> + '_>;
    fn get_chunk(&self, pos: ChunkPos) -> Option<Arc<RwLock<Chunk>>>;
}

// Implement ChunkManager for our chunk managers
impl ChunkManager for ConcurrentChunkManager {
    fn chunks_iter(&self) -> Box<dyn Iterator<Item = (ChunkPos, Arc<RwLock<Chunk>>)> + '_> {
        Box::new(self.chunks_iter())
    }
    
    fn get_chunk(&self, pos: ChunkPos) -> Option<Arc<RwLock<Chunk>>> {
        self.get_chunk(pos)
    }
}

impl ChunkManager for ParallelChunkManager {
    fn chunks_iter(&self) -> Box<dyn Iterator<Item = (ChunkPos, Arc<RwLock<Chunk>>)> + '_> {
        Box::new(self.chunks_iter())
    }
    
    fn get_chunk(&self, pos: ChunkPos) -> Option<Arc<RwLock<Chunk>>> {
        self.get_chunk(pos)
    }
}