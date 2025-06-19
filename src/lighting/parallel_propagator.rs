#![allow(unused_variables, dead_code)]
use std::sync::Arc;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use parking_lot::{RwLock, Mutex};
use crossbeam_channel::{unbounded, Sender, Receiver};
use dashmap::DashMap;
use crate::{
    world::{ChunkPos, VoxelPos, BlockId},
    lighting::{LightLevel, LightType, MAX_LIGHT_LEVEL, LIGHT_FALLOFF},
    thread_pool::{ThreadPoolManager, PoolCategory},
};

/// Light update request
#[derive(Debug, Clone)]
pub struct LightUpdate {
    pub pos: VoxelPos,
    pub light_type: LightType,
    pub level: u8,
    pub is_removal: bool,
}

/// Chunk light data for parallel access
#[derive(Debug)]
pub struct ChunkLightData {
    pub chunk_pos: ChunkPos,
    pub light_data: Arc<RwLock<Vec<u8>>>, // Packed light data
    pub size: u32,
}

impl ChunkLightData {
    pub fn new(chunk_pos: ChunkPos, size: u32) -> Self {
        let total_size = (size * size * size) as usize;
        Self {
            chunk_pos,
            light_data: Arc::new(RwLock::new(vec![0; total_size])),
            size,
        }
    }
    
    fn index(&self, x: u32, y: u32, z: u32) -> usize {
        (y * self.size * self.size + z * self.size + x) as usize
    }
    
    pub fn get_light(&self, local_pos: VoxelPos) -> LightLevel {
        let x = local_pos.x as u32;
        let y = local_pos.y as u32;
        let z = local_pos.z as u32;
        
        if x >= self.size || y >= self.size || z >= self.size {
            return LightLevel::dark();
        }
        
        let data = self.light_data.read();
        let idx = self.index(x, y, z);
        let packed = data.get(idx).copied().unwrap_or(0);
        LightLevel {
            sky: (packed >> 4) & 0x0F,
            block: packed & 0x0F,
        }
    }
    
    pub fn set_light(&self, local_pos: VoxelPos, light_type: LightType, level: u8) {
        let x = local_pos.x as u32;
        let y = local_pos.y as u32;
        let z = local_pos.z as u32;
        
        if x >= self.size || y >= self.size || z >= self.size {
            return;
        }
        
        let idx = self.index(x, y, z);
        let mut data = self.light_data.write();
        
        if let Some(light_byte) = data.get_mut(idx) {
            match light_type {
                LightType::Sky => {
                    let block_light = *light_byte & 0x0F;
                    *light_byte = ((level.min(15) & 0x0F) << 4) | block_light;
                }
                LightType::Block => {
                    let sky_light = *light_byte & 0xF0;
                    *light_byte = sky_light | (level.min(15) & 0x0F);
                }
            }
        }
    }
}

/// Statistics for parallel light propagation
#[derive(Debug, Clone, Default)]
pub struct LightingStats {
    pub updates_processed: usize,
    pub chunks_affected: usize,
    pub total_propagation_time: Duration,
    pub updates_per_second: f32,
    pub cross_chunk_updates: usize,
}

/// Parallel light propagation data (DOP - no methods)
/// Pure data structure for light propagation state
pub struct ParallelLightPropagatorData {
    /// Light update queue
    pub update_sender: Sender<LightUpdate>,
    pub update_receiver: Receiver<LightUpdate>,
    /// Chunk light data cache
    pub chunk_lights: Arc<DashMap<ChunkPos, Arc<ChunkLightData>>>,
    /// Block data provider (thread-safe)
    pub block_provider: Arc<dyn BlockProvider>,
    /// Chunk size
    pub chunk_size: u32,
    /// Statistics
    pub stats: Arc<RwLock<LightingStats>>,
    /// Active light propagation jobs
    pub active_jobs: Arc<DashMap<ChunkPos, Arc<Mutex<ChunkLightJob>>>>,
}

/// Thread-safe block data provider trait
pub trait BlockProvider: Send + Sync {
    fn get_block(&self, pos: VoxelPos) -> BlockId;
    fn is_transparent(&self, pos: VoxelPos) -> bool;
}

