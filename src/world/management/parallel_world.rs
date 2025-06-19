//! Parallel world implementation for unified architecture
//! 
//! This provides a parallel world manager that works with the unified GPU/CPU backend.

use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;
use crate::world::{
    core::{ChunkPos, VoxelPos, BlockId},
    interfaces::WorldInterface,
    generation::WorldGenerator,
    management::UnifiedWorldManager,
};

/// Configuration for parallel world
#[derive(Debug, Clone)]
pub struct ParallelWorldConfig {
    /// Number of threads for chunk generation
    pub generation_threads: usize,
    /// Number of threads for mesh building
    pub mesh_threads: usize,
    /// Maximum chunks to generate per frame
    pub chunks_per_frame: usize,
    /// View distance in chunks
    pub view_distance: i32,
    /// Chunk size
    pub chunk_size: u32,
    /// Enable GPU acceleration
    pub enable_gpu: bool,
}

impl Default for ParallelWorldConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            generation_threads: cpu_count.saturating_sub(2).max(2),
            mesh_threads: cpu_count.saturating_sub(2).max(2),
            chunks_per_frame: cpu_count * 2,
            view_distance: 8,
            chunk_size: 32,
            enable_gpu: true,
        }
    }
}

impl ParallelWorldConfig {
    pub fn new(chunk_size: u32) -> Self {
        Self {
            chunk_size,
            ..Default::default()
        }
    }
}

/// Parallel world manager using unified architecture
pub struct ParallelWorld {
    manager: Arc<RwLock<UnifiedWorldManager>>,
    config: ParallelWorldConfig,
    generator: Arc<dyn WorldGenerator + Send + Sync>,
}

impl ParallelWorld {
    /// Create a new parallel world
    pub async fn new(
        config: ParallelWorldConfig,
        generator: Box<dyn WorldGenerator + Send + Sync>,
        device: Option<Arc<wgpu::Device>>,
        queue: Option<Arc<wgpu::Queue>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let manager_config = crate::world::management::WorldManagerConfig {
            chunk_size: config.chunk_size,
            view_distance: config.view_distance as u32,
            ..Default::default()
        };
        
        let manager = if let (Some(device), Some(queue)) = (device, queue) {
            UnifiedWorldManager::new_gpu(device, queue, manager_config).await?
        } else {
            UnifiedWorldManager::new_cpu(manager_config)?
        };
        
        Ok(Self {
            manager: Arc::new(RwLock::new(manager)),
            config,
            generator: Arc::from(generator),
        })
    }
    
    /// Get world configuration
    pub fn config(&self) -> &ParallelWorldConfig {
        &self.config
    }
    
    /// Check if first chunks are loaded
    pub fn has_chunks_loaded(&self) -> bool {
        if let Some(manager) = self.manager.try_read() {
            manager.loaded_chunk_count() > 0
        } else {
            false
        }
    }
    
    /// Ensure camera chunk is loaded
    pub fn ensure_camera_chunk_loaded(&mut self, camera_pos: cgmath::Point3<f32>) -> bool {
        let chunk_pos = crate::ChunkPos {
            x: (camera_pos.x / self.config.chunk_size as f32).floor() as i32,
            y: (camera_pos.y / self.config.chunk_size as f32).floor() as i32,
            z: (camera_pos.z / self.config.chunk_size as f32).floor() as i32,
        };
        
        let mut manager = self.manager.write();
        if !manager.is_chunk_loaded(chunk_pos) {
            manager.load_chunk(chunk_pos).is_ok()
        } else {
            true
        }
    }
    
    /// Get block at position - available regardless of feature flags
    pub fn get_block(&self, pos: VoxelPos) -> BlockId {
        let manager = self.manager.read();
        manager.get_block(pos)
    }
    
    /// Update world state with camera position
    pub fn update(&mut self, camera_pos: cgmath::Point3<f32>) {
        // Update loaded chunks based on camera position
        self.update_loaded_chunks(camera_pos);
    }
    
