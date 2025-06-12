use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use crossbeam_channel::{unbounded, bounded, Sender, Receiver};
use rayon::{ThreadPool, ThreadPoolBuilder};
use dashmap::DashMap;
use crate::{
    ChunkPos, Chunk, BlockRegistry,
    renderer::{ChunkMesh, greedy_mesher::GreedyMesher},
};

/// Request to build a mesh for a chunk
pub struct MeshBuildRequest {
    pub chunk_pos: ChunkPos,
    pub chunk: Arc<RwLock<Chunk>>,
    pub priority: i32, // Lower values = higher priority
    pub neighbors: [Option<Arc<RwLock<Chunk>>>; 6], // +X, -X, +Y, -Y, +Z, -Z
}

/// Completed mesh ready for GPU upload
pub struct CompletedMesh {
    pub chunk_pos: ChunkPos,
    pub mesh: ChunkMesh,
    pub build_time: Duration,
    pub vertex_count: usize,
    pub face_count: usize,
}

/// Statistics for mesh building performance
#[derive(Debug, Clone, Default)]
pub struct MeshBuildStats {
    pub meshes_built: usize,
    pub total_build_time: Duration,
    pub average_build_time: Duration,
    pub meshes_per_second: f32,
    pub total_vertices: usize,
    pub total_faces: usize,
    pub queued_requests: usize,
}

/// Async mesh builder that processes mesh generation in background threads
pub struct AsyncMeshBuilder {
    /// Thread pool for mesh building
    mesh_pool: Arc<ThreadPool>,
    /// Send mesh build requests
    request_sender: Sender<MeshBuildRequest>,
    request_receiver: Receiver<MeshBuildRequest>,
    /// Receive completed meshes
    completed_sender: Sender<CompletedMesh>,
    completed_receiver: Receiver<CompletedMesh>,
    /// Block registry for mesh generation
    registry: Arc<BlockRegistry>,
    /// Chunk size
    chunk_size: u32,
    /// Statistics
    stats: Arc<RwLock<MeshBuildStats>>,
    /// Active mesh builds (for deduplication)
    active_builds: Arc<DashMap<ChunkPos, Instant>>,
    /// Priority queue for mesh requests
    priority_queue: Arc<RwLock<Vec<MeshBuildRequest>>>,
}

impl AsyncMeshBuilder {
    pub fn new(
        registry: Arc<BlockRegistry>, 
        chunk_size: u32,
        thread_count: Option<usize>,
    ) -> Self {
        let threads = thread_count.unwrap_or_else(|| {
            num_cpus::get().saturating_sub(2).max(2)
        });
        
        // Create thread pool for mesh building
        let mesh_pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|idx| format!("mesh-builder-{}", idx))
            .build()
            .expect("Failed to create mesh builder thread pool");
        
        // Channels for communication
        let (req_send, req_recv) = unbounded();
        let (comp_send, comp_recv) = bounded(threads * 4); // Limit completed queue
        