/// Light propagation job for a chunk
pub struct ChunkLightJob {
    chunk_pos: ChunkPos,
    light_queue: VecDeque<(VoxelPos, LightType, u8)>,
    removal_queue: VecDeque<(VoxelPos, LightType, u8)>,
    boundary_updates: Vec<LightUpdate>, // Updates that affect neighboring chunks
}

/// Create new parallel light propagator data
/// Pure function - returns data structure, no behavior
pub fn create_parallel_light_propagator_data(
    block_provider: Arc<dyn BlockProvider>,
    chunk_size: u32,
    thread_count: Option<usize>,
) -> ParallelLightPropagatorData {
    let (update_send, update_recv) = unbounded();
    
    ParallelLightPropagatorData {
        update_sender: update_send,
        update_receiver: update_recv,
        chunk_lights: Arc::new(DashMap::new()),
        block_provider,
        chunk_size,
        stats: Arc::new(RwLock::new(LightingStats::default())),
        active_jobs: Arc::new(DashMap::new()),
    }
}

/// Queue a light update
/// Pure function - transforms update queue
pub fn queue_light_update(data: &ParallelLightPropagatorData, update: LightUpdate) {
    let _ = data.update_sender.send(update);
}

/// Add a light source
/// Pure function - transforms light data via queue
pub fn add_light_parallel(data: &ParallelLightPropagatorData, pos: VoxelPos, light_type: LightType, level: u8) {
    queue_light_update(data, LightUpdate {
        pos,
        light_type,
        level,
        is_removal: false,
    });
}

/// Remove a light source
/// Function - transforms light data via queue
pub fn remove_light_parallel(data: &ParallelLightPropagatorData, pos: VoxelPos, light_type: LightType) {
    // Get current light level for removal
    let chunk_pos = world_to_chunk_pos_static(pos, data.chunk_size);
    let chunk_light = get_or_create_chunk_light(data, chunk_pos);
    let local_pos = world_to_local_pos_static(pos, data.chunk_size);
    let current = chunk_light.get_light(local_pos);
    let level = match light_type {
        LightType::Sky => current.sky,
        LightType::Block => current.block,
    };
    
    queue_light_update(data, LightUpdate {
        pos,
        light_type,
        level,
        is_removal: true,
    });
}

/// Process light updates in parallel
/// Function - transforms light data by processing queued updates
pub fn process_light_updates_parallel(data: &ParallelLightPropagatorData, max_updates: usize) {
        let start_time = Instant::now();
        let mut updates_by_chunk: std::collections::HashMap<ChunkPos, Vec<LightUpdate>> = 
            std::collections::HashMap::new();
        
        // Collect updates and group by chunk
        let mut count = 0;
        while count < max_updates {
            match data.update_receiver.try_recv() {
                Ok(update) => {
                    let chunk_pos = world_to_chunk_pos_static(update.pos, data.chunk_size);
                    updates_by_chunk.entry(chunk_pos)
                        .or_insert_with(Vec::new)
                        .push(update);
                    count += 1;
                }
                Err(_) => break,
            }
        }
        
        if updates_by_chunk.is_empty() {
            return;
        }
        
        let chunks_affected = updates_by_chunk.len();
        let total_updates = count;
        
        // Process each chunk's updates in parallel
        let chunk_lights = Arc::clone(&data.chunk_lights);
        let block_provider = Arc::clone(&data.block_provider);
        let active_jobs = Arc::clone(&data.active_jobs);
        let chunk_size = data.chunk_size;
        let stats = Arc::clone(&data.stats);
        
        ThreadPoolManager::global().execute(PoolCategory::Lighting, || {
            
            rayon::scope(|s| {
            for (chunk_pos, updates) in updates_by_chunk {
                let chunk_lights = Arc::clone(&chunk_lights);
                let block_provider = Arc::clone(&block_provider);
                let active_jobs = Arc::clone(&active_jobs);
                
                s.spawn(move |_| {
                    // Get or create job for this chunk
                    let job = active_jobs.entry(chunk_pos)
                        .or_insert_with(|| Arc::new(Mutex::new(ChunkLightJob {
                            chunk_pos,
                            light_queue: VecDeque::new(),
                            removal_queue: VecDeque::new(),
                            boundary_updates: Vec::new(),
                        })))
                        .clone();
                    
                    let mut job = job.lock();
                    
                    // Add updates to appropriate queues
                    for update in updates {
                        if update.is_removal {
                            job.removal_queue.push_back((update.pos, update.light_type, update.level));
                        } else {
                            job.light_queue.push_back((update.pos, update.light_type, update.level));
                        }
                    }
                    
                    // Process the job
                    process_chunk_job(
                        &mut job,
                        chunk_pos,
                        &chunk_lights,
                        &block_provider,
                        chunk_size,
                    );
                });
            }
            });
        });
        
        // Collect boundary updates from all jobs and requeue them
        let mut cross_chunk_updates = 0;
        for entry in active_jobs.iter() {
            let mut job = entry.value().lock();
            for update in job.boundary_updates.drain(..) {
                queue_light_update(data, update);
                cross_chunk_updates += 1;
            }
        }
        
        // Update statistics
        let elapsed = start_time.elapsed();
        let mut stats = stats.write();
        stats.updates_processed += total_updates;
        stats.chunks_affected += chunks_affected;
        stats.total_propagation_time += elapsed;
        stats.cross_chunk_updates += cross_chunk_updates;
        
        let total_secs = stats.total_propagation_time.as_secs_f32();
        if total_secs > 0.0 {
            stats.updates_per_second = stats.updates_processed as f32 / total_secs;
        }
    }