    /// Update loaded chunks based on player position
    pub fn update_loaded_chunks(&mut self, player_pos: cgmath::Point3<f32>) {
        let chunk_pos = ChunkPos {
            x: (player_pos.x / self.config.chunk_size as f32).floor() as i32,
            y: (player_pos.y / self.config.chunk_size as f32).floor() as i32,
            z: (player_pos.z / self.config.chunk_size as f32).floor() as i32,
        };
        
        let mut manager = self.manager.write();
        let view_distance = self.config.view_distance;
        
        // Load chunks in view distance
        for x in -view_distance..=view_distance {
            for y in -2..=2 { // Limited vertical range
                for z in -view_distance..=view_distance {
                    let load_pos = ChunkPos {
                        x: chunk_pos.x + x,
                        y: chunk_pos.y + y,
                        z: chunk_pos.z + z,
                    };
                    
                    if !manager.is_chunk_loaded(load_pos) {
                        let _ = manager.load_chunk(load_pos);
                    }
                }
            }
        }
    }
    
    /// Get world buffer for GPU operations
    pub fn get_world_buffer(&self) -> Option<Arc<std::sync::Mutex<crate::world::storage::WorldBuffer>>> {
        None // TODO: Expose from UnifiedWorldManager if needed
    }
    
    /// Get chunk manager interface for compatibility
    pub fn chunk_manager(&self) -> ChunkManagerAdapter {
        ChunkManagerAdapter {
            manager: &self.manager,
        }
    }
}

/// Adapter to provide chunk manager interface
pub struct ChunkManagerAdapter<'a> {
    manager: &'a RwLock<UnifiedWorldManager>,
}

impl<'a> ChunkManagerAdapter<'a> {
    /// Get loaded chunk count
    pub fn loaded_chunk_count(&self) -> usize {
        if let Some(manager) = self.manager.try_read() {
            manager.loaded_chunk_count()
        } else {
            0
        }
    }
}

// Implement unified WorldInterface
impl crate::world::interfaces::WorldInterface for ParallelWorld {
    fn get_block(&self, pos: VoxelPos) -> BlockId {
        let manager = self.manager.read();
        manager.get_block(pos)
    }
    
    fn set_block(&mut self, pos: VoxelPos, block_id: BlockId) -> Result<(), crate::world::interfaces::WorldError> {
        let mut manager = self.manager.write();
        manager.set_block(pos, block_id)
            .map_err(|e| crate::world::interfaces::WorldError::OperationFailed { 
                operation: "set_block".to_string(),
                reason: e.to_string() 
            })
    }
    
    fn get_surface_height(&self, x: f64, z: f64) -> i32 {
        let manager = self.manager.read();
        manager.get_surface_height(x, z)
    }
    
    fn is_chunk_loaded(&self, chunk_pos: ChunkPos) -> bool {
        let manager = self.manager.read();
        manager.is_chunk_loaded(chunk_pos)
    }
    
    fn load_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), crate::world::interfaces::WorldError> {
        let mut manager = self.manager.write();
        manager.load_chunk(chunk_pos)
            .map_err(|e| crate::world::interfaces::WorldError::OperationFailed { 
                operation: "load_chunk".to_string(),
                reason: e.to_string() 
            })
    }
    
    fn unload_chunk(&mut self, chunk_pos: ChunkPos) -> Result<(), crate::world::interfaces::WorldError> {
        // TODO: Implement chunk unloading
        Ok(())
    }
    
    fn raycast(&self, ray: crate::Ray, max_distance: f32) -> Option<crate::RaycastHit> {
        // TODO: Implement raycast
        None
    }
    
    fn query(&self, query: crate::world::interfaces::WorldQuery) -> Result<crate::world::interfaces::QueryResult, crate::world::interfaces::WorldError> {
        Err(crate::world::interfaces::WorldError::OperationFailed {
            message: "Query operation not implemented".to_string()
        })
    }
    
    fn get_chunks_in_radius(&self, center: ChunkPos, radius: u32) -> Vec<ChunkPos> {
        // TODO: Implement
        Vec::new()
    }
    
    fn batch_operation(&mut self, operations: Vec<crate::world::interfaces::WorldOperation>) -> Result<Vec<crate::world::interfaces::OperationResult>, crate::world::interfaces::WorldError> {
        Err(crate::world::interfaces::WorldError::OperationFailed {
            message: "Query operation not implemented".to_string()
        })
    }
}