        Self {
            mesh_pool: Arc::new(mesh_pool),
            request_sender: req_send,
            request_receiver: req_recv,
            completed_sender: comp_send,
            completed_receiver: comp_recv,
            registry,
            chunk_size,
            stats: Arc::new(RwLock::new(MeshBuildStats::default())),
            active_builds: Arc::new(DashMap::new()),
            priority_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Queue a chunk for mesh building
    pub fn queue_chunk(
        &self,
        chunk_pos: ChunkPos,
        chunk: Arc<RwLock<Chunk>>,
        priority: i32,
        neighbors: [Option<Arc<RwLock<Chunk>>>; 6],
    ) {
        // Check if already building
        if self.active_builds.contains_key(&chunk_pos) {
            return;
        }
        
        // Mark as active
        self.active_builds.insert(chunk_pos, Instant::now());
        
        let request = MeshBuildRequest {
            chunk_pos,
            chunk,
            priority,
            neighbors,
        };
        
        // Add to priority queue
        let mut queue = self.priority_queue.write();
        queue.push(request);
        queue.sort_by_key(|r| r.priority); // Sort by priority (lower first)
        
        // Update stats
        self.stats.write().queued_requests = queue.len();
    }
    
    /// Process mesh building queue
    pub fn process_queue(&self, max_builds: usize) {
        let mut requests = Vec::new();
        
        // Get up to max_builds requests from priority queue
        {
            let mut queue = self.priority_queue.write();
            for _ in 0..max_builds.min(queue.len()) {
                if let Some(request) = queue.pop() {
                    requests.push(request);
                }
            }
            self.stats.write().queued_requests = queue.len();
        }
        
        // Process requests in parallel
        if !requests.is_empty() {
            let pool = Arc::clone(&self.mesh_pool);
            let registry = Arc::clone(&self.registry);
            let sender = self.completed_sender.clone();
            let stats = Arc::clone(&self.stats);
            let active = Arc::clone(&self.active_builds);
            let chunk_size = self.chunk_size;
            
            pool.spawn(move || {
                requests.into_par_iter().for_each(|request| {
                    let start_time = Instant::now();
                    
                    // Build the mesh
                    let mesh = {
                        let chunk = request.chunk.read();
                        let mut mesher = GreedyMesher::new(chunk_size);
                        mesher.build_chunk_mesh(
                            &chunk,
                            request.chunk_pos,
                            chunk_size,
                            &registry,
                            &request.neighbors,
                        )
                    };
                    
                    let build_time = start_time.elapsed();
                    let vertex_count = mesh.vertices.len();
                    let face_count = vertex_count / 4; // Quads
                    
                    // Send completed mesh
                    let completed = CompletedMesh {
                        chunk_pos: request.chunk_pos,
                        mesh,
                        build_time,
                        vertex_count,
                        face_count,
                    };
                    
                    let _ = sender.try_send(completed);
                    
                    // Update stats
                    {
                        let mut s = stats.write();
                        s.meshes_built += 1;
                        s.total_build_time += build_time;
                        s.average_build_time = s.total_build_time / s.meshes_built as u32;
                        s.total_vertices += vertex_count;
                        s.total_faces += face_count;
                        
                        let elapsed_secs = s.total_build_time.as_secs_f32();
                        if elapsed_secs > 0.0 {
                            s.meshes_per_second = s.meshes_built as f32 / elapsed_secs;
                        }
                    }
                    
                    // Remove from active builds
                    active.remove(&request.chunk_pos);
                });
            });
        }
    }
    
    /// Get completed meshes
    pub fn get_completed_meshes(&self) -> Vec<CompletedMesh> {
        let mut meshes = Vec::new();
        while let Ok(mesh) = self.completed_receiver.try_recv() {
            meshes.push(mesh);
        }
        meshes
    }
    
    /// Process a single mesh build request (for immediate builds)
    pub fn build_mesh_immediate(
        &self,
        chunk_pos: ChunkPos,
        chunk: &Chunk,
        neighbors: &[Option<Arc<RwLock<Chunk>>>; 6],
    ) -> ChunkMesh {
        let mut mesher = GreedyMesher::new(self.chunk_size);
        mesher.build_chunk_mesh(
            chunk,
            chunk_pos,
            self.chunk_size,
            &self.registry,
            neighbors,
        )
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> MeshBuildStats {
        self.stats.read().clone()
    }
    
    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.write() = MeshBuildStats::default();
    }
    
    /// Get number of active mesh builds
    pub fn active_builds(&self) -> usize {
        self.active_builds.len()
    }
    
    /// Clear the build queue
    pub fn clear_queue(&self) {
        self.priority_queue.write().clear();
        self.active_builds.clear();
        self.stats.write().queued_requests = 0;
    }
    
    /// Check if a chunk is queued or being built
    pub fn is_queued_or_building(&self, chunk_pos: &ChunkPos) -> bool {
        if self.active_builds.contains_key(chunk_pos) {
            return true;
        }
        
        let queue = self.priority_queue.read();
        queue.iter().any(|r| r.chunk_pos == *chunk_pos)
    }
}

use rayon::prelude::*;

/// Batch mesh builder for maximum throughput
pub struct BatchMeshBuilder {
    builder: AsyncMeshBuilder,
    batch_size: usize,
}

impl BatchMeshBuilder {
    pub fn new(builder: AsyncMeshBuilder, batch_size: usize) -> Self {
        Self { builder, batch_size }
    }
    
    /// Process multiple chunks in a single batch
    pub fn build_batch(&self, requests: Vec<MeshBuildRequest>) -> Vec<CompletedMesh> {
        requests
            .into_par_iter()
            .map(|request| {
                let start_time = Instant::now();
                
                let mesh = {
                    let chunk = request.chunk.read();
                    self.builder.build_mesh_immediate(
                        request.chunk_pos,
                        &chunk,
                        &request.neighbors,
                    )
                };
                
                let build_time = start_time.elapsed();
                
                CompletedMesh {
                    chunk_pos: request.chunk_pos,
                    mesh,
                    build_time,
                    vertex_count: 0, // Will be calculated from mesh
                    face_count: 0,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_async_mesh_building() {
        // Create test registry and builder
        let registry = Arc::new(BlockRegistry::new());
        let builder = AsyncMeshBuilder::new(registry.clone(), 32, Some(4));
        
        // Create test chunk
        let pos = ChunkPos::new(0, 0, 0);
        let chunk = Arc::new(RwLock::new(Chunk::new(pos, 32)));
        
        // Queue mesh build
        builder.queue_chunk(pos, chunk, 0, Default::default());
        
        // Process queue
        builder.process_queue(10);
        
        // Wait a bit for processing
        std::thread::sleep(Duration::from_millis(100));
        
        // Check for completed meshes
        let completed = builder.get_completed_meshes();
        assert!(!completed.is_empty());
        
        // Check stats
        let stats = builder.get_stats();
        assert_eq!(stats.meshes_built, 1);
    }
}