/// Process a single chunk's light job
/// Pure function - transforms job data based on light propagation rules
fn process_chunk_job(
        job: &mut ChunkLightJob,
        chunk_pos: ChunkPos,
        chunk_lights: &DashMap<ChunkPos, Arc<ChunkLightData>>,
        block_provider: &Arc<dyn BlockProvider>,
        chunk_size: u32,
    ) {
        // Get chunk light data
        let chunk_light = chunk_lights.get(&chunk_pos)
            .map(|entry| Arc::clone(&entry))
            .unwrap_or_else(|| {
                let new_chunk = Arc::new(ChunkLightData::new(chunk_pos, chunk_size));
                chunk_lights.insert(chunk_pos, Arc::clone(&new_chunk));
                new_chunk
            });
        
        // Process removals first
        while let Some((pos, light_type, old_level)) = job.removal_queue.pop_front() {
            remove_light_recursive(
                job,
                pos,
                light_type,
                old_level,
                &chunk_light,
                chunk_lights,
                block_provider,
                chunk_size,
            );
        }
        
        // Process additions
        while let Some((pos, light_type, level)) = job.light_queue.pop_front() {
            propagate_light_recursive(
                job,
                pos,
                light_type,
                level,
                &chunk_light,
                chunk_lights,
                block_provider,
                chunk_size,
            );
        }
    }

/// Propagate light recursively within chunk
/// Pure function - transforms light data by recursive propagation
fn propagate_light_recursive(
        job: &mut ChunkLightJob,
        pos: VoxelPos,
        light_type: LightType,
        level: u8,
        chunk_light: &Arc<ChunkLightData>,
        chunk_lights: &DashMap<ChunkPos, Arc<ChunkLightData>>,
        block_provider: &Arc<dyn BlockProvider>,
        chunk_size: u32,
    ) {
        // Skip if position is solid
        if block_provider.get_block(pos) != BlockId::AIR && !block_provider.is_transparent(pos) {
            return;
        }
        
        // Convert to chunk-local coordinates
        let local_pos = world_to_local_pos_static(pos, chunk_size);
        let current_chunk_pos = world_to_chunk_pos_static(pos, chunk_size);
        
        // Handle cross-chunk propagation
        if current_chunk_pos != job.chunk_pos {
            job.boundary_updates.push(LightUpdate {
                pos,
                light_type,
                level,
                is_removal: false,
            });
            return;
        }
        
        // Get current light level
        let current = chunk_light.get_light(local_pos);
        let current_level = match light_type {
            LightType::Sky => current.sky,
            LightType::Block => current.block,
        };
        
        // Only update if new level is higher
        if level <= current_level {
            return;
        }
        
        // Set the new light level
        chunk_light.set_light(local_pos, light_type, level);
        
        // Propagate to neighbors
        if level > LIGHT_FALLOFF {
            let next_level = level - LIGHT_FALLOFF;
            
            let neighbors = [
                VoxelPos::new(pos.x + 1, pos.y, pos.z),
                VoxelPos::new(pos.x - 1, pos.y, pos.z),
                VoxelPos::new(pos.x, pos.y + 1, pos.z),
                VoxelPos::new(pos.x, pos.y - 1, pos.z),
                VoxelPos::new(pos.x, pos.y, pos.z + 1),
                VoxelPos::new(pos.x, pos.y, pos.z - 1),
            ];
            
            for neighbor in neighbors {
                // Special handling for skylight
                if light_type == LightType::Sky && neighbor.y < pos.y && level == MAX_LIGHT_LEVEL {
                    job.light_queue.push_back((neighbor, light_type, MAX_LIGHT_LEVEL));
                } else {
                    job.light_queue.push_back((neighbor, light_type, next_level));
                }
            }
        }
    }