// Also implement UnifiedInterface (always available)
impl crate::world::interfaces::UnifiedInterface for ParallelWorld {
    fn backend_type(&self) -> &str {
        if self.config.enable_gpu {
            "GPU"
        } else {
            "CPU"
        }
    }
    
    fn supports_capability(&self, capability: &str) -> bool {
        use crate::world::interfaces::capabilities;
        match capability {
            capabilities::GPU_ACCELERATION => self.config.enable_gpu,
            capabilities::REAL_TIME_GENERATION => true,
            capabilities::BATCH_OPERATIONS => false,
            capabilities::INFINITE_WORLDS => true,
            capabilities::MULTI_THREADING => true,
            capabilities::LIGHTING_CALCULATION => self.config.enable_gpu,
            capabilities::PHYSICS_SIMULATION => self.config.enable_gpu,
            capabilities::WEATHER_EFFECTS => self.config.enable_gpu,
            capabilities::MEMORY_STREAMING => true,
            capabilities::LOD_SUPPORT => self.config.enable_gpu,
            _ => false,
        }
    }
    
    fn performance_metrics(&self) -> Option<std::collections::HashMap<String, f64>> {
        None
    }
}

/// Find a safe spawn location
pub struct SpawnFinder;

impl SpawnFinder {
    /// Find a safe spawn location
    pub fn find_safe_spawn(world: &ParallelWorld, x: f32, z: f32, search_radius: i32) -> Option<cgmath::Point3<f32>> {
        // Start from a reasonable height and search downward
        for y in (50..=100).rev() {
            let pos = crate::VoxelPos::new(x as i32, y, z as i32);
            let ground_pos = crate::VoxelPos::new(x as i32, y - 1, z as i32);
            let above_pos = crate::VoxelPos::new(x as i32, y + 1, z as i32);
            
            // Check if current position is air and ground below is solid
            if world.get_block(pos) == crate::BlockId::AIR && 
               world.get_block(above_pos) == crate::BlockId::AIR &&
               world.get_block(ground_pos) != crate::BlockId::AIR {
                return Some(cgmath::Point3::new(x, y as f32, z));
            }
        }
        None
    }
    
    /// Debug helper to print blocks at a position
    pub fn debug_blocks_at_position(world: &ParallelWorld, pos: cgmath::Point3<f32>) {
        log::info!("Debugging blocks around position ({}, {}, {})", pos.x, pos.y, pos.z);
        for dy in -2..=2 {
            let check_pos = crate::VoxelPos::new(pos.x as i32, pos.y as i32 + dy, pos.z as i32);
            let block_id = world.get_block(check_pos);
            log::info!("  Y={}: Block {:?}", pos.y as i32 + dy, block_id);
        }
    }

    /// Find spawn location near a given position
    pub fn find_spawn_near(world: &dyn crate::WorldInterface, position: cgmath::Point3<f32>, chunk_size: u32) -> Option<cgmath::Point3<f32>> {
        let start_y = position.y as i32;
        let search_radius = 16;
        
        // Search in expanding circles
        for radius in 0..search_radius {
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    // Only check perimeter
                    if (dx as i32).abs() != radius && (dz as i32).abs() != radius {
                        continue;
                    }
                    
                    let x = position.x as i32 + dx;
                    let z = position.z as i32 + dz;
                    
                    // Search vertically
                    for y in (start_y - 32)..=(start_y + 32) {
                        let pos = VoxelPos::new(x, y, z);
                        let above = VoxelPos::new(x, y + 1, z);
                        let above2 = VoxelPos::new(x, y + 2, z);
                        
                        // Check for solid ground with 2 blocks of air above
                        if world.get_block(pos) != BlockId::AIR &&
                           world.get_block(above) == BlockId::AIR &&
                           world.get_block(above2) == BlockId::AIR {
                            return Some(cgmath::Point3::new(
                                x as f32 + 0.5,
                                y as f32 + 1.0,
                                z as f32 + 0.5,
                            ));
                        }
                    }
                }
            }
        }
        
        None
    }
}