/// Remove light recursively within chunk
/// Pure function - transforms light data by recursive removal
fn remove_light_recursive(
        job: &mut ChunkLightJob,
        pos: VoxelPos,
        light_type: LightType,
        old_level: u8,
        chunk_light: &Arc<ChunkLightData>,
        chunk_lights: &DashMap<ChunkPos, Arc<ChunkLightData>>,
        block_provider: &Arc<dyn BlockProvider>,
        chunk_size: u32,
    ) {
        let local_pos = world_to_local_pos_static(pos, chunk_size);
        let current_chunk_pos = world_to_chunk_pos_static(pos, chunk_size);
        
        // Handle cross-chunk removal
        if current_chunk_pos != job.chunk_pos {
            job.boundary_updates.push(LightUpdate {
                pos,
                light_type,
                level: old_level,
                is_removal: true,
            });
            return;
        }
        
        // Get current light level
        let current = chunk_light.get_light(local_pos);
        let current_level = match light_type {
            LightType::Sky => current.sky,
            LightType::Block => current.block,
        };
        
        // If light level has changed, skip
        if current_level != old_level {
            return;
        }
        
        // Clear the light
        chunk_light.set_light(local_pos, light_type, 0);
        
        // Check neighbors
        let neighbors = [
            VoxelPos::new(pos.x + 1, pos.y, pos.z),
            VoxelPos::new(pos.x - 1, pos.y, pos.z),
            VoxelPos::new(pos.x, pos.y + 1, pos.z),
            VoxelPos::new(pos.x, pos.y - 1, pos.z),
            VoxelPos::new(pos.x, pos.y, pos.z + 1),
            VoxelPos::new(pos.x, pos.y, pos.z - 1),
        ];
        
        for neighbor in neighbors {
            let neighbor_chunk_pos = world_to_chunk_pos_static(neighbor, chunk_size);
            let neighbor_local = world_to_local_pos_static(neighbor, chunk_size);
            
            // Get neighbor light level
            let neighbor_level = if neighbor_chunk_pos == job.chunk_pos {
                let light = chunk_light.get_light(neighbor_local);
                match light_type {
                    LightType::Sky => light.sky,
                    LightType::Block => light.block,
                }
            } else if let Some(neighbor_chunk) = chunk_lights.get(&neighbor_chunk_pos) {
                let light = neighbor_chunk.get_light(neighbor_local);
                match light_type {
                    LightType::Sky => light.sky,
                    LightType::Block => light.block,
                }
            } else {
                0
            };
            
            if neighbor_level > 0 && neighbor_level < old_level {
                // This neighbor was lit by us, remove it
                job.removal_queue.push_back((neighbor, light_type, neighbor_level));
            } else if neighbor_level >= old_level {
                // This neighbor has its own light source, re-propagate
                job.light_queue.push_back((neighbor, light_type, neighbor_level));
            }
        }
    }

/// Calculate initial skylight for a chunk in parallel
/// Function - transforms chunk light data by calculating skylight
pub fn calculate_chunk_skylight_parallel(data: &ParallelLightPropagatorData, chunk_pos: ChunkPos) {
    let chunk_light = get_or_create_chunk_light(data, chunk_pos);
    let chunk_size = data.chunk_size;
    let block_provider = Arc::clone(&data.block_provider);
        
        // Process columns in parallel
        (0..chunk_size).into_par_iter().for_each(|x| {
            for z in 0..chunk_size {
                let mut light_level = MAX_LIGHT_LEVEL;
                
                // Process from top to bottom
                for y in (0..chunk_size).rev() {
                    let world_pos = VoxelPos::new(
                        chunk_pos.x * chunk_size as i32 + x as i32,
                        chunk_pos.y * chunk_size as i32 + y as i32,
                        chunk_pos.z * chunk_size as i32 + z as i32,
                    );
                    
                    // Check if this block blocks light
                    if block_provider.get_block(world_pos) != BlockId::AIR 
                        && !block_provider.is_transparent(world_pos) {
                        light_level = 0;
                    }
                    
                    // Set skylight level
                    let local_pos = VoxelPos::new(x as i32, y as i32, z as i32);
                    chunk_light.set_light(local_pos, LightType::Sky, light_level);
                }
            }
        });
    }

/// Get or create chunk light data
/// Function - transforms chunk light cache
fn get_or_create_chunk_light(data: &ParallelLightPropagatorData, chunk_pos: ChunkPos) -> Arc<ChunkLightData> {
    data.chunk_lights
        .entry(chunk_pos)
        .or_insert_with(|| Arc::new(ChunkLightData::new(chunk_pos, data.chunk_size)))
        .clone()
}

/// Convert world position to chunk position
/// Pure function - coordinate transformation
fn world_to_chunk_pos_static(pos: VoxelPos, chunk_size: u32) -> ChunkPos {
    ChunkPos::new(
        pos.x.div_euclid(chunk_size as i32),
        pos.y.div_euclid(chunk_size as i32),
        pos.z.div_euclid(chunk_size as i32),
    )
}

/// Convert world position to local chunk position
/// Pure function - coordinate transformation
fn world_to_local_pos_static(pos: VoxelPos, chunk_size: u32) -> VoxelPos {
    VoxelPos::new(
        pos.x.rem_euclid(chunk_size as i32),
        pos.y.rem_euclid(chunk_size as i32),
        pos.z.rem_euclid(chunk_size as i32),
    )
}

/// Get lighting statistics
/// Pure function - reads statistics data
pub fn get_lighting_stats(data: &ParallelLightPropagatorData) -> LightingStats {
    data.stats.read().clone()
}

/// Reset statistics
/// Function - transforms statistics data
pub fn reset_lighting_stats(data: &ParallelLightPropagatorData) {
    *data.stats.write() = LightingStats::default();
}

/// Get chunk light data for a specific chunk
/// Pure function - reads chunk light data
pub fn get_chunk_light_data(data: &ParallelLightPropagatorData, chunk_pos: ChunkPos) -> Option<Arc<ChunkLightData>> {
    data.chunk_lights.get(&chunk_pos).map(|entry| Arc::clone(&entry))
}

/// Clear all chunk light data
/// Function - transforms chunk light cache
pub fn clear_all_chunk_lights(data: &ParallelLightPropagatorData) {
    data.chunk_lights.clear();
    data.active_jobs.clear();
}

use rayon::prelude::*;

/// Batch light calculator data (DOP - no methods)
/// Pure data structure for batch operations
pub struct BatchLightCalculatorData {
    pub propagator: Arc<ParallelLightPropagatorData>,
}

/// Create batch light calculator data
/// Pure function - returns data structure
pub fn create_batch_light_calculator_data(propagator: Arc<ParallelLightPropagatorData>) -> BatchLightCalculatorData {
    BatchLightCalculatorData { propagator }
}

/// Calculate skylight for multiple chunks in parallel
/// Function - transforms light data for multiple chunks
pub fn calculate_skylight_batch(data: &BatchLightCalculatorData, chunk_positions: Vec<ChunkPos>) {
    chunk_positions.par_iter().for_each(|&chunk_pos| {
        calculate_chunk_skylight_parallel(&data.propagator, chunk_pos);
    });
}

/// Process a batch of light updates
/// Function - transforms light data for multiple updates
pub fn process_light_batch(data: &BatchLightCalculatorData, updates: Vec<LightUpdate>) {
    for update in updates {
        queue_light_update(&data.propagator, update);
    }
    process_light_updates_parallel(&data.propagator, usize::MAX);
}

